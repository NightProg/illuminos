#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(iter_next_chunk)]
#![feature(naked_functions)]
#![allow(static_mut_refs)]
#![allow(unused)]
#![feature(ascii_char)]
#![allow(unsafe_op_in_unsafe_fn)]
#![allow(unused_mut)]
#![allow(const_item_mutation)]

extern crate alloc;

mod allocator;
mod context;
mod drivers;
mod elf;
mod fs;
mod gdt;
mod graphic;
mod idt;
mod io;
mod log;
mod math;
mod syscall;
mod util;
mod libc;
mod thread;

use alloc::sync::Arc;
use alloc::{boxed::Box, vec, vec::Vec};
use allocator::memory::init_heap;
use ata_x86::{ATA_BLOCK_SIZE, list, read};
use context::{Context, GLOBAL_CONTEXT, init_global_context};
use core::f32::consts::TAU;
use core::mem::offset_of;
use core::{
    alloc::{GlobalAlloc, Layout},
    arch::asm,
    error,
    ops::DerefMut,
    panic::PanicInfo,
};
use drivers::{
    disk,
    keyboard::{KeyEvent, set_keyboard_handler},
};
use fs::fat32::FAT32;
use graphic::text::TextBuffer;
use graphic::{
    GraphicMode,
    font::FONT_DEFAULT,
    framebuffer::{FrameBuffer, VirtualFrameBuffer},
    text::TextEdit,
    windows::Window,
};
use io::port::{Fd, STDIO};
use log::set_log_output;
use spin::Mutex;

use core::fmt::Write;

use crate::context::app_ready;
use crate::graphic::app::APP_MANAGER;
use crate::graphic::console::Console;
use crate::graphic::windows::WINDOW_MANAGER;
use crate::log::LogOutput;
use bootloader_api::{
    BootInfo, BootloaderConfig,
    config::Mapping,
    entry_point,
    info::{self, FrameBufferInfo},
};
use idt::{init_idt, init_pic};
use io::serial::SerialPortWriter;
use x86_64::{
    VirtAddr,
    instructions::port::Port,
    structures::paging::{PageTableFlags, Translate},
};

pub static BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.mappings.framebuffer = Mapping::FixedAddress(graphic::vram::VRAM_VIRT_ADDR);
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
    let mut fd = Fd(STDIO.fd());

    write!(fd,"{}", key_event.key.to_string().unwrap_or('?'));
}

pub fn check_pic_mask() {
    let mask: u8;
    unsafe {
        core::arch::asm!("in al, 0x21", out("al") mask);
    }
    info!("PIC1 Mask: {:#X}", mask);
}

pub unsafe fn unmask_pic() {
    unsafe {
        Port::new(0x21).write(0xFCu8);
        Port::new(0xA1).write(0xFFu8); // Unmask IRQ2 (série)
    }
}



pub fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    gdt::init_gdt();
    init_idt();
    init_pic();
    unsafe { unmask_pic() };

    x86_64::instructions::interrupts::enable();

    set_keyboard_handler(keyboard_handler);

    let mut paging_manager = unsafe { allocator::paging::PagingManager::new(boot_info) };
    init_heap(
        &mut paging_manager,
        PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
    );
    syscall::init_syscall();

    let framebuffer = unsafe {
        FrameBuffer::create_from_raw_addr(
            graphic::vram::VRAM_VIRT_ADDR,
            boot_info.framebuffer.as_ref().unwrap().info(),
        )
    };

    init_global_context(framebuffer);

    GLOBAL_CONTEXT
        .lock()
        .deref_mut()
        .framebuffer
        .as_mut()
        .map(|x| x.clear_screen(0x000000));

    //
    // let window = WINDOW_MANAGER.lock().new_window(800, 600, 0, 0);
    // WINDOW_MANAGER.lock().sync(
    //     GLOBAL_CONTEXT.lock().framebuffer.as_mut().unwrap()
    // );
    // info!("Window manager initialized!");
    // // set_log_output(log::LogOutput::TextBuffer(window));
    //
    // info!("Kernel started");
    //
    //
    //
    //
    // app_ready();

    WINDOW_MANAGER.lock().init();
    info!(
        "Window manager initialized: {:?}",
        GLOBAL_CONTEXT.lock().framebuffer
    );

    let mut win_log = WINDOW_MANAGER.lock().new_window(600, 800, 0, 0);
    let mut text_buffer = TextBuffer::create(1200 - 600, 800, 600, 0);
    set_log_output(LogOutput::TextBuffer(win_log));

    let mut stdio = io::port::Ports::new_port(
        || Vec::new(),
        move |data| {
            text_buffer.write_string(core::str::from_utf8(data).unwrap());
        },
    );


    for region in boot_info.memory_regions.into_iter() {
        print_serial!("region start: {:x} ", region.start);
        print_serial!("region end: {:x} ", region.end);
        println_serial!("region kind: {:?}", region.kind);

    }

    io::port::STDIO.set(stdio.fd());
    let mut disk = drivers::disk::ata::AtaPio::detect_disks();
    let mut last = disk.last().unwrap().clone();
    let mut ext2 = fs::ext2::Ext2FS::from_disk(&mut last).unwrap();
    use fs::FileSystem;
    info!("read /hello file");
    let hello_exe = ext2.read(fs::Path::new("/hello")).unwrap();
    info!("Execute hello");
    let mut elf = elf::ProgLoader::from_bytes(&hello_exe).unwrap();
    //write!(stdio, "elf: {:#?}", elf);
    elf.execute(&mut paging_manager);




    loop {}
}


#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    write!(SerialPortWriter, "Kernel panic: {}\n", info).unwrap();
    loop {}
}
