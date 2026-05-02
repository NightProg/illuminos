// memory/process_paging.rs

use crate::allocator::paging::{KERNEL_CR3, KERNEL_CR3_FRAME, KERNEL_PAGING_MANAGER};

use super::paging::{PagingManager, map_page};
use x86_64::{
    PhysAddr, VirtAddr,
    registers::control::{Cr3, Cr3Flags},
    structures::paging::{
        FrameAllocator, OffsetPageTable, Page, PageTable, PageTableFlags, PhysFrame, Size4KiB,
    },
};

#[derive(Debug, Clone, Copy)]
pub struct ProcessPageTable {
    pub pml4_frame: PhysFrame,
    phys_offset: VirtAddr,
}

impl ProcessPageTable {
    pub fn kernel() -> Self {
        let mut paging_manager_lock = KERNEL_PAGING_MANAGER.lock();
        let paging_manager = paging_manager_lock
            .as_mut()
            .expect("KERNEL_PAGING_MANAGER not initialized before creating ProcessPageTable");
        let phys_offset = paging_manager.mapper.phys_offset();

        let kernel_pml4_frame = KERNEL_CR3_FRAME.lock().unwrap();
        let kernel_pml4_virt = phys_offset + kernel_pml4_frame.start_address().as_u64();
        let kernel_pml4: &PageTable = unsafe { &*(kernel_pml4_virt.as_ptr()) };

        Self {
            pml4_frame: kernel_pml4_frame,
            phys_offset,
        }
    }

    pub fn new() -> Self {
        let mut paging_manager_lock = KERNEL_PAGING_MANAGER.lock();
        let mut paging_manager = paging_manager_lock
            .as_mut()
            .expect("KERNEL_PAGING_MANAGER not initialized before creating ProcessPageTable");
        let phys_offset = paging_manager.mapper.phys_offset();

        let pml4_frame = paging_manager
            .frame_allocator
            .allocate_frame()
            .expect("plus de frames pour PML4");

        let pml4_virt = phys_offset + pml4_frame.start_address().as_u64();
        let new_pml4: &mut PageTable = unsafe { &mut *(pml4_virt.as_mut_ptr()) };

        new_pml4.zero();

        let kernel_pml4_frame = KERNEL_CR3_FRAME.lock().unwrap();
        let kernel_pml4_virt = phys_offset + kernel_pml4_frame.start_address().as_u64();
        let kernel_pml4: &PageTable = unsafe { &*(kernel_pml4_virt.as_ptr()) };

        for i in 0..512 {
            new_pml4[i] = kernel_pml4[i].clone();
        }

        if !kernel_pml4[0].is_unused() {
            let pdpt_frame = paging_manager
                .frame_allocator
                .allocate_frame()
                .expect("plus de frames pour PDPT");
            let pdpt_virt = phys_offset + pdpt_frame.start_address().as_u64();
            let new_pdpt: &mut PageTable = unsafe { &mut *(pdpt_virt.as_mut_ptr()) };
            new_pdpt.zero();

            let kernel_pdpt_phys = kernel_pml4[0].addr();
            let kernel_pdpt_virt = phys_offset + kernel_pdpt_phys.as_u64();
            let kernel_pdpt: &PageTable = unsafe { &*(kernel_pdpt_virt.as_ptr()) };

            for i in 0..512 {
                new_pdpt[i] = kernel_pdpt[i].clone();
            }

            new_pdpt[0].set_unused();

            let flags = kernel_pml4[0].flags();
            new_pml4[0].set_addr(pdpt_frame.start_address(), flags);
        }

        new_pml4[255].set_unused();

        Self {
            pml4_frame,
            phys_offset,
        }
    }

    unsafe fn with_mapper<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut OffsetPageTable) -> R,
    {
        let pml4_virt = self.phys_offset + self.pml4_frame.start_address().as_u64();
        let pml4: &mut PageTable = &mut *(pml4_virt.as_mut_ptr());
        let mut mapper = OffsetPageTable::new(pml4, self.phys_offset);
        f(&mut mapper)
    }

    pub fn map(
        &mut self,
        page: Page<Size4KiB>,
        frame: PhysFrame,
        flags: PageTableFlags,
        frame_allocator: &mut impl FrameAllocator<Size4KiB>,
    ) {
        unsafe {
            self.with_mapper(|mapper| {
                map_page(page, frame, mapper, frame_allocator, flags);
            });
        }
    }

    pub fn map_range(
        &mut self,
        virt_start: VirtAddr,
        size: u64,
        flags: PageTableFlags,
        frame_allocator: &mut impl FrameAllocator<Size4KiB>,
    ) {
        let start_page = Page::containing_address(virt_start);
        let end_page = Page::containing_address(virt_start + size - 1u64);

        for page in Page::range_inclusive(start_page, end_page) {
            self.map(
                page,
                frame_allocator.allocate_frame().unwrap(),
                flags,
                frame_allocator,
            );
        }
    }

    pub fn map_elf_segment(
        &mut self,
        virt_start: VirtAddr,
        data: &[u8],
        mem_size: usize,
        flags: PageTableFlags,
        frame_allocator: &mut impl FrameAllocator<Size4KiB>,
    ) {
        let pages = (mem_size + 4095) / 4096;

        for i in 0..pages {
            let frame = frame_allocator.allocate_frame().expect("plus de frames");

            let frame_virt = self.phys_offset + frame.start_address().as_u64();
            let dst =
                unsafe { core::slice::from_raw_parts_mut(frame_virt.as_mut_ptr::<u8>(), 4096) };

            dst.fill(0);

            let file_offset = i * 4096;
            if file_offset < data.len() {
                let copy_len = (data.len() - file_offset).min(4096);
                dst[..copy_len].copy_from_slice(&data[file_offset..file_offset + copy_len]);
            }

            let page = Page::containing_address(virt_start + (i * 4096) as u64);
            self.map(page, frame, flags, frame_allocator);
        }
    }

    pub fn cr3_value(&self) -> (PhysFrame, Cr3Flags) {
        (self.pml4_frame, Cr3Flags::empty())
    }
}
