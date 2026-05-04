use core::{
    alloc::{GlobalAlloc, Layout},
    sync::atomic::AtomicU64,
};

use core::mem;
use core::ptr;
use linked_list_allocator::LockedHeap;
use spin::Mutex;
use x86_64::{
    PhysAddr, VirtAddr,
    structures::paging::{
        FrameAllocator, OffsetPageTable, Page, PageTableFlags, Size4KiB, Translate,
    },
};

use crate::println;

use super::paging::{PagingManager, map_page};

pub const KERNEL_HEAP_START: VirtAddr = VirtAddr::new(0xFFFF_C000_0000_0000); // PML4[384] → libre
pub const KERNEL_HEAP_SIZE: u64 = 1024 * 1024 * 100; // 500 Mo

pub const USER_HEAP_START: VirtAddr = VirtAddr::new(0x4444_4444_4444);
pub const USER_HEAP_SIZE: u64 = 1024 * 1024 * 10; // 10 Mo

pub fn init_heap(
    paging_manager: &mut PagingManager,
    heap: &LockedHeap,
    start: VirtAddr,
    size: u64,
    flags: PageTableFlags,
) {
    reserve_memory(
        start,
        size,
        &mut paging_manager.mapper,
        &mut paging_manager.frame_allocator,
        flags,
    );

    crate::println_serial!("Heap start: {:#X}", start.as_u64());
    unsafe {
        heap.lock().init(start.as_u64() as usize, size as usize);
    }
}

pub fn init_kernel_heap(paging_manager: &mut PagingManager) {
    init_heap(
        paging_manager,
        &KERNEL_ALLOCATOR,
        KERNEL_HEAP_START,
        KERNEL_HEAP_SIZE,
        PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
    );
}

pub fn init_user_heap(paging_manager: &mut PagingManager) {
    init_heap(
        paging_manager,
        &USER_ALLOCATOR,
        USER_HEAP_START,
        USER_HEAP_SIZE,
        PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE,
    );
}

#[global_allocator]
pub static KERNEL_ALLOCATOR: LockedHeap = LockedHeap::empty();

pub static USER_ALLOCATOR: LockedHeap = LockedHeap::empty();

pub fn reserve_memory(
    start: VirtAddr,
    size: u64,
    mapper: &mut OffsetPageTable,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
    flags: PageTableFlags,
) {
    let start_page = Page::containing_address(start);
    let end_page = Page::containing_address(start + size - 1u64);

    for page in Page::range(start_page, end_page + 1) {
        let frame = frame_allocator
            .allocate_frame()
            .expect("no frame available");
        map_page(page, frame, mapper, frame_allocator, flags);
    }
}
