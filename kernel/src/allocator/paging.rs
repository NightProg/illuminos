use alloc::vec::Vec;
use bootloader_api::{
    BootInfo,
    info::{MemoryRegion, MemoryRegionKind, MemoryRegions},
};
use x86_64::{
    PhysAddr, VirtAddr,
    structures::paging::{
        FrameAllocator, FrameDeallocator, Mapper, OffsetPageTable, Page, PageSize, PageTable,
        PageTableFlags, PhysFrame, Size4KiB, Translate, mapper::UnmapError,
    },
};

use crate::info;
use crate::math;
use spin::Mutex;
use x86_64::registers::model_specific::Msr;

pub static KERNEL_CR3: Mutex<Option<PageTable>> = Mutex::new(None);
pub static KERNEL_CR3_FRAME: Mutex<Option<PhysFrame>> = Mutex::new(None);

pub static KERNEL_PAGING_MANAGER: Mutex<Option<PagingManager>> = Mutex::new(None);

pub fn get_physical_memory_offset(boot_info: &BootInfo) -> VirtAddr {
    let memory_regions = boot_info.memory_regions.iter();
    let usable_regions = memory_regions.filter(|r| r.kind == MemoryRegionKind::Usable);
    let addr_ranges = usable_regions.map(|r| r.start..r.end);
    let frame_addresses = addr_ranges.flat_map(|r| r.step_by(4096));
    let mut min_addr = u64::MAX;
    for addr in frame_addresses {
        if addr < min_addr {
            min_addr = addr;
        }
    }
    if min_addr == u64::MAX {
        panic!("No usable memory regions found");
    }
    let phys_mem_offset = PhysAddr::new(min_addr);

    VirtAddr::new(phys_mem_offset.as_u64())
}

pub unsafe fn init_paging(boot_info: &BootInfo) -> OffsetPageTable<'static> {
    let phys_mem_offset = VirtAddr::new(
        boot_info
            .physical_memory_offset
            .into_option()
            .expect("No physical memory offset found"),
    );
    let level_4_table = unsafe { active_level_4_table(phys_mem_offset) };

    KERNEL_CR3.lock().insert(level_4_table.clone());
    unsafe { OffsetPageTable::new(level_4_table, phys_mem_offset) }
}

unsafe fn active_level_4_table(phys_mem_offset: VirtAddr) -> &'static mut PageTable {
    let frame = x86_64::registers::control::Cr3::read().0;
    KERNEL_CR3_FRAME.lock().insert(frame);
    let phys = frame.start_address();
    let virt = phys_mem_offset + phys.as_u64();
    &mut *(virt.as_mut_ptr())
}

pub fn map_page(
    page: Page,
    frame: PhysFrame,
    mapper: &mut OffsetPageTable,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
    flags: PageTableFlags,
) {
    use x86_64::structures::paging::Mapper;
    let map_result = unsafe { mapper.map_to(page, frame, flags, frame_allocator) };
    map_result.expect("map_to failed").flush();
}

#[derive(Clone, Debug)]
pub struct KernelFrameAllocator<'a> {
    memory_map: &'a [MemoryRegion],
    freeed_frame: Vec<PhysFrame>,
    use_free_frame: bool,
    next: usize,
}

impl<'a> KernelFrameAllocator<'a> {
    pub unsafe fn init(boot_info: &'a BootInfo) -> Self {
        KernelFrameAllocator {
            memory_map: &boot_info.memory_regions,
            freeed_frame: Vec::new(),
            use_free_frame: false,
            next: 0,
        }
    }

    fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> {
        let regions = self.memory_map.iter();
        let usable_regions = regions.filter(|r| r.kind == MemoryRegionKind::Usable);
        let addr_ranges = usable_regions.map(|r| r.start..r.end);
        let frame_addresses = addr_ranges.flat_map(|r| r.step_by(4096));
        frame_addresses.map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))
    }

    pub fn allocate_frames(&mut self, size: usize) -> Option<Vec<PhysFrame>> {
        let size_frame = Size4KiB::SIZE;

        let use_frame = math::ceil((size as u64 / size_frame) as f64) as u64;
        let mut frames = Vec::new();
        for i in 0..use_frame {
            frames.push(self.allocate_frame()?);
        }

        return Some(frames);
    }
}

pub struct PagingManager<'p> {
    pub mapper: OffsetPageTable<'p>,
    pub frame_allocator: KernelFrameAllocator<'p>,
}

impl<'p> PagingManager<'p> {
    pub unsafe fn new(boot_info: &'p BootInfo) -> Self {
        let mut boot_info = boot_info;
        let mapper = unsafe { init_paging(boot_info) };
        let frame_allocator = unsafe { KernelFrameAllocator::init(&mut boot_info) };
        PagingManager {
            mapper,
            frame_allocator,
        }
    }

    pub unsafe fn update_flags(&mut self, addr: VirtAddr, flags: PageTableFlags) {
        self.mapper
            .update_flags(Page::<Size4KiB>::containing_address(addr), flags)
            .expect("update flags failed")
            .flush();
    }
    pub fn allocate_frame(&mut self) -> Option<PhysFrame> {
        return self.frame_allocator.allocate_frame();
    }

    pub fn deallocate_frame(&mut self, phys_frame: PhysFrame) {
        unsafe {
            self.frame_allocator.deallocate_frame(phys_frame);
        }
    }

    pub fn allocate_frames(&mut self, size: usize) -> Option<Vec<PhysFrame>> {
        return self.frame_allocator.allocate_frames(size);
    }

    pub fn map_page(&mut self, page: Page, frame: PhysFrame, flags: PageTableFlags) {
        map_page(
            page,
            frame,
            &mut self.mapper,
            &mut self.frame_allocator,
            flags,
        );
    }

    pub fn unmap_page(&mut self, page: Page) -> Result<PhysFrame, UnmapError> {
        let (frame, page_flush) = self.mapper.unmap(page)?;
        page_flush.flush();

        Ok(frame)
    }

    pub fn map_memory(
        &mut self,
        virt_addr: VirtAddr,
        size: usize,
        phys_addr: PhysAddr,
        flags: PageTableFlags,
    ) {
        let page = Page::containing_address(virt_addr);
        let frame = PhysFrame::containing_address(phys_addr);
        let page_count = ((size + 4095) / 4096) as u64;
        let pages = (0..page_count).map(|i| page + i);
        for page in pages {
            self.map_page(page, frame, flags);
        }
    }
}

unsafe impl<'a> FrameAllocator<Size4KiB> for KernelFrameAllocator<'a> {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        if self.use_free_frame {
            let frame = self.freeed_frame.get(self.next);
            self.next += 1;
            return frame.copied();
        } else {
            let frame = self.usable_frames().nth(self.next);

            if frame.is_none() {
                self.use_free_frame = true;
                return self.allocate_frame();
            }
            self.next += 1;
            frame
        }
    }
}

impl<'a> FrameDeallocator<Size4KiB> for KernelFrameAllocator<'a> {
    unsafe fn deallocate_frame(&mut self, frame: PhysFrame<Size4KiB>) {
        self.freeed_frame.push(frame);
    }
}

pub fn init_pat() {
    let new_pat: u64 = 0x0001_0406_0001_0406;
    unsafe {
        Msr::new(0x277).write(new_pat);
    }
}
