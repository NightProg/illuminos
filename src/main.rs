#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(generic_const_exprs)]
#![feature(panic_can_unwind)]
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
mod context;
mod fs;
use core::{alloc::{GlobalAlloc, Layout}, arch::asm, ops::DerefMut, panic::{ PanicInfo}};
use alloc::{boxed::Box, vec::{Vec}, vec};
use alloc::sync::Arc;
use allocator::memory::init_heap;
use ata_x86::{list, read, ATA_BLOCK_SIZE};
use context::{init_global_context, Context, GLOBAL_CONTEXT};
use drivers::{disk, keyboard::{set_keyboard_handler, KeyEvent}};
use fs::fat32::FAT32;
use graphic::{text::TextEdit, font::FONT_DEFAULT, framebuffer::{FrameBuffer, VirtualFrameBuffer}, windows::Window, GraphicMode};
use log::set_log_output;
use spin::Mutex;


use core::fmt::Write;

use bootloader_api::{config::Mapping, entry_point, info::FrameBufferInfo, BootInfo, BootloaderConfig};
use idt::{init_idt, init_pic};
use io::serial::SerialPortWriter;
use x86_64::{instructions::port::Port, structures::paging::{PageTableFlags, Translate}, VirtAddr};
use crate::context::app_ready;
use crate::graphic::app::APP_MANAGER;
use crate::graphic::console::Console;
use crate::graphic::windows::WINDOW_MANAGER;

pub static BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.mappings.physical_memory = Some(Mapping::Dynamic);
    config
};

entry_point!(kernel_main, config = &BOOTLOADER_CONFIG);

pub fn test_keyboard_polling() {
    loop {
        let mut status: u8;
        unsafe {
            core::arch::asm!("in al, 0x64", out("al") status);
        }

        if status & 1 != 0 {
            let mut scancode: u8;
            unsafe {
                core::arch::asm!("in al, 0x60", out("al") scancode);
            }
            info!("Scancode reçu (polling) : {:#X}", scancode);
        }
    }
}

fn keyboard_handler(key_event: KeyEvent, context: &Mutex<Context>) {
    info!("Keyboard event: {:?}", key_event);
    if !context.lock().is_app_initialized() {
        return;
    }
    info!("Keyboa");
    APP_MANAGER.lock().handle_keyboard_event(key_event);
}

pub fn check_pic_mask() {
    let mask: u8;
    unsafe {
        core::arch::asm!("in al, 0x21", out("al") mask); // Lire masque du PIC1
    }
    info!("PIC1 Mask: {:#X}", mask);
}

pub unsafe  fn unmask_pic() {
    unsafe {
        Port::new(0x21).write(0xFDu8); // Unmask IRQ1 (clavier)
        Port::new(0xA1).write(0xFFu8); // Unmask IRQ2 (série)
    }
}

pub fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    init_pic();
    unsafe { unmask_pic() };
    init_idt();
    set_keyboard_handler(keyboard_handler);

    x86_64::instructions::interrupts::enable();

    let mut paging_manager = unsafe { allocator::paging::PagingManager::new(boot_info) };
    init_heap(&mut paging_manager, PageTableFlags::PRESENT | PageTableFlags::WRITABLE);

    let framebuffer = boot_info.framebuffer.take();
    let framebuffer = FrameBuffer::new(framebuffer.unwrap());

    init_global_context(framebuffer);



    GLOBAL_CONTEXT.lock().deref_mut().framebuffer.as_mut().map(|x| x.clear_screen(0x000000));


    let window = WINDOW_MANAGER.lock().new_window(800, 600, 0, 0);
    // set_log_output(log::LogOutput::TextBuffer(window));

    info!("Kernel started");


    app_ready();
    WINDOW_MANAGER.lock().init();





    // // wait for 1 second
    // let mut i = 0;
    // while i < 100000000 {
    //     i += 1;
    // }
    // window.move_to(150, 150);
    // window.sync();


    // for i in 0..10 {
    //     let lba = i;
    //     for n in 0..10 {
    //         let block = n;
    //         read(0, lba, block ,&mut buffer);
    //         info!("ATA Drive: {:?}", &buffer[0..16]);
    //     }
    // }

    // set_keyboard_handler(keyboard_handler);


    loop {}

}


#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    
    write!(SerialPortWriter, "Kernel panic: {}\n", info).unwrap();
    loop {}
}
