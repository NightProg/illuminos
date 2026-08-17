use alloc::{sync::Arc, vec::Vec};
use bitflags::bitflags;
use spin::Mutex;
use x86_64::structures::paging::PageTableFlags;

use crate::{allocator::process_paging::ProcessPageTable, fs::Inode};

bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct ProtFlags: u64 {
        const NONE = 0;
        const READ  = 1 << 0;
        const WRITE = 1 << 1;
        const EXEC  = 1 << 2;
    }
}

impl ProtFlags {
    pub fn to_page_table_flags(&self) -> PageTableFlags {
        let mut flags = PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE;
        if self.contains(ProtFlags::WRITE) {
            flags |= PageTableFlags::WRITABLE;
        }
        if !self.contains(ProtFlags::EXEC) {
            flags |= PageTableFlags::NO_EXECUTE;
        }
        flags
    }
}

bitflags! {
    #[derive(Debug, Clone)]
    pub struct MapFlags: u64 {
        const NONE        = 0;

        const PRIVATE     = 1 << 0;
        const SHARED      = 1 << 1;
        const ANONYMOUS   = 1 << 2;
        const FIXED       = 1 << 3;
        const FIXED_NO_REPLACE = 1 << 4;
    }
}

#[derive(Clone)]
pub struct VMA {
    pub start: u64,
    pub end: u64,
    pub prot: ProtFlags,
    pub flags: MapFlags,
    pub file: Option<Arc<Mutex<dyn Inode>>>,
    pub offset: usize,
    pub phys_addr: Option<u64>,
}

impl core::fmt::Debug for VMA {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("VMA")
            .field("start", &format_args!("{:#x}", self.start))
            .field("end", &format_args!("{:#x}", self.end))
            .field("prot", &self.prot)
            .field("flags", &self.flags)
            .field("phys_addr", &self.phys_addr)
            .finish()
    }
}
