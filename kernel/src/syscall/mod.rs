mod file;
mod mem;
mod signal;
mod task;
use crate::allocator::mmap;
use crate::allocator::vma::MapFlags;
use crate::allocator::vma::ProtFlags;
use crate::context::GLOBAL_CONTEXT;
use crate::dbg;
use crate::drivers::disk;
use crate::fs::FileSystem;
use crate::gdt::GDT;

use crate::io::stdin;
use crate::io::stdout;
use crate::thread::SCHEDULER;
use crate::thread::TaskState;
use crate::thread::process;
use crate::thread::process::PROCESSES;
use crate::thread::process::ProcessArguments;
use crate::thread::process::current_process;
use crate::thread::schedule;
use crate::thread::signal::Signal;
use crate::{error, fs};
use crate::{info, println_serial};
use alloc::boxed::Box;
use alloc::rc::Rc;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;
use core::arch::global_asm;
use core::cell::RefCell;
use core::cmp::PartialEq;
use x86_64::VirtAddr;
use x86_64::registers::model_specific::{Efer, EferFlags, Msr};
pub const SYS_ERR: u64 = u64::MAX;
pub const SYS_EXIT: u64 = 0;
pub const SYS_READ: u64 = 1;
pub const SYS_WRITE: u64 = 2;
pub const SYS_OPENF: u64 = 3;
pub const SYS_YIELD: u64 = 4;
pub const SYS_FB_INFO: u64 = 5;
pub const SYS_FB_MMAP: u64 = 6;
pub const SYS_FB_SWAP: u64 = 7;
pub const SYS_SIGNAL: u64 = 8;
pub const SYS_RAISE: u64 = 9;
pub const SYS_FORK: u64 = 10;
pub const SYS_GETPID: u64 = 11;
pub const SYS_WAITPID: u64 = 12;
pub const SYS_MMAP: u64 = 13;
pub const SYS_PIPE: u64 = 14;
pub const SYS_DUP2: u64 = 15;
pub const SYS_EXECVE: u64 = 16;
pub const SYS_CLOSE: u64 = 17;
pub const SYS_MUNMAP: u64 = 18;
pub const SYS_EXIT_GROUP: u64 = 19;
pub const SYS_MPROTECT: u64 = 20;
pub const SYS_BRK: u64 = 21;
pub const SYS_IOCTL: u64 = 22;
pub const SYS_SLEEP: u64 = 23;
pub const SYS_THREAD_CREATE: u64 = 24; 


pub static SYSCALL_TABLE: spin::Once<SyscallTable> = spin::Once::new();

#[derive(Debug)]
pub enum SyscallError {
    InvalidSyscall,
    InvalidFileDescriptor,
    FileReadError,
    FileWriteError,
    FileOpenError,
    PipeCreationFailed,
    ProcessCreationFailed,
    ProcessNotFound,
    NotCurrentProcess,
    MmapFailed,
    MunmapFailed,
    MprotectFailed,
    BrkFailed,
    IoctlFailed,
    ForkFailed,
    ExecveFailed,
    SignalError,
    SleepError(String),
    UnknownError,
}

pub type SyscallResult = Result<Option<u64>, SyscallError>;
pub type SyscallHandler = fn(&mut SyscallCtx) -> SyscallResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyscallKind {
    Exit,
    Read,
    Write,
    OpenFile,
    Yield,
    FbInfo,
    FbMmap,
    FbSwap,
    Signal,
    Raise,
    Fork,
    GetPid,
    WaitPid,
    Mmap,
    Pipe,
    Dup2,
    Execve,
    Close,
    Munmap,
    ExitGroup,
    Mprotect,
    Brk,
    Ioctl,
    Sleep,
    ThreadCreate,
}

impl SyscallKind {
    pub fn from_u64(value: u64) -> Option<Self> {
        Some(match value {
            SYS_EXIT => SyscallKind::Exit,
            SYS_READ => SyscallKind::Read,
            SYS_WRITE => SyscallKind::Write,
            SYS_OPENF => SyscallKind::OpenFile,
            SYS_YIELD => SyscallKind::Yield,
            SYS_FB_INFO => SyscallKind::FbInfo,
            SYS_FB_MMAP => SyscallKind::FbMmap,
            SYS_FB_SWAP => SyscallKind::FbSwap,
            SYS_SIGNAL => SyscallKind::Signal,
            SYS_RAISE => SyscallKind::Raise,
            SYS_FORK => SyscallKind::Fork,
            SYS_GETPID => SyscallKind::GetPid,
            SYS_WAITPID => SyscallKind::WaitPid,
            SYS_MMAP => SyscallKind::Mmap,
            SYS_PIPE => SyscallKind::Pipe,
            SYS_DUP2 => SyscallKind::Dup2,
            SYS_EXECVE => SyscallKind::Execve,
            SYS_CLOSE => SyscallKind::Close,
            SYS_MUNMAP => SyscallKind::Munmap,
            SYS_EXIT_GROUP => SyscallKind::ExitGroup,
            SYS_MPROTECT => SyscallKind::Mprotect,
            SYS_BRK => SyscallKind::Brk,
            SYS_IOCTL => SyscallKind::Ioctl,
            SYS_SLEEP => SyscallKind::Sleep,
            SYS_THREAD_CREATE => SyscallKind::ThreadCreate,
            _ => return None,
        })
    }
}

pub struct Syscall {
    pub kind: SyscallKind,
    pub handler: SyscallHandler,
}

pub struct SyscallTable {
    pub syscalls: Vec<Syscall>,
}

impl SyscallTable {
    pub const fn new() -> SyscallTable {
        Self {
            syscalls: Vec::new(),
        }
    }

    pub fn register(&mut self, kind: SyscallKind, handler: SyscallHandler) -> &mut Self {
        self.syscalls.push(Syscall { kind, handler });
        self
    }

    pub fn handle_syscall(&self, ctx: &mut SyscallCtx) -> SyscallResult {
        let syscall_kind =
            SyscallKind::from_u64(ctx.syscall_id).ok_or(SyscallError::InvalidSyscall)?;
        let syscall = self
            .syscalls
            .iter()
            .find(|s| s.kind == syscall_kind)
            .ok_or(SyscallError::InvalidSyscall)?;
        (syscall.handler)(ctx)
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct SyscallCtx {
    pub syscall_id: u64,
    pub rip: u64,
    pub rflags: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub r8: u64,  //        - offset 0x30
    pub r9: u64,  //        - offset 0x38
    pub r10: u64, //        - offset 0x40
    pub rbx: u64, //        - offset 0x48
    pub rbp: u64, //        - offset 0x50
    pub r12: u64, //        - offset 0x58
    pub r13: u64, //        - offset 0x60
    pub r14: u64, //        - offset 0x68
    pub r15: u64, //        - offset 0x70
    pub rsp: u64, //        - offset 0x78
}
pub fn init_syscall() {
    SYSCALL_TABLE.call_once(|| {
        let mut syscall_table = SyscallTable::new();
        syscall_table
            .register(SyscallKind::Exit, |ctx| {
                let exit_code = ctx.rdi as u8;
                crate::thread::exit(exit_code);
                Ok(None)
            })
            .register(SyscallKind::ExitGroup, |ctx| {
                let exit_code = ctx.rdi as u8;
                crate::thread::exit_group(exit_code);
                Ok(None)
            })
            .register(SyscallKind::Yield, |_| {
                crate::thread::yield_now();
                Ok(None)
            })
            .register(SyscallKind::Read, file::sys_read_impl)
            .register(SyscallKind::Write, file::sys_write_impl)
            .register(SyscallKind::OpenFile, file::sys_openf_impl)
            .register(SyscallKind::Close, file::sys_close_impl)
            .register(SyscallKind::Dup2, file::sys_dup2_impl)
            .register(SyscallKind::Ioctl, file::sys_ioctl_impl)
            .register(SyscallKind::Pipe, file::sys_pipe_impl)
            .register(SyscallKind::GetPid, |ctx| {
                let pid = current_process().unwrap().pid;
                Ok(Some(pid as u64))
            })
            .register(SyscallKind::Fork, task::sys_fork_impl)
            .register(SyscallKind::WaitPid, |ctx| {
                let pid = ctx.rdi;
                process::wait(pid);
                Ok(None)
            })
            .register(SyscallKind::Execve, task::sys_execve_impl)
            .register(SyscallKind::Mmap, mem::sys_mmap_impl)
            .register(SyscallKind::Munmap, mem::sys_munmap_impl)
            .register(SyscallKind::Mprotect, mem::sys_mprotect_impl)
            .register(SyscallKind::Brk, mem::sys_brk_impl)
            .register(SyscallKind::Signal, signal::sys_signal_impl)
            .register(SyscallKind::Raise, signal::sys_raise_impl)
            .register(SyscallKind::Sleep, task::sys_sleep_impl);
        syscall_table
    });
    let mut efer = Efer::read();

    efer.insert(EferFlags::SYSTEM_CALL_EXTENSIONS | EferFlags::NO_EXECUTE_ENABLE);
    unsafe {
        Efer::write(efer);
    }

    let mut lstar = Msr::new(0xC0000082);
    let mut star = Msr::new(0xC0000081);
    let mut sfmask = Msr::new(0xC0000084);

    let sys_handler_addr = sys_handler as u64;

    unsafe {
        lstar.write(sys_handler_addr);
        star.write(0x0013000800000000u64);
        sfmask.write(1 << 9);
    }
}

#[unsafe(no_mangle)]
extern "C" fn sys_dispatch(sys_ctx: *mut SyscallCtx) {
    let mut ctx = unsafe { &mut *sys_ctx };
    let res = SYSCALL_TABLE.r#try().unwrap().handle_syscall(ctx);
    if let Err(ref e) = res {
        error!("Syscall error: {:?}", e);
        ctx.r9 = SYS_ERR;
    }
    if let Ok(Some(ret)) = res {
        ctx.r9 = ret;
    }

    if SCHEDULER.lock().current_task().unwrap().state != TaskState::Ready {
        println_serial!("schedule");
        schedule();
    }

    x86_64::instructions::interrupts::disable();
}

global_asm!(include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/asm/syscall.asm"
)));

unsafe extern "C" {
    fn sys_handler();
}
