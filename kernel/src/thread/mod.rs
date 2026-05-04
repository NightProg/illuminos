pub mod process;
pub mod signal;

use crate::allocator::process_paging::ProcessPageTable;
use crate::thread::process::{PROCESSES, Pid};
use crate::thread::signal::{Signal, SignalState};
use crate::{dbg, println};
use alloc::boxed::Box;
use alloc::collections::btree_map::BTreeMap;
use alloc::vec::Vec;
use core::arch::global_asm;
use spin::Mutex;
use x86_64::{PhysAddr, VirtAddr};

pub static PENDING_FREE: Mutex<Vec<ProcessPageTable>> = Mutex::new(Vec::new());
pub const MAX_TASKS: usize = 64;
pub static SCHEDULER: Mutex<Scheduler> = Mutex::new(Scheduler {
    tasks: BTreeMap::new(),
    current: 0,
    is_ready: false,
    is_running: false,
    exit_callbacks: Vec::new(),
});

pub type TaskId = usize;

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct TaskContext {
    pub r15: u64,    // 0x00
    pub r14: u64,    // 0x08
    pub r13: u64,    // 0x10
    pub r12: u64,    // 0x18
    pub r11: u64,    // 0x20
    pub r10: u64,    // 0x28
    pub r9: u64,     // 0x30
    pub r8: u64,     // 0x38
    pub rbx: u64,    // 0x40
    pub rbp: u64,    // 0x48
    pub rflags: u64, // 0x50
    pub rax: u64,    // 0x58  ← valeur de retour syscall
    pub rip: u64,    // 0x60
    pub rsp: u64,    // 0x68
}

impl TaskContext {
    pub const fn default() -> Self {
        TaskContext {
            r15: 0,
            r14: 0,
            r13: 0,
            r12: 0,
            r11: 0,
            r10: 0,
            r9: 0,
            r8: 0,
            rbx: 0,
            rbp: 0,
            rflags: 0x202, // IF=1
            rax: 0,
            rip: 0,
            rsp: 0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Task {
    pub signals: SignalState,
    pub id: usize,
    pub parent: Pid,
    pub context: TaskContext,
    kernel_stack: VirtAddr,
    kernel_stack_top: VirtAddr,
    state: TaskState,
    cr3: Option<u64>,
}

impl Task {
    pub fn new_user_with_kstack(
        id: usize,
        parent: Pid,
        entry: u64,
        user_rsp: u64,
        pml4_phys: u64,
        kernel_stack: VirtAddr,
        kernel_stack_size: usize,
    ) -> Self {
        let kernel_stack_top = kernel_stack + kernel_stack_size as u64;

        let mut sp = kernel_stack_top.as_u64();

        unsafe fn push(sp: &mut u64, val: u64) {
            *sp -= 8;
            *(*sp as *mut u64) = val;
        }

        unsafe {
            push(&mut sp, 0x18 | 3); // ss  (user data, RPL 3)
            push(&mut sp, user_rsp); // rsp user
            push(&mut sp, 0x202); // rflags (IF=1)
            push(&mut sp, 0x20 | 3); // cs  (user code, RPL 3)
            push(&mut sp, entry); // rip (e_entry)
        }

        crate::println_serial!(
            "new_user task={} kstack={:#x} kstack_top={:#x} iretq_frame_rsp={:#x}",
            id,
            kernel_stack.as_u64(),
            kernel_stack_top.as_u64(),
            sp,
        );

        let context = TaskContext {
            rip: iretq_trampoline as u64,
            rsp: sp,
            ..TaskContext::default()
        };

        Task {
            id,
            context,
            kernel_stack,
            kernel_stack_top,
            parent,
            state: TaskState::Ready,
            cr3: Some(pml4_phys),
            signals: SignalState::default(),
        }
    }
    pub fn new_user(
        id: usize,
        parent: Pid,
        entry: u64,
        user_rsp: u64,
        pml4_phys: u64,
        kernel_stack_size: usize,
    ) -> Self {
        let kernel_stack = unsafe {
            let layout =
                core::alloc::Layout::from_size_align(kernel_stack_size + 0x1000, 0x1000).unwrap();
            let ptr = alloc::alloc::alloc_zeroed(layout);
            VirtAddr::new(ptr as u64 + 0x1000)
        };
        Self::new_user_with_kstack(
            id,
            parent,
            entry,
            user_rsp,
            pml4_phys,
            kernel_stack,
            kernel_stack_size,
        )
    }

    pub fn new(id: usize, func: extern "C" fn()) -> Self {
        let stack_size = 0x8000;
        let kernel_stack = unsafe {
            let layout = core::alloc::Layout::from_size_align(stack_size, 0x1000).unwrap();
            let ptr = alloc::alloc::alloc_zeroed(layout);
            VirtAddr::new(ptr as u64)
        };

        let mut stack_top = kernel_stack + stack_size as u64;

        stack_top -= 8u64;
        unsafe {
            *(stack_top.as_mut_ptr::<u64>()) = task_exit as u64;
        }

        let context = TaskContext {
            rip: func as u64,
            rsp: stack_top.as_u64(),
            ..TaskContext::default()
        };
        Task {
            id,
            parent: 0,
            context,
            kernel_stack,
            state: TaskState::Ready,
            cr3: None,
            kernel_stack_top: stack_top,
            signals: SignalState::default(),
        }
    }

    pub fn send_signal(&mut self, signal: Signal) {
        self.signals.send(signal);
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TaskState {
    Ready,
    Running,
    Blocked,
    Zombie,
}

pub struct Scheduler {
    pub tasks: BTreeMap<TaskId, Task>,
    pub current: usize,
    pub is_ready: bool,
    pub is_running: bool,
    pub exit_callbacks: Vec<(usize, Box<dyn FnOnce() + Send>)>,
}

impl Scheduler {
    pub fn init() {}
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }

    pub fn add_task(&mut self, task: Task) -> usize {
        let id = self.tasks.len();
        let mut task = task;
        task.id = id;
        self.tasks.insert(id, task);
        id
    }

    pub fn add_kernel_task(&mut self, func: extern "C" fn()) -> TaskState {
        let id = self.tasks.len();
        let task = Task::new(id, func);
        self.tasks.insert(id, task);
        TaskState::Ready
    }

    pub fn add_user_task(
        &mut self,
        entry: u64,
        user_rsp: u64,
        pml4_phys: u64,
        kernel_stack_size: usize,
        parent_pid: Pid,
    ) -> TaskId {
        let id = self.tasks.len();
        let task = Task::new_user(
            id,
            parent_pid,
            entry,
            user_rsp,
            pml4_phys,
            kernel_stack_size,
        );
        self.tasks.insert(id, task);
        id
    }

    pub fn current_task(&self) -> Option<&Task> {
        if self.tasks.is_empty() {
            None
        } else {
            self.tasks.get(&self.current)
        }
    }

    pub fn current_task_mut(&mut self) -> Option<&mut Task> {
        if self.tasks.is_empty() {
            None
        } else {
            self.tasks.get_mut(&self.current)
        }
    }

    pub fn next_task(&mut self) -> bool {
        if self.tasks.is_empty() {
            return false;
        }
        let start = self.current;
        loop {
            self.current = (self.current + 1) % self.tasks.len();
            if matches!(self.current_task().unwrap().state, TaskState::Ready) {
                return true;
            }
            if self.current == start {
                return false;
            }
        }
    }

    pub fn exit_current_task(&mut self) {
        if !self.tasks.is_empty() {
            let current = self.current;
            self.current_task_mut().unwrap().state = TaskState::Zombie;

            let mut i = 0;
            while i < self.exit_callbacks.len() {
                if self.exit_callbacks[i].0 == current {
                    let (_, cb) = self.exit_callbacks.remove(i);
                    cb();
                } else {
                    i += 1;
                }
            }
        }
    }

    pub fn register_exit_callback(&mut self, wait_for_id: TaskId, cb: Box<dyn FnOnce() + Send>) {
        if let Some(task) = self.tasks.get(&wait_for_id) {
            if matches!(task.state, TaskState::Zombie) {
                cb();
                return;
            }
        }
        self.exit_callbacks.push((wait_for_id, cb));
    }

    pub fn ready(&mut self) {
        self.is_ready = true;
    }
}

pub fn wait_for_task<W>(id: usize, mut wait_func: W)
where
    W: FnMut(),
{
    loop {
        {
            let scheduler = SCHEDULER.lock();
            if let Some(task) = scheduler.tasks.get(&id) {
                if matches!(task.state, TaskState::Zombie) {
                    break;
                }
            } else {
                break;
            }
        }

        wait_func();
        schedule();
    }
}

pub fn on_quit_task<F>(id: usize, mut callback: F)
where
    F: FnMut(),
{
    loop {
        {
            let scheduler = SCHEDULER.lock();
            if let Some(task) = scheduler.tasks.get(&id) {
                if matches!(task.state, TaskState::Zombie) {
                    break;
                }
            } else {
                break;
            }
        }
        schedule();
    }
    callback();
}

global_asm!(include_str!("../../asm/switch.asm"));

unsafe extern "C" {
    fn switch_to(task_context: *mut TaskContext, next_context: *const TaskContext);
}

pub fn schedule() {
    let mut interrupts_enabled = x86_64::instructions::interrupts::are_enabled();
    if interrupts_enabled {
        x86_64::instructions::interrupts::disable();
    }
    unsafe {
        let mut prev_task_ptr: *mut TaskContext = core::ptr::null_mut();
        let mut next_task_ptr: *const TaskContext = core::ptr::null();
        let mut first_run = false;

        {
            let mut scheduler = SCHEDULER.lock();

            if !scheduler.is_ready() {
                if interrupts_enabled {
                    x86_64::instructions::interrupts::enable();
                }
                return;
            }

            if !scheduler.is_running {
                if !scheduler.tasks.is_empty() {
                    scheduler.is_running = true;
                    let next = scheduler.current;
                    scheduler.tasks.get_mut(&next).unwrap().state = TaskState::Running;
                    let next_task = &scheduler.tasks.get(&next).unwrap();
                    next_task_ptr = &next_task.context as *const TaskContext;
                    first_run = true;
                } else {
                    if interrupts_enabled {
                        x86_64::instructions::interrupts::enable();
                    }
                    return;
                }
            } else if scheduler.tasks.len() > 1 {
                let prev = scheduler.current;
                if !matches!(
                    scheduler.tasks.get_mut(&prev).unwrap().state,
                    TaskState::Zombie
                ) {
                    scheduler.tasks.get_mut(&prev).unwrap().state = TaskState::Ready;
                }
                let found = scheduler.next_task();
                if !found {
                    return;
                }
                let next = scheduler.current;
                scheduler.tasks.get_mut(&next).unwrap().state = TaskState::Running;
                let next_cr3 = scheduler.tasks.get_mut(&next).unwrap().cr3;
                let next_stack_top = scheduler.tasks.get_mut(&next).unwrap().kernel_stack_top;

                prev_task_ptr =
                    &mut scheduler.tasks.get_mut(&prev).unwrap().context as *mut TaskContext;
                next_task_ptr =
                    &scheduler.tasks.get_mut(&next).unwrap().context as *const TaskContext;

                crate::gdt::set_tss_rsp0(next_stack_top);

                if let Some(cr3) = next_cr3 {
                    unsafe {
                        x86_64::registers::control::Cr3::write(
                            x86_64::structures::paging::PhysFrame::containing_address(
                                x86_64::PhysAddr::new(cr3),
                            ),
                            x86_64::registers::control::Cr3Flags::empty(),
                        );
                    }
                }
            }
        }

        if first_run {
            println!("First run of scheduler, jumping to first task");
            #[repr(C)]
            struct MainContext {
                context: TaskContext,
            }
            static mut MAIN_CONTEXT: MainContext = MainContext {
                context: TaskContext::default(),
            };
            let next_cr3 = {
                let scheduler = SCHEDULER.lock();
                let t = scheduler.current_task().unwrap();
                (t.cr3, t.kernel_stack_top)
            };

            if let Some(cr3) = next_cr3.0 {
                crate::gdt::set_tss_rsp0(next_cr3.1);

                unsafe {
                    x86_64::registers::control::Cr3::write(
                        x86_64::structures::paging::PhysFrame::containing_address(
                            x86_64::PhysAddr::new(cr3),
                        ),
                        x86_64::registers::control::Cr3Flags::empty(),
                    );
                }
            }
            switch_to(&mut MAIN_CONTEXT.context, next_task_ptr);
        } else {
            switch_to(prev_task_ptr, next_task_ptr);
        }

        if interrupts_enabled {
            x86_64::instructions::interrupts::enable();
        }
    }
}

pub extern "C" fn yield_now() {
    schedule();
}

pub extern "C" fn exit() {
    task_exit();
}
pub extern "C" fn task_exit() {
    let interrupts_enabled = x86_64::instructions::interrupts::are_enabled();
    if interrupts_enabled {
        x86_64::instructions::interrupts::disable();
    }

    {
        let mut procs = PROCESSES.lock();
        let task_id = SCHEDULER.lock().current;
        let stack_top = SCHEDULER
            .lock()
            .tasks
            .get(&task_id)
            .map(|t| t.kernel_stack_top)
            .unwrap();

        crate::println_serial!(
            "task_exit: task={} kernel_stack_top={:#x?}",
            task_id,
            stack_top
        );

        if let Some((_, proc)) = procs.iter_mut().find(|p| p.1.tasks.contains(&task_id)) {
            if proc.is_terminated_in_scheduler() {
                PENDING_FREE.lock().push(proc.pml4_table);
                proc.state = process::ProcessState::Terminated;
            }
        }
    }

    unsafe {
        SCHEDULER.lock().exit_current_task();
    }
    println!("QUITTING TASK");
    schedule();

    loop {
        x86_64::instructions::hlt();
    }
}

#[unsafe(naked)]
unsafe extern "C" fn iretq_trampoline() -> ! {
    core::arch::naked_asm!("xor rax, rax", "xor r9, r9", "iretq",);
}

pub extern "C" fn gc_task() {
    loop {
        let to_free: Vec<ProcessPageTable> = PENDING_FREE.lock().drain(..).collect();
        for mut pt in to_free {
            println!(
                "dealloc process page table: {:#x}",
                pt.pml4_frame.start_address().as_u64()
            );
            pt.dealloc();
        }
        yield_now();
    }
}
