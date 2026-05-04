use crate::context::GLOBAL_CONTEXT;
use crate::drivers::disk;
use crate::fs::FileSystem;
use crate::gdt::GDT;
use crate::io::port::Fd;
use crate::io::stdin;
use crate::io::stdout;
use crate::thread::SCHEDULER;
use crate::thread::process;
use crate::thread::signal::Signal;
use crate::{error, fs};
use crate::{info, println_serial};
use alloc::boxed::Box;
use alloc::rc::Rc;
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

pub static mut CURRENT_DISK_ID: usize = 0;
pub static SYS_CONF: SysConf = SysConf::new();

pub struct SysConf {
    pub disk_kind_used: disk::DiskKind,
    pub ext2_used: bool,
}

impl SysConf {
    pub const fn new() -> Self {
        Self {
            disk_kind_used: disk::DiskKind::AtaPio,
            ext2_used: true,
        }
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct SyscallCtx {
    pub syscall_id: u64, // rax    - offset 0x00
    pub rip: u64,        // rcx    - offset 0x08 (sauvé par syscall)
    pub rflags: u64,     // r11    - offset 0x10 (sauvé par syscall)
    pub rdx: u64,        //        - offset 0x18
    pub rsi: u64,        //        - offset 0x20
    pub rdi: u64,        //        - offset 0x28
    pub r8: u64,         //        - offset 0x30
    pub r9: u64,         //        - offset 0x38
    pub r10: u64,        //        - offset 0x40
    pub rbx: u64,        //        - offset 0x48
    pub rbp: u64,        //        - offset 0x50
    pub r12: u64,        //        - offset 0x58
    pub r13: u64,        //        - offset 0x60
    pub r14: u64,        //        - offset 0x68
    pub r15: u64,        //        - offset 0x70
    pub rsp: u64,        //        - offset 0x78
}
pub fn init_syscall() {
    let mut efer = Efer::read();

    efer.insert(EferFlags::SYSTEM_CALL_EXTENSIONS | EferFlags::NO_EXECUTE_ENABLE);
    unsafe {
        Efer::write(efer);
    }

    let mut lstar = Msr::new(0xC0000082);
    let mut star = Msr::new(0xC0000081);
    let mut sfmask = Msr::new(0xC0000084);

    let sys_handler_addr = sys_handler as u64;

    println_serial!("{:X?}", sys_handler_addr);

    unsafe {
        lstar.write(sys_handler_addr);
        star.write(0x0013000800000000u64);
        sfmask.write(1 << 9);
    }
}

#[unsafe(no_mangle)]
extern "C" fn sys_dispatch(sys_ctx: *mut SyscallCtx) {
    let mut ctx = unsafe { &mut *sys_ctx };
    crate::println_serial!(
        "sys_dispatch syscall_id={} rip={:#x} rsp={:#x}",
        ctx.syscall_id,
        ctx.rip,
        ctx.rsp
    );

    //x86_64::instructions::interrupts::enable();

    match ctx.syscall_id {
        SYS_EXIT => {
            crate::thread::exit();
        }
        SYS_YIELD => {
            crate::thread::yield_now();
        }
        SYS_READ => {
            let fd = ctx.rdi;
            let mut buf = ctx.rsi as *mut u8;
            let mut buf_len = ctx.rdx;
            let slice = unsafe { core::slice::from_raw_parts_mut(buf, buf_len as usize) };
            if fd == 0 {
                stdin::blocking_read(slice);
            } else {
                let fd = Fd(fd as usize);
                let buf = fd.read();
                if let Some(x) = buf {
                    slice.copy_from_slice(&x);
                } else {
                    ctx.r9 = SYS_ERR;
                }
            }
        }
        SYS_WRITE => {
            let fd = ctx.rdi;
            let buf = ctx.rsi as *const u8;
            let buf_len = ctx.rdx;
            let slice = unsafe { core::slice::from_raw_parts(buf, buf_len as usize) };

            if fd == 1 {
                println_serial!("WRITTING : {}", str::from_utf8(slice).unwrap());
                stdout::write(slice);
            } else {
                let fd = Fd(fd as usize);

                fd.write(slice);
            }
        }

        SYS_OPENF => {
            let addr = unsafe { ctx.rdi as *mut i8 };

            let fd_addr = unsafe { ctx.rsi as *mut u64 };

            let flags = fs::OpenFlags::from_bits(ctx.rdx);

            if flags.is_none() {
                ctx.r9 = SYS_ERR;
                info!("Can't parse flags");
                return;
            }
            let flags = flags.unwrap();

            let cstr = unsafe { core::ffi::CStr::from_ptr(addr) };
            let string = cstr.to_string_lossy().to_string();
            let mut file = unsafe {
                GLOBAL_CONTEXT.open(&string, flags).unwrap_or_else(|_| {
                    ctx.r9 = SYS_ERR;
                    error!("Failed to open file");
                    Fd(0xFFFFFFFF)
                })
            };

            unsafe {
                *fd_addr = file.0 as u64;
            }
        }

        SYS_SIGNAL => {
            let signum = ctx.rdi;
            let handler = ctx.rsi;
            info!(
                "Register signal handler: signum={}, handler={:X}",
                signum, handler
            );
            let sig = Signal::from_u64(signum);
            if sig.is_none() {
                ctx.r9 = SYS_ERR;
                return;
            }
            let sig = sig.unwrap();

            info!("Parsed signal: {:?}", sig);
            let mut scheduler = crate::thread::SCHEDULER.lock();
            if let Some(task) = scheduler.current_task_mut() {
                if signum < 32 {
                    info!(
                        "Registering handler for signal {:?} at {:X} for task {}",
                        sig, handler, task.id
                    );
                    task.signals.handlers[signum as usize] = handler;
                } else {
                    ctx.r9 = SYS_ERR;
                }
            }
        }

        SYS_RAISE => {
            let target_pid = ctx.rdi as usize;
            let signum = ctx.rsi;

            let sig = Signal::from_u64(signum);
            if sig.is_none() {
                ctx.r9 = SYS_ERR;
                return;
            }

            let mut scheduler = crate::thread::SCHEDULER.lock();
            let task = scheduler
                .tasks
                .iter_mut()
                .map(|kv| kv.1)
                .find(|t| t.id == target_pid);

            match task {
                Some(t) => t.signals.send(sig.unwrap()),
                None => ctx.r9 = 0xFF,
            }
        }

        SYS_GETPID => {
            let pid_ptr = ctx.rdi as *mut u64;
            let scheduler = crate::thread::SCHEDULER.lock();
            if let Some(task) = scheduler.current_task() {
                unsafe {
                    *pid_ptr = task.parent;
                }
                ctx.r9 = 0;
            } else {
                ctx.r9 = SYS_ERR;
            }
        }

        SYS_FB_INFO => {
            let info_ptr = ctx.rdi as *mut info::FrameBufferInfo;
            unsafe {
                if let Some(fb) = GLOBAL_CONTEXT.framebuffer.as_ref() {
                    *info_ptr = fb.info();
                } else {
                    ctx.r9 = SYS_ERR;
                }
            }
        }

        SYS_FB_MMAP => {
            let addr_ptr = ctx.rdi as *mut u64;
            unsafe {
                if let Some(fb) = GLOBAL_CONTEXT.framebuffer.as_ref() {
                    *addr_ptr = fb.buffer().as_ptr() as u64;
                } else {
                    ctx.r9 = SYS_ERR;
                }
            }
        }

        SYS_FB_SWAP => {
            ctx.r9 = SYS_ERR;
        }

        SYS_FORK => {
            let current = SCHEDULER.lock().current;
            let new_pid = process::fork(current, ctx);
            crate::debug!("NEW PID : {:?}", new_pid);
            if let Some(pid) = new_pid {
                ctx.r9 = 0;
                ctx.syscall_id = pid;
            } else {
                ctx.r9 = SYS_ERR;
            }
        }

        e => {
            println_serial!("{}", e);
        }
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
