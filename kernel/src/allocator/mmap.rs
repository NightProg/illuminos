use x86_64::{
    VirtAddr, align_down, align_up,
    structures::paging::{FrameDeallocator, Mapper, Page, PhysFrame, Size4KiB, Translate},
};

use crate::{
    allocator::{
        paging::{KERNEL_PAGING_MANAGER, frame_dec_refcount},
        vma::{MapFlags, ProtFlags, VMA},
    },
    dbg,
    thread::process::Process,
};

pub const MMAP_BASE: u64 = 0x7000_0000_0000;

pub fn mmap(
    process: &mut Process,
    addr: u64,
    len: u64,
    prot: ProtFlags,
    flags: MapFlags,
    fd: u64,
    offset: u64,
) -> Result<u64, &'static str> {
    let addr = process
        .find_free_vma_space(len, MMAP_BASE)
        .ok_or("No free space for mmap")?;
    if flags.contains(MapFlags::ANONYMOUS) && fd != 0 {
        return Err("Anonymous mapping cannot have a file descriptor");
    }
    if !flags.contains(MapFlags::ANONYMOUS) && fd == 0 {
        return Err("File-backed mapping must have a file descriptor");
    }

    let file = if let Some(file) = process.get_file(fd) {
        Some(file)
    } else {
        if !flags.contains(MapFlags::ANONYMOUS) {
            return Err("Invalid file descriptor");
        }
        None
    };

    let aligned_start = align_down(addr.as_u64(), 4096);
    let aligned_end = align_up(addr.as_u64() + len, 4096);
    let mut vma = VMA {
        start: aligned_start,
        end: aligned_end,
        prot,
        flags,
        file,
        offset: offset as usize,
        phys_addr: None,
    };

    if let Some(file) = vma.clone().file {
        file.lock()
            .mmap(&mut vma)
            .map_err(|_| "Failed to mmap file")?;
    }

    process.vmas.push(vma);
    Ok(addr.as_u64())
}

pub fn munmap(process: &mut Process, addr: u64, len: u64) -> Result<(), &'static str> {
    if addr == 0 || len == 0 {
        return Err("Invalid address or length");
    }
    if addr % 4096 != 0 {
        return Err("Address must be page-aligned");
    }

    if addr + len >= 0xffff_8000_0000_0000 {
        return Err("Address range exceeds user space limit");
    }

    let aligned_start = align_down(addr, 4096);
    let aligned_end = align_up(addr + len, 4096);

    for va in (aligned_start..aligned_end).step_by(4096) {
        unsafe {
            process.pml4_table.with_mapper(|mapper| {
                if let Some(phys) = mapper.translate_addr(VirtAddr::new(va)) {
                    let frame = PhysFrame::containing_address(phys);
                    let rc = frame_dec_refcount(&frame);
                    if rc == 0 {
                        KERNEL_PAGING_MANAGER
                            .lock()
                            .as_mut()
                            .unwrap()
                            .frame_allocator
                            .deallocate_frame(frame);
                    }
                    mapper.unmap(Page::<Size4KiB>::containing_address(VirtAddr::new(va)));
                }
            });
        }
    }

    process
        .vmas
        .retain(|vma| !(vma.start < aligned_end && vma.end > aligned_start));

    x86_64::instructions::tlb::flush_all();
    Ok(())
}

pub fn mprotect(
    process: &mut Process,
    addr: u64,
    len: u64,
    prot: ProtFlags,
) -> Result<(), &'static str> {
    if addr == 0 || len == 0 {
        return Err("Invalid address or length");
    }
    if addr % 4096 != 0 {
        return Err("Address must be page-aligned");
    }

    let aligned_start = align_down(addr, 4096);
    let aligned_end = align_up(addr + len, 4096);

    for vma in process.vmas.iter_mut() {
        if vma.start < aligned_end && vma.end > aligned_start {
            vma.prot = prot;
            unsafe {
                process.pml4_table.with_mapper(|mapper| {
                    for va in (vma.start..vma.end).step_by(4096) {
                        if let Some(phys) = mapper.translate_addr(VirtAddr::new(va)) {
                            let page = Page::<Size4KiB>::containing_address(VirtAddr::new(va));
                            let frame = PhysFrame::<Size4KiB>::containing_address(phys);
                            mapper
                                .update_flags(page, prot.to_page_table_flags())
                                .unwrap();
                        }
                    }
                });
            }
        }
    }

    x86_64::instructions::tlb::flush_all();
    Ok(())
}
