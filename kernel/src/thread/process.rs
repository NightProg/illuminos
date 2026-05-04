use crate::{
    allocator::{paging::KERNEL_PAGING_MANAGER, process_paging::ProcessPageTable},
    dbg,
    gdt::KERNEL_STACK_SIZE,
    println, println_serial,
    syscall::SyscallCtx,
    thread::{Task, iretq_trampoline, yield_now},
};
use alloc::{collections::btree_map::BTreeMap, vec::Vec};
use core::{hint::unreachable_unchecked, sync::atomic::AtomicU64};
use x86_64::{
    PhysAddr, VirtAddr,
    registers::control::{Cr3, Cr3Flags},
    structures::paging::{
        FrameAllocator, Mapper, OffsetPageTable, PageTable, PageTableFlags, PhysFrame, Size4KiB,
    },
};

use spin::Mutex;

use crate::thread::{SCHEDULER, Scheduler};

pub static PID: AtomicU64 = AtomicU64::new(1);

pub static PROCESSES: Mutex<BTreeMap<Pid, Process>> = Mutex::new(BTreeMap::new());

pub const USER_STACK_TOP: u64 = 0x0000_7FFF_FF00_0000;
pub const USER_STACK_SIZE: u64 = 0x8000;

#[derive(Debug, PartialEq, Clone)]
pub enum ProcessState {
    Ready,
    Running,
    Waiting,
    Terminated,
}

pub type Pid = u64;

#[derive(Debug, Clone)]
pub struct Process {
    pub pid: Pid,
    pub pml4_table: ProcessPageTable,
    pub state: ProcessState,
    pub tasks: Vec<usize>,
}

impl Process {
    pub fn create(pml4_table: ProcessPageTable) -> Pid {
        let mut processes_lock = PROCESSES.lock();
        let process = Process::new(pml4_table);
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
        }
    }

    pub fn new(pml4_table: ProcessPageTable) -> Self {
        Process {
            pid: PID.fetch_add(1, core::sync::atomic::Ordering::SeqCst),
            pml4_table,
            state: ProcessState::Ready,
            tasks: Vec::new(),
        }
    }

    /*pub unsafe fn exec(&self) -> ! {
        unsafe {
            let (kernel_frame, _) = Cr3::read();
            let mut paging_manager_lock = KERNEL_PAGING_MANAGER.lock();
            let phys_offset = paging_manager_lock
                .as_ref()
                .expect("KERNEL_PAGING_MANAGER not initialized before exec")
                .mapper
                .phys_offset();
            drop(paging_manager_lock); // Release lock before possible prints or context switch

            let rip: u64;
            core::arch::asm!("lea {}, [rip]", out(reg) rip);
            crate::println_serial!("exec RIP = {:#x}", rip);

            // PML4 index de cette adresse
            let pml4_idx = (rip >> 39) & 0x1FF;
            crate::println_serial!("PML4 index = {}", pml4_idx);

            // Vérifier aussi le nouveau PML4
            let new_pml4_virt = phys_offset + self.pml4_frame.start_address().as_u64();
            let new_pml4: &PageTable = unsafe { &*(new_pml4_virt.as_ptr()) };
            for i in 256..512 {
                if !new_pml4[i].is_unused() {
                    crate::println_serial!("new PML4[{}] = {:?}", i, new_pml4[i]);
                }
            }
            crate::println_serial!(
                "(1) iretq → rip={:#x} rsp={:#x} cr3={:#x}",
                self.rip.as_u64(),
                self.rsp.as_u64(),
                self.pml4_frame.start_address().as_u64()
            );
            Cr3::write(self.pml4_frame, Cr3Flags::empty());
            crate::println_serial!(
                "(2) iretq → rip={:#x} rsp={:#x} cr3={:#x}",
                self.rip.as_u64(),
                self.rsp.as_u64(),
                self.pml4_frame.start_address().as_u64()
            );
            jump_to_user(self.rip, self.rsp);
        }
    }*/

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
        self.add_to_scheduler(entry_point, stack_top)
    }

    pub fn add_to_scheduler(&mut self, rip: VirtAddr, rsp: VirtAddr) -> usize {
        let task_id = SCHEDULER.lock().add_user_task(
            rip.as_u64(),
            rsp.as_u64(),
            self.pml4_table.pml4_frame.start_address().as_u64(),
            KERNEL_STACK_SIZE,
            self.pid,
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
                task.state == crate::thread::TaskState::Zombie
            } else {
                true // Si la tâche n'est pas trouvée, on considère qu'elle est terminée
            }
        })
    }

    pub fn free_resources(&mut self) {
        self.pml4_table.dealloc();
        self.state = ProcessState::Terminated;
    }
}
unsafe fn jump_to_user(rip: VirtAddr, rsp: VirtAddr) -> ! {
    const USER_CODE_SEGMENT: u64 = 0x20 | 3; // Segment 4, RPL 3
    const USER_DATA_SEGMENT: u64 = 0x18 | 3; // Segment 3, RPL 3
    const RFLAGS_IF: u64 = 0x202;

    unsafe {
        core::arch::asm!(
            // Pousser le frame iretq avec des registres fixes
            "push {ds}",       // ss
            "push r10",        // rsp user
            "push {rflags}",   // rflags
            "push {cs}",       // cs
            "push r11",        // rip
            // Changer les segments après (ax est libre)
            "mov ax, {ds}",
            "mov ds, ax",
            "mov es, ax",
            "mov fs, ax",
            "mov gs, ax",
            "iretq",
            ds     = const USER_DATA_SEGMENT,
            cs     = const USER_CODE_SEGMENT,
            rflags = const RFLAGS_IF,
            in("r10") rsp.as_u64(),
            in("r11") rip.as_u64(),
            options(noreturn)
        );
    }
}

pub fn spawn(pid: Pid, rip: VirtAddr) -> Option<usize> {
    let mut processus_lock = PROCESSES.lock();
    let process = processus_lock.get_mut(&pid)?;
    Some(process.spawn_user_thread(rip))
}

/*
pub fn fork(ctx: &SyscallCtx) -> Option<Pid> {
    let child = SCHEDULER.lock().current_task()?.clone();
    let mut processes_lock = PROCESSES.lock();
    let parent_process = processes_lock.iter().find(|p| p.pid == child.parent)?;

    let mut new_pml4_table = ProcessPageTable::copy_from(&parent_process.pml4_table);

    let mut new_process = Process::new(new_pml4_table);
    println!("RIP fork: {:?}", ctx.rip);
    let task_id = new_process.add_to_scheduler(VirtAddr::new(ctx.rip), VirtAddr::new(ctx.rsp));
    SCHEDULER.lock().tasks[task_id].context.r9 = 0;

    let child_pid = new_process.pid;
    processes_lock.push(new_process);
    Some(child_pid)
}*/
pub fn fork(task_id: usize, sysctx: *mut SyscallCtx) -> Option<Pid> {
    let task = SCHEDULER.lock().tasks.get(&task_id)?.clone();
    let sysctx = unsafe { sysctx.as_ref().unwrap() };
    println_serial!("fork: child rip={:#x} rsp={:#x}", sysctx.rip, sysctx.rsp); // ← ici

    let pid = task.parent;

    let process = PROCESSES.lock().get(&pid).cloned().unwrap();

    let new_pid = Process::create(ProcessPageTable::copy_from(process.pml4_table));
    let new_process = PROCESSES.lock().get(&new_pid).cloned().unwrap();

    let mut new_task = Task::new_user(
        0,
        new_pid,
        sysctx.rip,
        sysctx.rsp,
        new_process.pml4_table.pml4_frame.start_address().as_u64(),
        KERNEL_STACK_SIZE,
    );

    new_task.context.r15 = sysctx.r15;
    new_task.context.r14 = sysctx.r14;
    new_task.context.r13 = sysctx.r13;
    new_task.context.r12 = sysctx.r12;
    new_task.context.r10 = sysctx.r10;
    new_task.context.r9 = sysctx.r9;
    new_task.context.r8 = sysctx.r8;
    new_task.context.rbx = sysctx.rbx;
    new_task.context.rbp = sysctx.rbp;
    new_task.context.rax = 0; // ← child retourne 0, pas r9

    SCHEDULER.lock().add_task(new_task);
    Some(new_pid)
}
fn test_function() -> ! {
    println_serial!("Hello from the child(test function) process!");
    loop {}
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
