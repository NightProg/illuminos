#![no_std]
#![no_main]
#![feature(iter_next_chunk)]
#![feature(const_trait_impl)]
#![feature(const_default)]
#![feature(generic_const_items)]
#![feature(abi_x86_interrupt)]
#![allow(unused)]
#![feature(ascii_char)]
#![allow(unsafe_op_in_unsafe_fn)]
#![allow(unused_mut)]
#![allow(const_item_mutation)]
#![allow(static_mut_refs)]

extern crate alloc;

mod allocator;
mod context;
mod debug;
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
mod limine_extra;
mod log;
mod math;
mod sync;
mod syscall;
mod thread;
mod tty;
mod util;

use ::elf::ElfBytes;
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
use graphic::{font::FONT_DEFAULT, framebuffer::RawFrameBuffer};
use illfs::InOutDevice;
use log::set_log_output;
use spin::Mutex;
use x86_64::structures::paging::PageTable;

use crate::allocator::paging::KERNEL_PAGING_MANAGER;
use crate::debug::{StackFrame, addr_to_demangle_name, addr_to_name};
use crate::drivers::apic::{map_apic_regions, set_hhdm_offset};
use crate::drivers::disk::Disk;
use crate::drivers::keyboard::{KEYBOARD_STREAM, KeyboardStream};
use crate::drivers::mouse::MOUSE;
use crate::elf::ElfProcess;
use crate::fs::OpenFlags;
use crate::graphic::Color;
// use crate::graphic::console::{Console, console_task};
use crate::graphic::font::{FONT_UNI2_TERMINUS32x16, Psf1Font, Psf2Font, PsfFont};
use crate::graphic::text_buffer::TextBuffer;
use crate::graphic::vram::VRAM_VIRT_ADDR;
use crate::idt::TICKS;
use crate::io::stdout;
use crate::log::LogOutput;
use crate::thread::process::PROCESSES;
use crate::thread::{SCHEDULER, exit, schedule, yield_now};

use core::fmt::Write;
use core::sync::atomic::Ordering;

use io::serial::SerialPortWriter;
use pc_keyboard::KeyEvent;
use x86_64::registers::model_specific;
use x86_64::{
    PhysAddr, VirtAddr,
    instructions::port::Port,
    structures::paging::{PageTableFlags, PhysFrame, Translate, page_table::PageTableEntry},
};

use limine::request::FramebufferRequest;
use limine::{BaseRevision, RequestsEndMarker, RequestsStartMarker};

#[used]
#[unsafe(link_section = ".requests")]
static BASE_REVISION: BaseRevision = BaseRevision::new();

#[used]
#[unsafe(link_section = ".requests")]
static FRAMEBUFFER_REQUEST: FramebufferRequest = FramebufferRequest::new();

#[used]
#[unsafe(link_section = ".requests")]
static HHDM_REQUEST: limine::request::HhdmRequest = limine::request::HhdmRequest::new();

#[used]
#[unsafe(link_section = ".requests")]
static MEMORY_MAP_REQUEST: limine::request::MemmapRequest = limine::request::MemmapRequest::new();

#[used]
#[unsafe(link_section = ".requests")]
static KERNEL_FILE_REQUEST: limine_extra::LimineKernelFileRequest =
    limine_extra::LimineKernelFileRequest::new();

#[used]
#[unsafe(link_section = ".requests_start_marker")]
static _START_MARKER: RequestsStartMarker = RequestsStartMarker::new();
#[used]
#[unsafe(link_section = ".requests_end_marker")]
static _END_MARKER: RequestsEndMarker = RequestsEndMarker::new();

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
    yield_now();
}

#[unsafe(no_mangle)]
pub fn kmain() -> ! {
    unsafe {
        gdt::init_gdt();
    };
    let hhdm_resp = HHDM_REQUEST.response().expect("HHDM_REQUEST failed");
    let memmap_resp = MEMORY_MAP_REQUEST
        .response()
        .expect("MEMORY_MAP_REQUEST failed");
    let framebuffer = FRAMEBUFFER_REQUEST
        .response()
        .expect("FRAMEBUFFER_REQUEST failed");

    let framebuffers = framebuffer
        .framebuffers_rev1()
        .expect("No framebuffer found (with rev1)"); // TODO: support rev0
    let first_framebuffer = framebuffers.first().expect("No framebuffer found");

    let kernel_res = unsafe {
        KERNEL_FILE_REQUEST
            .res
            .as_ref()
            .expect("No kernel file information")
    };

    let file = unsafe { kernel_res.file.as_ref().unwrap() };
    let data: &'static [u8] = unsafe { &*(file.data() as *const [u8]) };
    debug::KERNEL_ELF_FILE.call_once(|| ElfBytes::minimal_parse(data).unwrap());

    println_serial!("the addr of kernel_main is {:#x}", kmain as usize);

    let mut paging_manager =
        unsafe { allocator::paging::PagingManager::new(hhdm_resp.offset, memmap_resp.entries()) };

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
        GLOBAL_CONTEXT.keyboard_stream.lock().init_capacity();
        set_hhdm_offset(hhdm_resp.offset);

        map_apic_regions();
        idt::init_interrupts();
    }

    let fb_size = first_framebuffer.size() as usize;

    /*KERNEL_PAGING_MANAGER.lock().as_mut().unwrap().map_memory(
        VirtAddr::new(VRAM_VIRT_ADDR),
        fb_size,
        PhysAddr::try_new(first_framebuffer.address().addr() as u64).unwrap(),
        PageTableFlags::PRESENT
            | PageTableFlags::WRITABLE
            | PageTableFlags::NO_CACHE    // bit 4 → PCD=1
            | PageTableFlags::HUGE_PAGE,
    );*/

    syscall::init_syscall();
    allocator::paging::init_pat();

    let mut binding = disk::ata::AtaPio::detect_disks();
    let disks: Vec<Disk> = binding.iter().map(|d| Disk::AtaPio(d.clone())).collect();

    let fb_phys_addr = first_framebuffer.address().addr() as u64 - hhdm_resp.offset;
    let mut framebuffer = unsafe { RawFrameBuffer::from_limine(first_framebuffer, fb_phys_addr) };

    init_global_context(framebuffer);

    GLOBAL_CONTEXT.init_vfs(disks);

    // framebuffer.draw_psf_string(
    //     vec2(0, 0),
    //     "Hello, World",
    //     Color::red(),
    //     &psf_font
    // );

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

    println_serial!(
        "is interupt enabled: {}",
        x86_64::instructions::interrupts::are_enabled()
    );

    let disk1 = GLOBAL_CONTEXT
        .fs
        .lock()
        .open_file("/disk/1", fs::OpenFlags::READ | fs::OpenFlags::WRITE)
        .unwrap();

    let fs = illfs::IllFs::mount(disk1).unwrap();

    GLOBAL_CONTEXT
        .fs
        .lock()
        .mount("/mnt", fs::illfs::IllFS(Arc::new(Mutex::new(fs))));

    let mut shell_file = GLOBAL_CONTEXT
        .fs
        .lock()
        .open_file("/mnt/shell", OpenFlags::READ)
        .unwrap();

    let mut buf = vec![0; shell_file.inode.lock().size() as usize];

    shell_file.read(&mut buf).unwrap();

    let mut elf_file = elf::ElfProcess::new(&buf);

    let rip = elf_file.load().unwrap();

    let process_pid = thread::process::Process::create(
        *elf_file.get_page_table(),
        elf_file.get_vmas(),
        Some(thread::process::ProcessArguments {
            argv: Vec::new(),
            envp: Vec::new(),
        }),
        None,
        elf_file.get_brk_start(),
    );
    let session = GLOBAL_CONTEXT.sessions.write().create_session(process_pid);
    let mut process_lock = PROCESSES.lock();
    let process = process_lock.get_mut(&process_pid).unwrap();
    process.current_session = Some(session);

    process.spawn_user_thread(rip);

    drop(process_lock);

    println!("[INFO] Starting scheduler");
    unsafe {
        // SCHEDULER.lock().add_kernel_task(console_task);
        SCHEDULER.lock().add_kernel_task(task_hello_world);
        SCHEDULER.lock().add_kernel_task(task_greeting);
        SCHEDULER.lock().ready();

        schedule();

        loop {
            x86_64::instructions::hlt();
        }
    }

    /*loop {
        while let Some(event) = unsafe { GLOBAL_CONTEXT.pop_key() } {
            println!("[INFO] Received keyboard event: {:?}", event);
        }
    }*/
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    write!(SerialPortWriter, "Kernel panic: {}\n", info).unwrap();
    let frames = unsafe { StackFrame::get() };
    write!(SerialPortWriter, "Stack trace:\n").unwrap();
    for (i, frame) in frames.iter().enumerate() {
        write!(SerialPortWriter, "  #{:<2} {:#018x}", i, frame).unwrap();
        if let Some(name) = addr_to_demangle_name(*frame) {
            write!(SerialPortWriter, " ({})\n", name);
        } else {
            write!(SerialPortWriter, " (??)\n");
        }
    }

    loop {}
}
