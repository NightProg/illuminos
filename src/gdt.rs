use core::mem::size_of;
use x86_64::registers::segmentation::CS;
use x86_64::structures::gdt::{Descriptor, GlobalDescriptorTable};
use x86_64::VirtAddr;
use lazy_static::lazy_static;

use crate::println;

// La GDT doit être statique en mémoire
lazy_static! {
    static ref GDT: (GlobalDescriptorTable, Selectors) = {
        let mut gdt = GlobalDescriptorTable::new();
        
        // Ajouter un segment de code
        let code_selector = gdt.append(Descriptor::kernel_code_segment());
        (gdt, Selectors { code_selector })
    };
}

struct Selectors {
    code_selector: x86_64::structures::gdt::SegmentSelector,
}

pub fn init_gdt() {
    use x86_64::instructions::segmentation::Segment;
    
    GDT.0.load();

    unsafe { CS::set_reg(GDT.1.code_selector) };


    println!("GDT initialized!");
}