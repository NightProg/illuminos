use alloc::vec::Vec;
use elf::{ElfBytes, abi::ET_EXEC, endian::AnyEndian};
use x86_64::{
    VirtAddr,
    structures::paging::{PageTable, PageTableFlags},
};

use crate::{
    allocator::{paging::KERNEL_PAGING_MANAGER, process_paging::ProcessPageTable},
    thread::process::Process,
};

pub const USER_STACK_TOP: u64 = 0x0000_7FFF_FF00_0000;
pub const USER_STACK_SIZE: u64 = 0x8000;

pub struct ElfProcess {
    page_table: ProcessPageTable,
    elf_data: Vec<u8>,
}

impl ElfProcess {
    pub fn new(elf_data: &[u8]) -> Self {
        let page_table = ProcessPageTable::new();

        ElfProcess {
            page_table,
            elf_data: elf_data.to_vec(),
        }
    }

    pub fn load(&mut self) -> Option<VirtAddr> {
        let elf_bytes = ElfBytes::<AnyEndian>::minimal_parse(&self.elf_data).ok()?;
        let mut paging_manager_lock = KERNEL_PAGING_MANAGER.lock();
        let paging_manager = paging_manager_lock.as_mut()?;

        if let Some(segments) = elf_bytes.segments() {
            for program_header in segments {
                if program_header.p_type == elf::abi::PT_LOAD {
                    let virt_addr = VirtAddr::new(program_header.p_vaddr);
                    let file_size = program_header.p_filesz as usize;
                    let mem_size = program_header.p_memsz as usize;
                    let is_bss = file_size < mem_size;

                    let mut data = self.elf_data[program_header.p_offset as usize
                        ..(program_header.p_offset + file_size as u64) as usize]
                        .to_vec();
                    if is_bss {
                        data.resize(mem_size, 0);
                    }
                    let mut page_table_flags =
                        PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE;
                    if program_header.p_flags & elf::abi::PF_X == 0 {
                        page_table_flags |= PageTableFlags::NO_EXECUTE;
                    }
                    if program_header.p_flags & elf::abi::PF_W != 0 {
                        page_table_flags |= PageTableFlags::WRITABLE;
                    }
                    self.page_table.map_elf_segment(
                        virt_addr,
                        &data,
                        mem_size,
                        page_table_flags,
                        &mut paging_manager.frame_allocator,
                    );
                }
            }
        }
        Some(VirtAddr::new(elf_bytes.ehdr.e_entry))
    }

    pub fn get_page_table(&self) -> &ProcessPageTable {
        &self.page_table
    }
}
