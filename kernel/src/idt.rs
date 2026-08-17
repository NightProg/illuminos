use alloc::string::ToString;
use core::{
    ops::{Deref, DerefMut},
    sync::atomic::AtomicU64,
};

use alloc::vec;
use lazy_static::lazy_static;
use pc_keyboard::DecodedKey;
use pic8259::ChainedPics;
use spin::Mutex;
use x86_64::{
    PhysAddr, PrivilegeLevel, VirtAddr,
    instructions::port::Port,
    structures::{
        idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode},
        paging::{
            FrameAllocator, FrameDeallocator, Mapper, Page, PageTableFlags, PhysFrame, Size4KiB,
            Translate, mapper::TranslateResult,
        },
    },
};

use crate::{
    allocator::{
        process_paging::COW_FLAG,
        vma::{MapFlags, ProtFlags},
    },
    dbg,
    drivers::{
        apic::{
            VECTOR_KEYBOARD, VECTOR_TIMER, hhdm_offset, init_apic_timer, init_local_apic,
            ioapic_route, lapic_eoi, lapic_read,
        },
        pic::disable_pic,
    },
    thread::{
        self,
        process::{PROCESSES, current_process},
        signal::deliver_signals_irq,
    },
};

use crate::allocator::paging::KERNEL_PAGING_MANAGER;
use crate::drivers::keyboard::{KEYBOARD, KEYBOARD_STREAM};
use crate::drivers::mouse::{MOUSE, MOUSE_POS};
use crate::gdt::DOUBLE_FAULT_IST_INDEX;
use crate::graphic::Color;
use crate::io::stdout;
use crate::thread::signal::Signal;
use crate::thread::{SCHEDULER, exit, schedule};
use crate::{
    context::GLOBAL_CONTEXT, info, io::serial::SerialPortWriter, print, print_serial, println,
    println_serial,
};
use crate::debug::addr_to_demangle_name;
use crate::thread::sleeper::SLEEPER_QUEUE;

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = 0x20,
    Keyboard = 0x21,
}

impl InterruptIndex {
    fn as_u8(self) -> u8 {
        self as u8
    }

    fn as_usize(self) -> usize {
        usize::from(self.as_u8())
    }
}

pub static TICKS: AtomicU64 = AtomicU64::new(0);
pub static SLEEPING: AtomicU64 = AtomicU64::new(0);

lazy_static! {
    pub static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.divide_error.set_handler_fn(divide_by_zero_handler);
        idt.debug.set_handler_fn(debug_handler);
        idt.non_maskable_interrupt
            .set_handler_fn(non_maskable_interrupt_handle);
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt.overflow.set_handler_fn(overflow_handler);
        idt.bound_range_exceeded
            .set_handler_fn(bound_range_exceeded_handler);
        idt.invalid_opcode.set_handler_fn(invalid_opcode_handler);
        idt.device_not_available
            .set_handler_fn(device_not_available_handler);
        unsafe {
            idt.double_fault
                .set_handler_fn(double_fault_handler)
                .set_stack_index(DOUBLE_FAULT_IST_INDEX);
        }
        idt.page_fault.set_handler_fn(page_fault_handler);
        idt[InterruptIndex::Timer.as_u8()].set_handler_fn(timer_handler);
        idt[InterruptIndex::Keyboard.as_u8()].set_handler_fn(keyboard_handler);
        idt[0xFF].set_handler_fn(spurious_handler);
        idt
    };
}

pub fn init_idt() {
    IDT.load();
}

extern "x86-interrupt" fn divide_by_zero_handler(stack_frame: InterruptStackFrame) {
    panic!("EXCEPTION: Divide by zero\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn debug_handler(stack_frame: InterruptStackFrame) {
    panic!("EXCEPTION: Debug\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn non_maskable_interrupt_handle(stack_frame: InterruptStackFrame) {
    panic!("EXCEPTION: Non-maskable interrupt\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    panic!("EXCEPTION: Breakpoint\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn overflow_handler(stack_frame: InterruptStackFrame) {
    panic!("EXCEPTION: Overflow\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn bound_range_exceeded_handler(stack_frame: InterruptStackFrame) {
    panic!("EXCEPTION: Bound range exceeded\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn invalid_opcode_handler(stack_frame: InterruptStackFrame) {
    panic!("EXCEPTION: Invalid opcode\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn device_not_available_handler(stack_frame: InterruptStackFrame) {
    panic!("EXCEPTION: Device not available\n{:#?}", stack_frame);
}

pub extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    _error_code: u64,
) -> ! {
    panic!("EXCEPTION: Double fault\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    let mut stack_frame = stack_frame;
    use x86_64::registers::control::Cr2;
    let faulting_address = Cr2::read().unwrap_or_else(|_| {
        panic!(
            "Failed to read CR2 register for page fault at {:#?}",
            stack_frame
        )
    });

    println_serial!("PAGE FAULT, {:?}", stack_frame);
    let from_user = stack_frame.code_segment.rpl() == PrivilegeLevel::Ring3
        || ((faulting_address.as_u64() >> 47) & 1 != 1);
    let caused_by_write = error_code.contains(PageFaultErrorCode::CAUSED_BY_WRITE);
    let protection_violation = error_code.contains(PageFaultErrorCode::PROTECTION_VIOLATION);
    println!(
        "Page fault at address {:?}, error code: {:?}, from user: {}, caused by write: {}, protection violation: {}",
        faulting_address, error_code, from_user, caused_by_write, protection_violation
    );
    if from_user {
        println!("from user");
        let scheduler_lock = SCHEDULER.lock();
        let mut processes_lock = PROCESSES.lock();
        println!("locked");
        let current_task = scheduler_lock.current_task().unwrap_or_else(|| {
            panic!(
                "No current task in scheduler during page fault at {:#?}",
                stack_frame
            )
        });
        let current_process = processes_lock.get_mut(&current_task.parent).unwrap();
        let is_cow = unsafe {
            current_process.pml4_table.with_mapper(|mapper| {
                if let TranslateResult::Mapped { flags, .. } = mapper.translate(faulting_address) {
                    flags.contains(COW_FLAG)
                } else {
                    false
                }
            })
        };
        let demand_paging = unsafe {
            current_process.pml4_table.with_mapper(|mapper| {
                !matches!(
                    mapper.translate(faulting_address),
                    TranslateResult::Mapped { .. }
                )
            })
        };
        let faulting_address = faulting_address.align_down(4096u64);
        let vma = current_process.vmas.iter().find(|v| {
            faulting_address.as_u64() >= v.start as u64 && faulting_address.as_u64() <= v.end as u64
        });

        println_serial!(
            "Faulting address: {:?}, error code: {:?}, caused by write: {}, protection violation: {} demand_paging: {}",
            faulting_address,
            error_code,
            caused_by_write,
            protection_violation,
            demand_paging
        );

        match vma {
            None => {
                println_serial!(
                    "Segmentation fault(1): address {:?} is not mapped\n{:#?} error_code: {:?}",
                    faulting_address,
                    stack_frame,
                    error_code
                );
                current_process.write_file(1, b"Segmentation fault: invalid memory access\n");
                drop(scheduler_lock);
                drop(processes_lock);
                exit(139);
            }
            Some(vma)
                if caused_by_write
                    && protection_violation
                    && !vma.prot.contains(ProtFlags::WRITE) =>
            {
                current_process.write_file(1, b"Segmentation fault: write access violation\n");
                exit(139);
            }
            Some(vma) if demand_paging => {
                println!(
                    "Demand paging for address {:?} vma: {:?}",
                    faulting_address, vma
                );
                let mut kernel_paging_manager = KERNEL_PAGING_MANAGER.lock();
                println!("Locked kernel paging manager");
                let kernel_frame_allocator =
                    &mut kernel_paging_manager.as_mut().unwrap().frame_allocator;
                let data = if vma.flags.contains(MapFlags::ANONYMOUS) {
                    let size = vma.end - vma.start;
                    vec![0; size as usize]
                } else {
                    let file = vma.file.as_ref().expect("expect a file");
                    let mut buf = vec![0; file.lock().size() as usize];
                    file.lock().read_at(0, &mut buf);
                    buf
                };

                current_process.pml4_table.map_elf_segment(
                    VirtAddr::new(vma.start),
                    &data,
                    data.len(),
                    vma.prot.to_page_table_flags(),
                    kernel_frame_allocator,
                );

                println!(
                    "Mapped ELF segment at address {:?} vma: {:?}",
                    faulting_address, vma
                );
                return;
            }

            Some(vma)
                if caused_by_write
                    && protection_violation
                    && (is_cow || vma.flags.contains(MapFlags::PRIVATE)) =>
            unsafe {
                let page = Page::containing_address(faulting_address);
                let mut frame_allocator = KERNEL_PAGING_MANAGER
                    .lock()
                    .as_ref()
                    .unwrap()
                    .frame_allocator
                    .clone();
                use x86_64::structures::paging::FrameAllocator;
                let new_frame = frame_allocator.allocate_frame().unwrap();

                let src = faulting_address.align_down(4096u64).as_ptr::<u8>();
                let phys_offset = current_process.pml4_table.phys_offset;
                let dst = (phys_offset + new_frame.start_address().as_u64()).as_mut_ptr::<u8>();

                core::ptr::copy_nonoverlapping(src, dst, 4096);

                current_process.pml4_table.with_mapper(|mapper| {
                    mapper.unmap(page).unwrap().1.flush();
                    frame_allocator.deallocate_frame(PhysFrame::containing_address(PhysAddr::new(
                        faulting_address.as_u64(),
                    )));
                    crate::allocator::paging::map_page(
                        page,
                        new_frame,
                        mapper,
                        &mut frame_allocator,
                        PageTableFlags::PRESENT
                            | PageTableFlags::USER_ACCESSIBLE
                            | PageTableFlags::WRITABLE,
                    );
                });
                return;
            },
            Some(vma)
                if caused_by_write
                    && protection_violation
                    && vma.flags.contains(MapFlags::SHARED) =>
            unsafe {
                current_process.pml4_table.with_mapper(|mapper| {
                    if let TranslateResult::Mapped { mut flags, .. } =
                        mapper.translate(faulting_address)
                    {
                        flags.insert(PageTableFlags::WRITABLE);
                        mapper
                            .update_flags(
                                Page::<Size4KiB>::containing_address(faulting_address),
                                flags,
                            )
                            .unwrap()
                            .flush();
                    } else {
                        panic!(
                            "Page fault at {:?} is not mapped\n{:#?} error_code: {:?}",
                            faulting_address, stack_frame, error_code
                        );
                    }
                });

                return;
            },
            Some(vma) if vma.flags.contains(MapFlags::ANONYMOUS) && !protection_violation => {
                println_serial!(
                    "Requesting new frame for anonymous page at address {:?} vma: {:?}",
                    faulting_address,
                    vma
                );
                let new_frame = {
                    KERNEL_PAGING_MANAGER
                        .lock()
                        .as_mut()
                        .unwrap()
                        .allocate_frame()
                        .unwrap()
                };
                unsafe {
                    current_process.pml4_table.with_mapper(|mapper| {
                        println_serial!("{:?}", mapper.translate(faulting_address));
                        crate::allocator::paging::map_page(
                            Page::containing_address(faulting_address),
                            new_frame,
                            mapper,
                            &mut KERNEL_PAGING_MANAGER
                                .lock()
                                .as_mut()
                                .unwrap()
                                .frame_allocator
                                .clone(),
                            dbg!(vma.prot.to_page_table_flags()),
                        );
                    });
                }

                return;
            }
            _ => {
                println_serial!("DEBUG");
                current_process.write_file(1, b"Segmentation fault: invalid memory access\n");
                exit(139);
            }
        }
    }
    panic!(
        "EXCEPTION: Page fault at {:?}\n{:#?} error_code: {:?}, location {:X}, ({})",
        Cr2::read(),
        stack_frame,
        error_code,
        stack_frame.instruction_pointer.as_u64(),
        if let Some(symbol) = addr_to_demangle_name(stack_frame.instruction_pointer.as_u64()) {
            symbol
        } else {
            "unknown".to_string()
        }
    );
}

extern "x86-interrupt" fn keyboard_handler(_stack_frame: InterruptStackFrame) {
    use x86_64::instructions::port::Port;
    let mut port = Port::new(0x60);

    let scancode: u8 = unsafe { port.read() };
    let Some(mut keyboard) = KEYBOARD.try_lock() else {
        println_serial!("Failed to acquire keyboard stream lock in keyboard handler");
        return;
    };
    let event = keyboard.add_byte(scancode);

    if let Ok(Some(key_event)) = event {
        drop(keyboard);
        GLOBAL_CONTEXT.add_key(key_event);
    }

    unsafe {
        lapic_eoi();
    }
}

extern "x86-interrupt" fn timer_handler(stack_frame: InterruptStackFrame) {
    let mut stack_frame = stack_frame;

    TICKS.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    unsafe {
        lapic_eoi();
    };


    SLEEPER_QUEUE.lock().tick(TICKS.load(core::sync::atomic::Ordering::Relaxed));
    let Some(scheduler) = SCHEDULER.try_lock() else {
        println_serial!("Failed to acquire scheduler lock in timer handler");
        return;
    };
    if !scheduler.is_ready() {
        return;
    }
    drop(scheduler);
    schedule();
}

extern "x86-interrupt" fn spurious_handler(_stack_frame: InterruptStackFrame) {
    crate::println_serial!("Spurious interrupt");
}

pub fn disable_apic() {
    unsafe {
        let mut apic_base_msr = x86_64::registers::model_specific::Msr::new(0x1B);
        let mut val = apic_base_msr.read();
        val &= !(1 << 11); // clear the APIC global enable bit
        apic_base_msr.write(val);
    }
}
pub fn init_interrupts() {
    unsafe {
        disable_pic();
        println_serial!("PIC disabled");

        init_local_apic();

        // Vérifie que le SVR est bien écrit
        let svr = lapic_read(0x0F0);
        println_serial!("SVR = {:#x} (attendu: {:#x})", svr, (1 << 8) | 0xFF);

        let id = lapic_read(0x020);
        println_serial!("Local APIC ID = {:#x}", id);

        ioapic_route(0, VECTOR_TIMER, 0, false);
        ioapic_route(1, VECTOR_KEYBOARD, 0, false);

        let base = (hhdm_offset() + 0xFEC0_0000) as *mut u32;
        core::ptr::write_volatile(base, 0x10);
        let irq0_lo = core::ptr::read_volatile(base.byte_add(0x10));
        println_serial!(
            "IOAPIC IRQ0 redtbl lo = {:#x} (vecteur attendu: {:#x})",
            irq0_lo,
            VECTOR_TIMER
        );

        init_apic_timer(100, VECTOR_TIMER);

        // Vérifie le LVT timer et l'initial count
        let lvt = lapic_read(0x320);
        let ic = lapic_read(0x380);
        let cc = lapic_read(0x390);
        println_serial!("LVT_TIMER = {:#x}, IC = {}, CC = {}", lvt, ic, cc);
    }

    IDT.load();
    x86_64::instructions::interrupts::enable();
    println_serial!("Interrupts enabled");
}
