#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![allow(static_mut_refs)]
#![feature(ascii_char)]


mod drivers;
mod idt;
mod gdt;
mod util;
use core::{arch::asm, panic::PanicInfo};
use drivers::keyboard::set_keyboard_handler;
pub use drivers::vga::*;
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _start() -> ! {
    gdt::init_gdt();
    set_keyboard_handler(|key| {

        println!("{:?}", key);
    });
    idt::init_idt();
    idt::init_pic();

    x86_64::instructions::interrupts::enable();
    loop {
    }

}


#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    println!("{}", _info);
    loop {}
}
