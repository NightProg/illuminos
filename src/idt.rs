use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};
use lazy_static::lazy_static;
use spin::Mutex;

use crate::println;

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.divide_error.set_handler_fn(divide_by_zero_handler);
        idt.debug.set_handler_fn(debug_handler);
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt
    };
}

pub fn init_idt() {

    

    unsafe { IDT.load(); } // Charge l’IDT
}

// Handlers d'interruptions
extern "x86-interrupt" fn divide_by_zero_handler(stack_frame: InterruptStackFrame) {
    println!("EXCEPTION: Division by zero\n{:#?}", stack_frame);
    loop {} // Bloquer pour éviter de continuer l'exécution
}

extern "x86-interrupt" fn debug_handler(stack_frame: InterruptStackFrame) {
    println!("EXCEPTION: Debug\n{:#?}", stack_frame);
    loop {}
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    println!("EXCEPTION: Breakpoint\n{:#?}", stack_frame);
}

// Fonction d'initialisation de l'IDT appelée au boot
pub fn load_idt() {
    init_idt();
}
