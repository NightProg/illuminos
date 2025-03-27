#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![allow(static_mut_refs)]
#![feature(ascii_char)]

extern crate alloc;

mod drivers;
mod idt;
mod gdt;
mod util;
mod allocator;
mod log;
mod io;
mod graphic;
use core::{alloc::{GlobalAlloc, Layout}, arch::asm, panic::PanicInfo};
use alloc::{boxed::Box, vec::Vec};
use allocator::memory::init_heap;
use drivers::{keyboard::set_keyboard_handler, vga};
pub use drivers::vga::*;

use bootloader::BootInfo;
use x86_64::{structures::paging::{PageTableFlags, Translate}, VirtAddr};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _start(boot_info: &'static BootInfo) -> ! {
    gdt::init_gdt();
    idt::init_idt(); 
    idt::init_pic();

    x86_64::instructions::interrupts::enable();

    let mut paging_manager = unsafe { allocator::paging::PagingManager::new(boot_info) };
    init_heap(&mut paging_manager, PageTableFlags::PRESENT | PageTableFlags::WRITABLE);


    loop {
    }

}


#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    vga::set_color(Color::Red, Color::Black);
    println!("{}", _info);
    loop {}
}
