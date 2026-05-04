// memory/process_paging.rs

use super::paging::{PagingManager, map_page};
use crate::allocator::paging::{KERNEL_CR3, KERNEL_CR3_FRAME, KERNEL_PAGING_MANAGER};
use crate::println_serial;
use x86_64::structures::paging::{FrameDeallocator, Mapper};
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

    pub fn copy_from(other: ProcessPageTable) -> Self {
        let mut paging_manager_lock = KERNEL_PAGING_MANAGER.lock();
        let mut paging_manager = paging_manager_lock
            .as_mut()
            .expect("KERNEL_PAGING_MANAGER not initialized before creating ProcessPageTable");
        let phys_offset = paging_manager.mapper.phys_offset();

        let pml4_frame = paging_manager.allocate_frame().expect("no more frame ");

        let pml4_virt = phys_offset + pml4_frame.start_address().as_u64();
        let new_pml4: &mut PageTable = unsafe { &mut *(pml4_virt.as_mut_ptr()) };

        new_pml4.zero();

        let other_pml4_virt = phys_offset + other.pml4_frame.start_address().as_u64();
        let other_pml4: &PageTable = unsafe { &*(other_pml4_virt.as_ptr()) };

        for i in 0..512 {
            if other_pml4[i].is_unused() {
                continue;
            }
            if other_pml4[i].flags().contains(PageTableFlags::HUGE_PAGE) {
                println_serial!("Skipping huge page in PML4[{}]", i);
                continue;
            }
            if KERNEL_CR3.lock().clone().unwrap()[i]
                .flags()
                .contains(PageTableFlags::PRESENT)
            {
                new_pml4[i] = other_pml4[i].clone();
                continue;
            }

            let pdpt_phys = other_pml4[i].addr().as_u64();
            let other_pdpt: &PageTable = unsafe { &*((phys_offset + pdpt_phys).as_ptr()) };
            let new_pdpt_frame = paging_manager.allocate_frame().expect("no more frame");
            let new_pdpt_virt = phys_offset + new_pdpt_frame.start_address().as_u64();
            let new_pdpt: &mut PageTable = unsafe { &mut *(new_pdpt_virt.as_mut_ptr()) };
            new_pdpt.zero();
            for j in 0..512 {
                if other_pdpt[j].is_unused() {
                    continue;
                }
                if other_pdpt[j].flags().contains(PageTableFlags::HUGE_PAGE) {
                    println_serial!("Skipping huge page in PDPT[{}]", j);
                    continue;
                }

                let pd_phys = other_pdpt[j].addr().as_u64();
                let other_pd: &PageTable = unsafe { &*((phys_offset + pd_phys).as_ptr()) };
                let new_pd_frame = paging_manager.allocate_frame().expect("no more frame");
                let new_pd_virt = phys_offset + new_pd_frame.start_address().as_u64();
                let new_pd: &mut PageTable = unsafe { &mut *(new_pd_virt.as_mut_ptr()) };
                new_pd.zero();
                for k in 0..512 {
                    if other_pd[k].is_unused() {
                        continue;
                    }
                    if other_pd[k].flags().contains(PageTableFlags::HUGE_PAGE) {
                        println_serial!("Skipping huge page in PD[{}]", k);
                        continue;
                    }

                    let pt_phys = other_pd[k].addr().as_u64();
                    let other_pt: &PageTable = unsafe { &*((phys_offset + pt_phys).as_ptr()) };
                    let new_pt_frame = paging_manager.allocate_frame().expect("no more frame");
                    let new_pt_virt = phys_offset + new_pt_frame.start_address().as_u64();
                    let new_pt: &mut PageTable = unsafe { &mut *(new_pt_virt.as_mut_ptr()) };
                    new_pt.zero();
                    for l in 0..512 {
                        if other_pt[l].is_unused() {
                            continue;
                        }

                        let src_phys = other_pt[l].addr().as_u64();
                        let flags = other_pt[l].flags();

                        // allouer un nouveau frame pour la copie
                        let new_data_frame =
                            paging_manager.allocate_frame().expect("no more frame");
                        let new_data_virt = phys_offset + new_data_frame.start_address().as_u64();

                        let src = unsafe {
                            core::slice::from_raw_parts(
                                (phys_offset + src_phys).as_ptr::<u8>(),
                                4096,
                            )
                        };
                        let dst = unsafe {
                            core::slice::from_raw_parts_mut(new_data_virt.as_mut_ptr::<u8>(), 4096)
                        };
                        dst.copy_from_slice(src);

                        // pointer vers le nouveau frame
                        new_pt[l].set_addr(new_data_frame.start_address(), flags);
                    }
                    let flags = other_pd[k].flags();
                    new_pd[k].set_addr(new_pt_frame.start_address(), flags);
                }
                let flags = other_pdpt[j].flags();
                new_pdpt[j].set_addr(new_pd_frame.start_address(), flags);
            }
            let flags = other_pml4[i].flags();
            new_pml4[i].set_addr(new_pdpt_frame.start_address(), flags);
        }

        Self {
            pml4_frame,
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
            .expect("no more frame ");

        let pml4_virt = phys_offset + pml4_frame.start_address().as_u64();
        let new_pml4: &mut PageTable = unsafe { &mut *(pml4_virt.as_mut_ptr()) };

        new_pml4.zero();

        let kernel_pml4_frame = KERNEL_CR3_FRAME.lock().unwrap();
        let kernel_pml4_virt = phys_offset + kernel_pml4_frame.start_address().as_u64();
        let kernel_pml4: &PageTable = unsafe { &*(kernel_pml4_virt.as_ptr()) };

        for i in 0..512 {
            if !kernel_pml4[i].is_unused() {
                println_serial!(
                    "Copying kernel PML4 entry {}: {:#x}",
                    i,
                    kernel_pml4[i].addr().as_u64()
                );
            }
            new_pml4[i] = kernel_pml4[i].clone();
        }

        if !kernel_pml4[0].is_unused() {
            let pdpt_frame = paging_manager
                .frame_allocator
                .allocate_frame()
                .expect("no more frame");
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

    pub fn dealloc(&mut self) {
        let pml4_frame = self.pml4_frame.start_address().as_u64();
        let mut paging_lock = KERNEL_PAGING_MANAGER.lock();
        let paging = paging_lock.as_mut().unwrap();
        let phys_offset = paging.mapper.phys_offset();

        let pml4: &PageTable = unsafe { &*((phys_offset + pml4_frame).as_ptr()) };

        for i in 0..256 {
            if pml4[i].is_unused() {
                continue;
            }
            if pml4[i].flags().contains(PageTableFlags::HUGE_PAGE) {
                continue;
            }

            let pdpt_phys = pml4[i].addr().as_u64();
            let pdpt: &PageTable = unsafe { &*((phys_offset + pdpt_phys).as_ptr()) };

            for j in 0..512 {
                if pdpt[j].is_unused() {
                    continue;
                }
                if pdpt[j].flags().contains(PageTableFlags::HUGE_PAGE) {
                    continue;
                }

                let pd_phys = pdpt[j].addr().as_u64();
                let pd: &PageTable = unsafe { &*((phys_offset + pd_phys).as_ptr()) };

                for k in 0..512 {
                    if pd[k].is_unused() {
                        continue;
                    }
                    if pd[k].flags().contains(PageTableFlags::HUGE_PAGE) {
                        continue;
                    }

                    let pt_phys = pd[k].addr().as_u64();
                    let pt: &PageTable = unsafe { &*((phys_offset + pt_phys).as_ptr()) };

                    for l in 0..512 {
                        if pt[l].is_unused() {
                            continue;
                        }
                        paging.deallocate_frame(PhysFrame::containing_address(pt[l].addr()));
                    }

                    paging.deallocate_frame(PhysFrame::containing_address(PhysAddr::new(pt_phys)));
                }

                paging.deallocate_frame(PhysFrame::containing_address(PhysAddr::new(pd_phys)));
            }

            paging.deallocate_frame(PhysFrame::containing_address(PhysAddr::new(pdpt_phys)));
        }

        paging.deallocate_frame(self.pml4_frame);
    }
}
