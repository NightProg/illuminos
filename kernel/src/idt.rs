use core::{
    ops::{Deref, DerefMut},
    sync::atomic::AtomicU64,
};

use lazy_static::lazy_static;
use pic8259::ChainedPics;
use spin::Mutex;
use x86_64::{
    PrivilegeLevel,
    instructions::port::Port,
    structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode},
};

use crate::thread::{self, signal::deliver_signals_irq};

use crate::allocator::paging::KERNEL_PAGING_MANAGER;
use crate::drivers::keyboard::{KEYBOARD, KEYBOARD_STREAM};
use crate::drivers::mouse::{MOUSE, MOUSE_POS};
use crate::gdt::DOUBLE_FAULT_IST_INDEX;
use crate::graphic::Color;
use crate::graphic::framebuffer::FrameBuffer;
use crate::io::stdout;
use crate::thread::signal::Signal;
use crate::thread::{SCHEDULER, schedule, task_exit};
use crate::{
    context::GLOBAL_CONTEXT, info, io::serial::SerialPortWriter, print, print_serial, println,
    println_serial,
};

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;
const PIT_CH0: u16 = 0x40;
const PIT_CMD: u16 = 0x43;
const PIT_FREQ: u32 = 1193182;

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
    Keyboard,
}

impl InterruptIndex {
    fn as_u8(self) -> u8 {
        self as u8
    }

    fn as_usize(self) -> usize {
        usize::from(self.as_u8())
    }
}

pub static PICS: Mutex<ChainedPics> =
    Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });
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
    let faulting_address = Cr2::read();
    let from_user = stack_frame.code_segment.rpl() == PrivilegeLevel::Ring3;
    if from_user {
        crate::io::stdout::write(b"Segmentation fault\n");
        crate::thread::task_exit();
        unreachable!();
    }
    panic!(
        "EXCEPTION: Page fault at {:?}\n{:#?} error_code: {:?}",
        Cr2::read(),
        stack_frame,
        error_code
    );
}

extern "x86-interrupt" fn keyboard_handler(_stack_frame: InterruptStackFrame) {
    use x86_64::instructions::port::Port;

    let mut port = Port::new(0x60);

    let scancode: u8 = unsafe { port.read() };
    let event = unsafe { KEYBOARD.add_byte(scancode) };
    if let Ok(Some(key_event)) = event {
        unsafe {
            GLOBAL_CONTEXT.add_key(key_event);
        }
    }

    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }
}

extern "x86-interrupt" fn timer_handler(stack_frame: InterruptStackFrame) {
    let mut stack_frame = stack_frame;
    TICKS.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    let from_user = stack_frame.code_segment.rpl() == x86_64::PrivilegeLevel::Ring3;

    if from_user {
        deliver_signals_irq(&mut stack_frame);
    }
    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Timer.as_u8())
    };
    schedule();
}

fn init_pit() {
    let frequency: u32 = 100; // 100 Hz
    let divisor = PIT_FREQ / frequency;
    unsafe {
        let mut cmd = Port::new(PIT_CMD);
        cmd.write(0x34u8);
        let mut ch0 = Port::new(PIT_CH0);
        ch0.write((divisor & 0xFF) as u8); // low byte
        ch0.write((divisor >> 8) as u8); // high byte
    }
}
pub fn init_pic() {
    unsafe { PICS.lock().initialize() };
}

pub fn init_interrupts() {
    init_idt();
    init_pic();
    init_pit();

    unsafe {
        PICS.lock().write_masks(0x00, 0x00);
    }

    unsafe {
        x86_64::instructions::interrupts::enable();
    }
}
