use core::sync::atomic::AtomicU64;

use lazy_static::lazy_static;
use x86_64::structures::gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector};
use x86_64::structures::tss::TaskStateSegment;
use x86_64::VirtAddr;

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;
pub const STACK_SIZE: usize = 4096 * 7;
pub static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

pub const KERNEL_STACK_SIZE: usize = 4096 * 7;
pub static mut KERNEL_STACK: [u8; KERNEL_STACK_SIZE] = [0; KERNEL_STACK_SIZE];

static mut TSS: TaskStateSegment = TaskStateSegment::new();

pub unsafe fn init_tss() {
    let stack_start = VirtAddr::from_ptr(&raw const STACK);
    let stack_end = stack_start + STACK_SIZE as u64;
    TSS.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = stack_end;
    let kernel_stack_start = VirtAddr::from_ptr(&raw const KERNEL_STACK);
    let kernel_stack_end = kernel_stack_start + KERNEL_STACK_SIZE as u64;
    TSS.privilege_stack_table[0] = kernel_stack_end;
}

pub static mut GDT: (GlobalDescriptorTable, GdtSelectors) = (
    GlobalDescriptorTable::new(),
    GdtSelectors {
        code_selector: SegmentSelector(0),
        data_selector: SegmentSelector(0),
        user_data_selector: SegmentSelector(0),
        user_code_selector: SegmentSelector(0),
        tss_selector: SegmentSelector(0),
    },
);

pub struct GdtSelectors {
    pub code_selector: SegmentSelector,
    pub data_selector: SegmentSelector,
    pub user_data_selector: SegmentSelector,
    pub user_code_selector: SegmentSelector,
    pub tss_selector: SegmentSelector,
}

pub unsafe fn init_gdt() {
    use x86_64::instructions::segmentation::{Segment, CS, DS};
    use x86_64::instructions::tables::load_tss;
    init_tss();

    let code_selector = GDT.0.append(Descriptor::kernel_code_segment());
    let data_selector = GDT.0.append(Descriptor::kernel_data_segment());
    let user_data_selector = GDT.0.append(Descriptor::user_data_segment());
    let user_code_selector = GDT.0.append(Descriptor::user_code_segment());
    let tss_selector = GDT.0.append(Descriptor::tss_segment(unsafe { &TSS }));
    GDT.1.code_selector = code_selector;
    GDT.1.data_selector = data_selector;
    GDT.1.user_code_selector = user_code_selector;
    GDT.1.user_data_selector = user_data_selector;
    GDT.1.tss_selector = tss_selector;

    GDT.0.load();
    unsafe {
        CS::set_reg(GDT.1.code_selector);
        DS::set_reg(GDT.1.data_selector);
        load_tss(GDT.1.tss_selector);
    }
}

pub unsafe fn set_tss_rsp0(stack_top: VirtAddr) {
    TSS.privilege_stack_table[0] = stack_top;
}
