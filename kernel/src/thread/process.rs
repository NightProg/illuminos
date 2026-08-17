use crate::{
    allocator::{
        mmap::{self, MMAP_BASE},
        paging::KERNEL_PAGING_MANAGER,
        process_paging::ProcessPageTable,
        vma::{self, MapFlags, ProtFlags, VMA},
    },
    context::GLOBAL_CONTEXT,
    dbg,
    elf::ElfProcess,
    fs::{FdTable, Inode, OpenFile, OpenFlags},
    gdt::KERNEL_STACK_SIZE,
    io::pipe::Pipe,
    println, println_serial,
    sync::mutex::TimeoutMutex,
    syscall::SyscallCtx,
    thread::{Task, iretq_trampoline, yield_now},
    tty::session::SessionId,
};
use alloc::{
    boxed::Box,
    collections::btree_map::BTreeMap,
    format,
    string::{String, ToString},
    sync::Arc,
    vec::{self, Vec},
};
use core::{hint::unreachable_unchecked, sync::atomic::AtomicU64};
use illfs::InOutDevice;
use x86_64::{
    PhysAddr, VirtAddr, align_up,
    registers::control::{Cr3, Cr3Flags},
    structures::paging::{
        FrameAllocator, FrameDeallocator, Mapper, OffsetPageTable, PageTable, PageTableFlags,
        PhysFrame, Size4KiB, Translate,
    },
};

use spin::Mutex;

use crate::thread::{SCHEDULER, Scheduler};

pub static PID: AtomicU64 = AtomicU64::new(1);

pub static PROCESSES: TimeoutMutex<BTreeMap<Pid, Process>> = TimeoutMutex::new(BTreeMap::new());

pub const USER_STACK_TOP: u64 = 0x0000_7FFF_FF00_0000;
pub const USER_STACK_SIZE: u64 = 0x8000;

#[derive(Debug, PartialEq, Clone)]
pub enum ProcessState {
    Ready,
    Running,
    Waiting,
    Terminated,
}

#[derive(Debug, Clone)]
pub struct ProcessArguments {
    pub argv: Vec<String>,
    pub envp: Vec<String>,
}

pub type Pid = u64;

#[derive(Clone)]
pub struct Process {
    pub pid: Pid,
    pub brk: u64,
    pub current_session: Option<SessionId>,
    pub brk_start: u64,
    pub pml4_table: ProcessPageTable,
    pub state: ProcessState,
    pub tasks: Vec<usize>,
    pub open_files: Arc<FdTable>,
    pub vmas: Vec<VMA>,
    pub arguments: Option<ProcessArguments>,
    pub exit_code: Option<u8>,
    pub exit_callbacks: Vec<Arc<dyn Fn() + Send + Sync>>,
}

impl core::fmt::Debug for Process {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Process")
            .field("pid", &self.pid)
            .field("state", &self.state)
            .field("tasks", &self.tasks)
            .field("open_files_count", &self.open_files.len())
            .field("vmas", &self.vmas)
            .field("current_session", &self.current_session)
            .finish()
    }
}

impl Process {
    pub fn create(
        pml4_table: ProcessPageTable,
        vmas: &[VMA],
        process_argument: Option<ProcessArguments>,
        session: Option<SessionId>,
        brk_start: u64,
    ) -> Pid {
        let mut processes_lock = PROCESSES.lock();
        let process = Process::new(pml4_table, vmas, process_argument, session, brk_start);
        let pid = process.pid;
        processes_lock.insert(pid, process);
        pid
    }

    pub fn kernel() -> Self {
        Process {
            pid: PID.fetch_add(1, core::sync::atomic::Ordering::SeqCst),
            pml4_table: ProcessPageTable::kernel(),
            state: ProcessState::Ready,
            tasks: Vec::new(),
            open_files: Arc::new(FdTable::new_std()),
            vmas: Vec::new(),
            arguments: None,
            exit_code: None,
            exit_callbacks: Vec::new(),
            brk: 0,
            brk_start: 0,
            current_session: None,
        }
    }

    pub fn new(
        pml4_table: ProcessPageTable,
        vmas: &[VMA],
        process_argument: Option<ProcessArguments>,
        session: Option<SessionId>,
        brk_start: u64,
    ) -> Self {
        Process {
            pid: PID.fetch_add(1, core::sync::atomic::Ordering::SeqCst),
            pml4_table,
            state: ProcessState::Ready,
            tasks: Vec::new(),
            open_files: Arc::new(FdTable::new_std()),
            vmas: vmas.to_vec(),
            arguments: process_argument,
            exit_code: None,
            exit_callbacks: Vec::new(),
            brk: brk_start,
            brk_start,
            current_session: session,
        }
    }

    pub fn add_to_processes(&self) {
        let mut processes_lock = PROCESSES.lock();
        processes_lock.insert(self.pid, self.clone());
    }

    pub fn add_exit_callback<F: Fn() + Send + Sync + 'static>(&mut self, callback: F) {
        self.exit_callbacks.push(Arc::new(callback));
    }

    pub fn spawn_user_thread_with_stack(
        &mut self,
        entry_point: VirtAddr,
        stack_base: VirtAddr,
    ) -> usize {
        let stack_top = stack_base + USER_STACK_SIZE;
        self.pml4_table.map_range(
            stack_base,
            USER_STACK_SIZE,
            PageTableFlags::PRESENT
                | PageTableFlags::WRITABLE
                | PageTableFlags::USER_ACCESSIBLE
                | PageTableFlags::NO_EXECUTE,
            &mut KERNEL_PAGING_MANAGER
                .lock()
                .as_mut()
                .expect("KERNEL_PAGING_MANAGER not initialized before spawn_thread_with_stack")
                .frame_allocator,
        );

        self.add_to_scheduler(entry_point, stack_top)
    }

    pub fn spawn_user_thread(&mut self, entry_point: VirtAddr) -> usize {
        let stack_base = VirtAddr::new(USER_STACK_TOP - self.tasks.len() as u64 * USER_STACK_SIZE);
        self.pml4_table.map_range(
            stack_base,
            USER_STACK_SIZE,
            PageTableFlags::PRESENT
                | PageTableFlags::WRITABLE
                | PageTableFlags::USER_ACCESSIBLE
                | PageTableFlags::NO_EXECUTE,
            &mut KERNEL_PAGING_MANAGER
                .lock()
                .as_mut()
                .expect("KERNEL_PAGING_MANAGER not initialized before spawn_thread")
                .frame_allocator,
        );

        let stack_top = stack_base + USER_STACK_SIZE;

        self.vmas.push(VMA {
            start: stack_base.as_u64(),
            end: stack_top.as_u64(),
            flags: MapFlags::PRIVATE | MapFlags::ANONYMOUS,
            prot: ProtFlags::READ | ProtFlags::WRITE,
            file: None,
            offset: 0,
            phys_addr: None,
        });

        self.add_to_scheduler(entry_point, stack_top)
    }

    pub fn add_to_scheduler(&mut self, rip: VirtAddr, rsp: VirtAddr) -> usize {
        let task_id = SCHEDULER.lock().add_user_task(
            rip.as_u64(),
            rsp.as_u64(),
            self.pml4_table.pml4_frame.start_address().as_u64(),
            KERNEL_STACK_SIZE,
            self.pid,
            self.arguments.clone(),
        );

        self.tasks.push(task_id);
        task_id
    }

    pub fn add_task(&mut self, task: Task) -> usize {
        let task_id = SCHEDULER.lock().add_task(task);
        self.tasks.push(task_id);
        task_id
    }

    pub fn is_terminated_in_scheduler(&self) -> bool {
        let scheduler = SCHEDULER.lock();
        self.tasks.iter().all(|&task_id| {
            if let Some(task) = scheduler.tasks.get(&task_id) {
                matches!(task.state, crate::thread::TaskState::Zombie(_))
            } else {
                true
            }
        })
    }

    pub fn open_file_path(&mut self, path: &str, flags: OpenFlags) -> Result<usize, String> {
        let file = GLOBAL_CONTEXT.fs.lock().open_file(path, flags)?;
        Ok(self.open_files.open(Arc::new(Mutex::new(file))))
    }

    pub fn open_inode<I: Inode + 'static>(
        &mut self,
        inode: I,
        flags: OpenFlags,
    ) -> Result<usize, String> {
        let file = OpenFile::new(Arc::new(spin::Mutex::new(inode)), flags);

        Ok(self.open_files.open(Arc::new(Mutex::new(file))))
    }

    pub fn read_file(&self, fd: usize, buf: &mut [u8]) -> Result<usize, String> {
        if let Some(mut open_file) = self.open_files.get(fd) {
            let mut open_file_lock = open_file.lock();
            open_file_lock.read(buf)
        } else {
            Err(format!("Invalid file descriptor: {}", fd))
        }
    }

    pub fn write_file(&self, fd: usize, buf: &[u8]) -> Result<usize, String> {
        if let Some(mut open_file) = self.open_files.get(fd) {
            let mut open_file_lock = open_file.lock();
            open_file_lock.write(buf)
        } else {
            Err(format!("Invalid file descriptor: {}", fd))
        }
    }

    pub fn close_file(&mut self, fd: usize) -> Result<(), String> {
        if self.open_files.close(fd).is_some() {
            Ok(())
        } else {
            Err(format!("Invalid file descriptor: {}", fd))
        }
    }

    pub fn ioctl(&self, fd: usize, request: u64, arg: u64) -> Result<u64, String> {
        if let Some(mut open_file) = self.open_files.get(fd) {
            let mut open_file_lock = open_file.lock();
            open_file_lock.inode.lock().ioctl(request, arg)
        } else {
            Err(format!("Invalid file descriptor: {}", fd))
        }
    }

    pub fn get_file(&self, fd: u64) -> Option<Arc<Mutex<dyn Inode>>> {
        self.open_files
            .get(fd as usize)
            .map(|open_file| open_file.lock().inode.clone())
    }

    pub fn dealloc(&mut self) {
        let phys_offset = self.pml4_table.phys_offset.as_u64();
        for vma in self.vmas.iter() {
            for va in (vma.start..vma.end).step_by(4096) {
                unsafe {
                    let mut kernel_paging_manager_lock = KERNEL_PAGING_MANAGER.lock();
                    let frame_allocator =
                        &mut kernel_paging_manager_lock.as_mut().unwrap().frame_allocator;
                    self.pml4_table.with_mapper(|mapper| {
                        let exist = mapper.translate_addr(VirtAddr::new(va)).is_some();
                        if exist {
                            let frame = PhysFrame::containing_address(PhysAddr::new(va));
                            frame_allocator.deallocate_frame(frame);
                        }
                    });
                }
            }
        }

        self.pml4_table.dealloc();
    }

    pub fn find_free_vma_space(&self, len: u64, base: u64) -> Option<VirtAddr> {
        const PAGE_SIZE: u64 = 4096;

        let mut candidate = align_up(base, PAGE_SIZE);

        let mut vmas = self.vmas.clone();
        vmas.sort_by_key(|v| v.start);

        for vma in vmas {
            let start = vma.start as u64;
            let end = vma.end as u64;

            if let Some(end_candidate) = candidate.checked_add(len) {
                if end_candidate <= start {
                    return Some(VirtAddr::new(candidate));
                }
            } else {
                return None;
            }

            candidate = align_up(end, PAGE_SIZE);
        }

        candidate.checked_add(len)?;
        Some(VirtAddr::new(candidate))
    }
}

pub fn fork(task_id: usize, sysctx: *mut SyscallCtx) -> Option<Pid> {
    let task = SCHEDULER.lock().tasks.get(&task_id)?.clone();
    let sysctx = unsafe { sysctx.as_ref().unwrap() };

    let pid = task.parent;

    let process = PROCESSES.lock().get(&pid).cloned().unwrap();

    let new_pid = Process::create(
        ProcessPageTable::copy_from(process.pml4_table),
        &process.vmas,
        None,
        process.current_session,
        process.brk_start,
    );
    let mut processes_lock = PROCESSES.lock();
    let new_process = processes_lock.get_mut(&new_pid).unwrap();
    new_process.open_files = process.open_files.clone();
    new_process.brk = process.brk;
    let mut new_task = Task::new_user(
        0,
        new_pid,
        sysctx.rip,
        sysctx.rsp,
        new_process.pml4_table.pml4_frame.start_address().as_u64(),
        KERNEL_STACK_SIZE,
        None,
    );

    new_task.context.r15 = sysctx.r15;
    new_task.context.r14 = sysctx.r14;
    new_task.context.r13 = sysctx.r13;
    new_task.context.r12 = sysctx.r12;
    new_task.context.r10 = sysctx.r10;
    new_task.context.r9 = 0;
    new_task.context.r8 = sysctx.r8;
    new_task.context.rbx = sysctx.rbx;
    new_task.context.rbp = sysctx.rbp;
    new_task.context.rax = 0;

    println!("NEW TASK FORKED {:?}", new_task);

    SCHEDULER.lock().add_task(new_task);
    Some(new_pid)
}

pub fn execve(path: &str, process_argument: ProcessArguments) -> Result<(), String> {
    let path = GLOBAL_CONTEXT
        .fs
        .lock()
        .lookup(path)
        .map_err(|e| format!("execve: failed to lookup {}: {}", path, e))?;

    let size = path.lock().size() as usize;
    let mut buf = alloc::vec![0u8; size];
    path.lock()
        .read_at(0, &mut buf)
        .map_err(|e| format!("execve: failed to read file {}: {}", size, e))?;
    let mut elf = ElfProcess::new(&buf);
    let entry_point = elf
        .load()
        .ok_or_else(|| "execve: failed to load ELF".to_string())?;
    let vmas = elf.get_vmas();
    let pml4_table = elf.get_page_table();
    let mut scheduler_lock = SCHEDULER.lock();
    let mut processes_lock = PROCESSES.lock();
    let current_task = scheduler_lock
        .current_task()
        .ok_or_else(|| "execve: no current task".to_string())?;
    let pid = current_task.parent;
    let process = processes_lock
        .get_mut(&pid)
        .ok_or_else(|| "execve: current task's parent process not found".to_string())?;

    for taskid in process.tasks.iter() {
        scheduler_lock.exit_task(*taskid, 137); // 137 = SIGKILL
    }
    drop(scheduler_lock);
    process.dealloc();

    process.arguments = Some(process_argument);
    process.pml4_table = *pml4_table;
    process.vmas = vmas.clone();
    process.brk_start = elf.get_brk_start();
    process.brk = process.brk_start;
    process.tasks.clear();

    process.spawn_user_thread(entry_point);
    Ok(())
}

pub fn wait(pid: Pid) {
    loop {
        let mut processes_lock = PROCESSES.lock();
        if let Some(process) = processes_lock.get(&pid) {
            if process.is_terminated_in_scheduler() {
                break;
            }
        } else {
            break;
        }
        drop(processes_lock);
        yield_now();
    }
}

pub fn wait_all() {
    loop {
        let processes_lock = PROCESSES.lock();
        if processes_lock
            .iter()
            .all(|p| p.1.is_terminated_in_scheduler())
        {
            break;
        }
        drop(processes_lock);
        yield_now();
    }
}

pub fn pipe() -> Result<(usize, usize), String> {
    let (reader, writer) = Pipe::new();
    let mut scheduler_lock = SCHEDULER.lock();
    let current_task = scheduler_lock
        .current_task()
        .expect("No current task in pipe");
    let mut processes_lock = PROCESSES.lock();
    let process = processes_lock
        .get_mut(&current_task.parent)
        .expect("Current task's parent process not found in pipe");

    let reader_fd = process.open_inode(reader, OpenFlags::READ)?;
    let writer_fd = process.open_inode(writer, OpenFlags::WRITE)?;

    Ok((reader_fd, writer_fd))
}

pub fn current_process() -> Option<Process> {
    let scheduler = SCHEDULER.lock();
    let current_task = scheduler.current_task()?;
    let processes_lock = PROCESSES.lock();
    processes_lock.get(&current_task.parent).cloned()
}

pub fn spawn(pid: Pid, rip: VirtAddr) -> Option<usize> {
    let mut processus_lock = PROCESSES.lock();
    let process = processus_lock.get_mut(&pid)?;
    Some(process.spawn_user_thread(rip))
}

pub fn get_exit_code(pid: Pid) -> Option<u8> {
    let processes_lock = PROCESSES.lock();
    processes_lock.get(&pid)?.exit_code
}

pub fn brk(process: &mut Process, addr: u64) -> Result<u64, String> {
    if addr == 0 {
        return Ok(process.brk);
    }

    if addr < process.brk_start {
        return Ok(process.brk);
    }

    if addr > process.brk {
        process.vmas.push(VMA {
            start: process.brk,
            end: addr,
            flags: MapFlags::PRIVATE | MapFlags::ANONYMOUS,
            prot: ProtFlags::READ | ProtFlags::WRITE,
            file: None,
            offset: 0,
            phys_addr: None,
        });
    } else {
        let aligned_addr = align_up(addr, 4096);
        mmap::munmap(process, addr, process.brk - aligned_addr)?;
    }

    process.brk = addr;
    Ok(addr)
}
