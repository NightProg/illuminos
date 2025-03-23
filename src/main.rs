#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

mod drivers;
mod idt;
mod gdt;
use core::{arch::asm, panic::PanicInfo};
pub use drivers::vga::*;
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _start() -> ! {
    gdt::init_gdt();
    idt::init_idt();
    asm!("int 0x1");

    loop {}
}


#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    println!("{}", _info);
    loop {}
}
