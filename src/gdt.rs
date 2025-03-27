use core::mem::size_of;
use x86_64::registers::segmentation::CS;
use x86_64::structures::gdt::{Descriptor, GlobalDescriptorTable};
use x86_64::VirtAddr;
use lazy_static::lazy_static;

use crate::{info, println};

use x86_64::structures::tss::TaskStateSegment;

// Taille de la pile pour les interruptions
const STACK_SIZE: usize = 4096 * 5;

#[repr(align(16))] // Alignement nécessaire pour la pile
struct AlignedStack([u8; STACK_SIZE]);

// Pile dédiée aux double faults (statique pour être accessible)
static mut DOUBLE_FAULT_STACK: AlignedStack = AlignedStack([0; STACK_SIZE]);

lazy_static! {
    static ref TSS: TaskStateSegment = {
        let mut tss = TaskStateSegment::new();

        // Définition de la pile pour les doubles fautes
        tss.interrupt_stack_table[0] = VirtAddr::new(unsafe {
            &DOUBLE_FAULT_STACK as *const _ as u64 + STACK_SIZE as u64
        });

        tss
    };
}


// La GDT doit être statique en mémoire
lazy_static! {
    static ref GDT: (GlobalDescriptorTable, Selectors) = {
        let mut gdt = GlobalDescriptorTable::new();
        
        // Ajouter un segment de code
        let code_selector = gdt.append(Descriptor::kernel_code_segment());
        let tss_selector = gdt.append(Descriptor::tss_segment(&TSS));
        (gdt, Selectors { code_selector, tss_selector })
    };
}

struct Selectors {
    code_selector: x86_64::structures::gdt::SegmentSelector,
    tss_selector: x86_64::structures::gdt::SegmentSelector,
}

pub fn init_gdt() {
    use x86_64::instructions::segmentation::Segment;
    
    GDT.0.load();

    unsafe { CS::set_reg(GDT.1.code_selector) };
    unsafe { x86_64::instructions::tables::load_tss(GDT.1.tss_selector) };


    info!("GDT initialized");
}