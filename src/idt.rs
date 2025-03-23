use pic8259::ChainedPics;
use spin::Mutex;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};
use lazy_static::lazy_static;

use crate::{drivers::keyboard::{Keyboard, KEYBOARD}, print};


pub static PICS: Mutex<ChainedPics> = Mutex::new(unsafe {
    ChainedPics::new(0x20, 0x28) // PIC1 à 0x20, PIC2 à 0x28
});



lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.divide_error.set_handler_fn(divide_by_zero_handler);
        idt.debug.set_handler_fn(debug_handler);
        idt.non_maskable_interrupt.set_handler_fn(non_maskable_interrupt_handle);
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt.overflow.set_handler_fn(overflow_handler);
        idt.bound_range_exceeded.set_handler_fn(bound_range_exceeded_handler);
        idt.invalid_opcode.set_handler_fn(invalid_opcode_handler);
        idt.device_not_available.set_handler_fn(device_not_available_handler);
        idt.double_fault.set_handler_fn(double_fault_handler);
        idt[32].set_handler_fn(timer_handler);
        idt[33].set_handler_fn(keyboard_handler);
        idt
    };
}

pub fn init_idt() {
    IDT.load();
}

// Handlers d'interruptions
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

extern "x86-interrupt" fn double_fault_handler(stack_frame: InterruptStackFrame, _error_code: u64) -> ! {
    panic!("EXCEPTION: Double fault\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn keyboard_handler(_stack_frame: InterruptStackFrame) {
    let scancode = KEYBOARD.lock().read_key();
    if let Some(key) = scancode {
        KEYBOARD.lock().handle_key(key);
    }
    unsafe { PICS.lock().notify_end_of_interrupt(33) };
}


extern "x86-interrupt" fn timer_handler(_stack_frame: InterruptStackFrame) {
    unsafe { PICS.lock().notify_end_of_interrupt(32) };
}

pub fn init_pic() {
    unsafe { PICS.lock().initialize() };
}