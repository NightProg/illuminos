use core::{alloc::{GlobalAlloc, Layout}, sync::atomic::AtomicU64};



use core::mem;
use core::ptr;
use linked_list_allocator::LockedHeap;
use spin::Mutex;
use x86_64::{structures::paging::{FrameAllocator, OffsetPageTable, Page, PageTableFlags, Size4KiB, Translate}, PhysAddr, VirtAddr};

use crate::{println};

use super::paging::{map_page, PagingManager};



pub const HEAP_START: VirtAddr = VirtAddr::new(0x4444_4444_0000);
pub const HEAP_SIZE: u64 = 1024 * 1024 * 10; // 100 Mo

pub fn init_heap(paging_manager: &mut PagingManager, flags: PageTableFlags) {
    reserve_memory(HEAP_START, HEAP_SIZE, &mut paging_manager.mapper, &mut paging_manager.frame_allocator, flags);

    unsafe {
        ALLOCATOR.lock().init(HEAP_START.as_u64() as usize, HEAP_SIZE as usize);
    }
}


#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();


// pub static mut ALLOCATOR: Option<AllocatorBase> = None;

// pub struct Heap<'a> {
//     start: VirtAddr,
//     atomic_start: AtomicU64,
//     mapper: &'a mut OffsetPageTable<'a>,
//     size: u64,
// }

// impl<'a> Heap<'a> {

//     pub fn new(start: VirtAddr, size: u64, mapper: &'a mut OffsetPageTable<'a>) -> Self {
//         Heap {
//             start,
//             size,
//             mapper,
//             atomic_start: AtomicU64::new(start.as_u64()),
//         }
//     }


//     fn reserve_memory(
//         &mut self,
//         frame_allocator: &mut impl FrameAllocator<Size4KiB>,
//         flags: PageTableFlags
//     ) {
//         reserve_memory(self.start, self.size, self.mapper, frame_allocator, flags);
//     }
// }
// const MIN_BLOCK_SIZE: usize = mem::size_of::<usize>();

// struct FreeBlock {
//     size: usize,
//     next: *mut FreeBlock,
// }

// pub fn init_allocator(allocator: AllocatorBase, paging_manager: &mut PagingManager, flags: PageTableFlags) {
//     unsafe {
//         ALLOCATOR = Some(allocator);
//         println!("Allocator initialized");
//         ALLOCATOR.as_mut().unwrap().reserve_memory(paging_manager, flags);
//     }
// }

// pub struct AllocatorBase {
//     head: Mutex<*mut FreeBlock>,
//     start: VirtAddr,
//     end: VirtAddr,
// }

// impl AllocatorBase {
//     pub fn new(start: VirtAddr, size: u64) -> Self {
//         Self {
//             head: Mutex::new(ptr::null_mut()),
//             start: start,
//             end: start + size,
//         }
//     }

//     pub fn reserve_memory(
//         &mut self,
//         paging_manager: &mut PagingManager,
//         flags: PageTableFlags
//     ) {
//         reserve_memory(self.start, self.end.as_u64() - self.start.as_u64(), &mut paging_manager.mapper, &mut paging_manager.frame_allocator, flags);
//     }


//     pub fn align_up(&self, addr: usize, align: usize) -> usize {
//         let remainder = addr % align;
//         if remainder == 0 {
//             addr
//         } else {
//             addr + (align - remainder)
//         }
//     }

//     unsafe fn add_free_block(&self, ptr: *mut u8, size: usize) {
//         if size < MIN_BLOCK_SIZE {
//             return; // Trop petit pour être utile
//         }

//         let block = ptr as *mut FreeBlock;
//         unsafe {
//             (*block).size = size;
//             (*block).next = *self.head.lock();
//         }
//         *self.head.lock() = block;
//     }

//     pub fn is_aligned(addr: usize, align: usize) -> bool {
//         addr % align == 0
//     }

//     unsafe fn alloc(&mut self, layout: Layout) -> *mut u8 {
//         let mut head = self.head.lock();
//         let mut prev: *mut FreeBlock = ptr::null_mut();
//         let mut current = *head;

//         let align = layout.align();
        

//         let aligned_size = layout.size().max(MIN_BLOCK_SIZE);
        
//         while !current.is_null() {
//             if (*current).size >= aligned_size {
//                 if !prev.is_null() {
//                     (*prev).next = (*current).next;
//                 } else {
//                     *head = (*current).next;
//                 }
//                 return current as *mut u8;
//             }
//             prev = current;
//             current = (*current).next;
//         }

//         // Pas de bloc libre, on essaie d'en créer un nouveau
//         let new_block = self.start.as_mut_ptr();
//         self.start += self.align_up(aligned_size, align) as u64;

//         if self.start > self.end {
//             return ptr::null_mut(); // Mémoire épuisée
//         }

//         new_block
//     }

//     unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
//         unsafe {
//             self.add_free_block(ptr, layout.size());
//         }
//     }
// }

// pub struct AllocImpl;

// unsafe impl GlobalAlloc for AllocImpl {
//     unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
//         let mut allocator = ALLOCATOR.as_mut().expect("Allocator not initialized");
//         let a = unsafe {
//             allocator.alloc(layout)
//         };

//         info!("Allocating {} memory at {:p}", layout.size(), a);

//         a
//     }

//     unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
//         let allocator = ALLOCATOR.as_ref().expect("Allocator not initialized");
//         info!("Deallocating memory at {:p}", ptr);
//         unsafe {
//             allocator.dealloc(ptr, layout);
//         }
//     }
// }

// #[global_allocator]
// pub static GLOBAL_ALLOCATOR: AllocImpl = AllocImpl;


pub fn reserve_memory(
    start: VirtAddr,
    size: u64,
    mapper: &mut OffsetPageTable,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
    flags: PageTableFlags
) {
    let start_page = Page::containing_address(start);
    let end_page = Page::containing_address(start + size - 1u64);

    for page in Page::range(start_page, end_page + 1) {
        let frame = frame_allocator.allocate_frame().expect("no frame available");
        map_page(page, frame, mapper, frame_allocator, flags);
    }
}
