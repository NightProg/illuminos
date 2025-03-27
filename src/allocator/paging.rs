use bootloader::{bootinfo::{MemoryMap, MemoryRegionType}, BootInfo};
use x86_64::{structures::paging::{FrameAllocator, Mapper, OffsetPageTable, Page, PageTable, PageTableFlags, PhysFrame, Size4KiB, Translate}, PhysAddr, VirtAddr};




pub unsafe fn init_paging(boot_info: &'static BootInfo) -> OffsetPageTable<'static> {
    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    
    // Récupérer l'adresse physique de la PML4 et la convertir en VirtAddr
    let level_4_table = unsafe {
        active_level_4_table(phys_mem_offset)
    };
    
    unsafe {
        OffsetPageTable::new(level_4_table, phys_mem_offset)
    }
}

unsafe fn active_level_4_table(phys_mem_offset: VirtAddr) -> &'static mut PageTable {
    let frame = x86_64::registers::control::Cr3::read().0;
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



pub struct KernelFrameAllocator {
    memory_map: &'static MemoryMap,
    next: usize,
}

impl KernelFrameAllocator {
    pub unsafe fn init(memory_map: &'static MemoryMap) -> Self {
        KernelFrameAllocator {
            memory_map,
            next: 0,
        }
    }

    fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> {
        let regions = self.memory_map.iter();
        let usable_regions = regions.filter(|r| r.region_type == MemoryRegionType::Usable);
        let addr_ranges = usable_regions.map(|r| r.range.start_addr()..r.range.end_addr());
        let frame_addresses = addr_ranges.flat_map(|r| r.step_by(4096));
        frame_addresses.map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))
    }
}


pub struct PagingManager {
    pub mapper: OffsetPageTable<'static>,
    pub frame_allocator: KernelFrameAllocator,
}

impl PagingManager {
    pub unsafe fn new(boot_info: &'static BootInfo) -> Self {
        let mapper = unsafe { init_paging(boot_info) };
        let frame_allocator = unsafe { KernelFrameAllocator::init(&boot_info.memory_map) };
        PagingManager {
            mapper,
            frame_allocator,
        }
    }

    pub fn map_memory(
        &mut self,
        virt_addr: VirtAddr,
        phys_addr: PhysAddr,
        flags: PageTableFlags,
    ) {
        let page = Page::containing_address(virt_addr);
        let frame = PhysFrame::containing_address(phys_addr);
        map_page(page, frame, &mut self.mapper, &mut self.frame_allocator, flags);
    }
    
}

unsafe impl FrameAllocator<Size4KiB> for KernelFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        let frame = self.usable_frames().nth(self.next);
        self.next += 1;
        frame
    }
}


