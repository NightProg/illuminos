/*use core::mem::size_of;
use x86_64::registers::segmentation::CS;
use x86_64::structures::gdt::{Descriptor, GlobalDescriptorTable};
use lazy_static::lazy_static;
use crate::allocator::paging::PagingManager;
use x86_64::{VirtAddr, PhysAddr, structures::paging::{Page, PageTableFlags, Mapper, FrameAllocator, Size4KiB, Translate}};


use crate::{println};

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
        let user_selector = gdt.append(Descriptor::UserSegment(0x23));
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



}


pub fn map_tss_and_df_stack(
    pm: &mut PagingManager
) {
    use x86_64::structures::paging::mapper::MapToError;

    // Mapping de la TSS
    let tss_virt = VirtAddr::from_ptr(&*TSS);
    let tss_page: Page = Page::containing_address(tss_virt);

    if pm.mapper.translate_addr(tss_virt).is_none() {
        let frame = pm.frame_allocator
            .allocate_frame()
            .expect("no physical frames available for TSS");
        unsafe {
            pm.mapper
                .map_to(
                    tss_page,
                    frame,
                    PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
                    &mut pm.frame_allocator,
                )
                .expect("TSS mapping failed")
                .flush();
        }
    }

    // Mapping de la stack de double fault
    let df_stack_ptr = unsafe { &DOUBLE_FAULT_STACK as *const _ as u64 };
    let df_stack_start = VirtAddr::new(df_stack_ptr);
    let df_stack_end = df_stack_start + STACK_SIZE as u64;

    let start_page = Page::containing_address(df_stack_start);
    let end_page = Page::containing_address(df_stack_end - 1u64);

    for page in Page::range_inclusive(start_page, end_page) {
        if pm.mapper.translate_addr(page.start_address()).is_some() {
            continue; // déjà mappée
        }

        let frame = pm.frame_allocator
            .allocate_frame()
            .expect("no physical frames available for DF stack");
        unsafe {
            pm.mapper
                .map_to(
                    page,
                    frame,
                    PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
                    &mut pm.frame_allocator,
                )
                .expect("DF stack mapping failed")
                .flush();
        }
    }
}*/

use lazy_static::lazy_static;
use x86_64::VirtAddr;
use x86_64::structures::gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector};
use x86_64::structures::tss::TaskStateSegment;

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

lazy_static! {
    static ref TSS: TaskStateSegment = {
        let mut tss = TaskStateSegment::new();
        const STACK_SIZE: usize = 4096 * 5;
        static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];
        let stack_start = VirtAddr::from_ptr(&raw const STACK);
        let stack_end = stack_start + STACK_SIZE as u64;
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
            stack_end
        };


        tss.privilege_stack_table[0] = stack_end;

        tss
    };


}

lazy_static! {
    pub static ref GDT: (GlobalDescriptorTable, GdtSelectors) = {
        let mut gdt = GlobalDescriptorTable::new();
        let code_selector = gdt.append(Descriptor::kernel_code_segment());
        let data_selector = gdt.append(Descriptor::kernel_data_segment());
        let user_code_selector = gdt.append(Descriptor::user_code_segment());
        let user_data_selector = gdt.append(Descriptor::user_data_segment());
        let tss_selector = gdt.append(Descriptor::tss_segment(&TSS));
        (
            gdt,
            GdtSelectors {
                code_selector,
                data_selector,
                user_data_selector,
                user_code_selector,
                tss_selector,
            },
        )
    };
}

pub struct GdtSelectors {
    pub code_selector: SegmentSelector,
    pub data_selector: SegmentSelector,
    pub user_data_selector: SegmentSelector,
    pub user_code_selector: SegmentSelector,
    pub tss_selector: SegmentSelector,
}

pub fn init_gdt() {
    use x86_64::instructions::segmentation::{CS, DS, Segment};
    use x86_64::instructions::tables::load_tss;

    GDT.0.load();
    unsafe {
        CS::set_reg(GDT.1.code_selector);
        DS::set_reg(GDT.1.data_selector);
        load_tss(GDT.1.tss_selector);
    }
}
