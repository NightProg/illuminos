use x86_64::registers::model_specific::{Efer, EferFlags, Msr};
use core::arch::global_asm;
use crate::gdt::GDT;
use crate::println_serial;
use crate::io::port::Fd;

#[repr(C)]
#[derive(Debug)]
pub struct SyscallCtx {
    pub syscall_id: u64, // rax
    pub rip: u64,        // rcx
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
}

pub fn init_syscall() {
    let mut efer = Efer::read();

    efer.insert(EferFlags::SYSTEM_CALL_EXTENSIONS);
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
    let ctx = unsafe {
        &*sys_ctx
    };

    match ctx.syscall_id {
        2 => {
            let fd = ctx.rdi;
            let buf = ctx.rsi as *const u8;
            let buf_len = ctx.rdx;

            let slice = unsafe {
                core::slice::from_raw_parts(buf, buf_len as usize)
            };

            let fd = Fd(fd as usize);

            fd.write(slice);
        }

        e => {
            println_serial!("{}", e);
        }

    }
}

global_asm!(
    include_str!(
        concat!(
            env!("CARGO_MANIFEST_DIR"), "/asm/syscall.asm"
        )
    )
);


unsafe extern "C" {
    fn sys_handler();
}
