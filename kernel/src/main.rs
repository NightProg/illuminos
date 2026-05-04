#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(iter_next_chunk)]
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
mod geo;
mod gpu;
mod graphic;
mod hash;
mod idt;
mod io;
mod libc;
mod log;
mod math;
mod modern_graphic;
mod syscall;
mod thread;
mod util;

use alloc::borrow::ToOwned;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::{boxed::Box, vec, vec::Vec};
use allocator::memory::{init_kernel_heap, init_user_heap};
use ata_x86::{ATA_BLOCK_SIZE, list, read};
use context::{Context, GLOBAL_CONTEXT, init_global_context};
use core::f32::consts::TAU;
use core::mem::{offset_of, swap};
use core::ptr::addr_of;
use core::{
    alloc::{GlobalAlloc, Layout},
    arch::asm,
    error,
    ops::DerefMut,
    panic::PanicInfo,
};
use drivers::disk;
use geo::{Object, rect};
use graphic::framebuffer::{DoubleBuffer, SwapBuffer};
use graphic::{
    font::FONT_DEFAULT,
    framebuffer::{FrameBuffer /*VirtualFrameBuffer*/, RawFrameBuffer},
};
use io::port::{Fd, STDOUT};
use log::set_log_output;
use math::vec2;
use spin::Mutex;
use x86_64::structures::paging::PageTable;

use crate::allocator::paging::KERNEL_PAGING_MANAGER;
use crate::context::app_ready;
use crate::drivers::disk::Disk;
use crate::drivers::keyboard::{KEYBOARD_STREAM, KeyboardStream};
use crate::drivers::mouse::MOUSE;
use crate::graphic::Color;
use crate::graphic::console::{Console, console_task};
use crate::graphic::font::{FONT_UNI2_TERMINUS32x16, Psf1Font, Psf2Font, PsfFont};
use crate::graphic::text_buffer::TextBuffer;
use crate::graphic::vram::VRAM_VIRT_ADDR;
use crate::graphic::window::Window;
use crate::idt::TICKS;
use crate::io::port::new_port;
use crate::io::stdout;
use crate::log::LogOutput;
use crate::thread::{SCHEDULER, exit, gc_task, schedule, yield_now};
use bootloader_api::{
    BootInfo, BootloaderConfig,
    config::Mapping,
    entry_point,
    info::{self, FrameBufferInfo},
};
use core::fmt::Write;
use core::sync::atomic::Ordering;
use idt::{init_idt, init_pic};
use io::serial::SerialPortWriter;
use pc_keyboard::KeyEvent;
use x86_64::registers::model_specific;
use x86_64::{
    PhysAddr, VirtAddr,
    instructions::port::Port,
    structures::paging::{PageTableFlags, PhysFrame, Translate, page_table::PageTableEntry},
};

pub static BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.mappings.framebuffer = Mapping::FixedAddress(VRAM_VIRT_ADDR);
    config.mappings.kernel_stack = Mapping::FixedAddress(0xFFFF_FFFF_8000_0000);
    //config.mappings.physical_memory = Some(Mapping::FixedAddress(0xFFFF_8000_0000_0000));
    config
};

entry_point!(kernel_main, config = &BOOTLOADER_CONFIG);

fn keyboard_handler(event: &KeyEvent) {
    println!("[INFO] Keyboard event: {:?}", event);
}

extern "C" fn task_hello_world() {
    x86_64::instructions::interrupts::enable();
    println!("Task hello world!");
    loop {
        yield_now();
    }
}

extern "C" fn task_greeting() {
    x86_64::instructions::interrupts::enable();
    println!("Task greeting!");
    loop {
        let ticks = TICKS.load(Ordering::SeqCst);
        if ticks % 10 == 0 {
            println!("Task greeting: {}", ticks);
        }

        if ticks % 100 == 0 {
            exit();
        }
        yield_now();
    }
}

pub fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    unsafe {
        gdt::init_gdt();
    };

    println_serial!("the addr of kernel_main is {:#x}", kernel_main as usize);

    let mut paging_manager = unsafe { allocator::paging::PagingManager::new(boot_info) };

    KERNEL_PAGING_MANAGER.lock().insert(paging_manager);

    for (i, entry) in KERNEL_PAGING_MANAGER
        .lock()
        .as_ref()
        .unwrap()
        .mapper
        .level_4_table()
        .iter()
        .enumerate()
    {
        if !entry.is_unused() {
            crate::println_serial!("PML4[{}] = {:#x}", i, entry.addr().as_u64());
        }
    }

    fn print_pml4_entry(phys_offset: VirtAddr, pml4: &PageTable, i: usize) {
        if pml4[i].is_unused() {
            return;
        }
        let pdpt: &PageTable = unsafe { &*((phys_offset + pml4[i].addr().as_u64()).as_ptr()) };
        for j in 0..512 {
            if !pdpt[j].is_unused() {
                let virt = (i as u64) << 39 | (j as u64) << 30;
                crate::println_serial!(
                    "  [{}/{}] virt={:#x} phys={:#x}",
                    i,
                    j,
                    virt,
                    pdpt[j].addr().as_u64()
                );
            }
        }
    }

    let phys_offset = KERNEL_PAGING_MANAGER
        .lock()
        .as_ref()
        .unwrap()
        .mapper
        .phys_offset();
    {
        let kernel_paging_manager = KERNEL_PAGING_MANAGER.lock();
        let kernel_pml4 = kernel_paging_manager
            .as_ref()
            .unwrap()
            .mapper
            .level_4_table();

        print_pml4_entry(phys_offset, kernel_pml4, 0);
        print_pml4_entry(phys_offset, kernel_pml4, 2);
        print_pml4_entry(phys_offset, kernel_pml4, 3);
    }

    {
        let mut paging_manager_lock = KERNEL_PAGING_MANAGER.lock();
        if let Some(paging_manager) = paging_manager_lock.as_mut() {
            init_kernel_heap(paging_manager);
            init_user_heap(paging_manager);
        }
    }

    println!("[INFO] Init kernel heap");
    unsafe {
        KEYBOARD_STREAM.lock().init_capacity();
        KEYBOARD_STREAM.lock().add_callback(keyboard_handler);
        GLOBAL_CONTEXT.keyboard_stream.init_capacity();
        idt::init_interrupts();
    }
    println!("[INFO] Init keyboard stream");

    syscall::init_syscall();
    allocator::paging::init_pat();

    let mut binding = disk::ata::AtaPio::detect_disks();
    let disks: Vec<Disk> = binding.iter().map(|d| Disk::AtaPio(d.clone())).collect();

    let framebuffer_info = boot_info.framebuffer.as_ref().unwrap().info();
    let mut framebuffer =
        unsafe { RawFrameBuffer::create_from_raw_addr(VRAM_VIRT_ADDR, framebuffer_info) };

    init_global_context(framebuffer);

    unsafe {
        let mut paging_manager_lock = KERNEL_PAGING_MANAGER.lock();
        if let Some(paging_manager) = paging_manager_lock.as_mut() {
            paging_manager.update_flags(
                VirtAddr::new(VRAM_VIRT_ADDR),
                PageTableFlags::PRESENT
                | PageTableFlags::WRITABLE
                | PageTableFlags::NO_CACHE    // bit 4 → PCD=1
                | PageTableFlags::HUGE_PAGE,
            );
        }
    }

    unsafe { GLOBAL_CONTEXT.init_vfs(disks) }

    let fd = new_port(
        || Vec::new(),
        |w| {
            println!("{}", core::str::from_utf8(w).unwrap());
        },
    );
    STDOUT.set(fd.0);

    framebuffer.clear_screen(Color::black());
    // framebuffer.draw_psf_string(
    //     vec2(0, 0),
    //     "Hello, World",
    //     Color::red(),
    //     &psf_font
    // );

    let fd = unsafe {
        GLOBAL_CONTEXT
            .create_port_text_buffer(Color::red(), Color::black())
            .unwrap()
    };

    STDOUT.set(fd.0);

    /*let mut console = unsafe {
        Console::new(
            GLOBAL_CONTEXT.framebuffer.as_mut().unwrap().deref_mut(),
            &*FONT_DEFAULT,
        )
    };

    loop {
        while let Some(event) = unsafe { GLOBAL_CONTEXT.pop_key() } {
            println!("[INFO] Received keyboard event: {:?}", event);
            console.handle_key_event(event);
            stdout::flush(&mut console);
        }
    }*/

    #[derive(Debug)]
    #[repr(C, packed)]
    struct Idtr {
        limit: u16,
        base: u64,
    }
    let mut idtr = Idtr { limit: 0, base: 0 };
    unsafe {
        core::arch::asm!("sidt [{}]", in(reg) &mut idtr, options(nostack));
        println_serial!("IDTR base: {:#x}, limit: {:#x}", { idtr.base }, {
            idtr.limit
        });
    }
    println_serial!("IDT static addr: {:#x}", &*idt::IDT as *const _ as u64);
    println_serial!("IDTR base: {:#x}", { idtr.base });
    println_serial!("IDTR: {:?}\n", idtr);

    crate::println!("[INFO] Starting scheduler");
    unsafe {
        SCHEDULER.lock().add_kernel_task(console_task);
        SCHEDULER.lock().add_kernel_task(gc_task);
        SCHEDULER.lock().add_kernel_task(task_hello_world);
        SCHEDULER.lock().ready();

        schedule();

        loop {
            x86_64::instructions::hlt();
        }
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    write!(SerialPortWriter, "Kernel panic: {}\n", info).unwrap();
    loop {}
}
