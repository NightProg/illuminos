use alloc::{sync::Arc, vec::Vec};
use elf::{ElfBytes, abi::ET_EXEC, endian::AnyEndian};
use x86_64::{
    VirtAddr, align_up,
    structures::paging::{PageTable, PageTableFlags},
};

use crate::{
    allocator::{
        paging::KERNEL_PAGING_MANAGER,
        process_paging::ProcessPageTable,
        vma::{MapFlags, ProtFlags, VMA},
    },
    fs::ramfs::RamFile,
    println_serial,
    thread::process::Process,
};

pub const USER_STACK_TOP: u64 = 0x0000_7FFF_FF00_0000;
pub const USER_STACK_SIZE: u64 = 0x8000;

pub struct ElfProcess {
    page_table: ProcessPageTable,
    elf_data: Vec<u8>,
    vmas: Vec<VMA>,
    brk_start: u64,
    tls_start: u64,
}

impl ElfProcess {
    pub fn new(elf_data: &[u8]) -> Self {
        let page_table = ProcessPageTable::new();

        ElfProcess {
            page_table,
            elf_data: elf_data.to_vec(),
            vmas: Vec::new(),
            brk_start: 0,
            tls_start: 0,
        }
    }

    pub fn load(&mut self) -> Option<VirtAddr> {
        let elf_bytes = ElfBytes::<AnyEndian>::minimal_parse(&self.elf_data).ok()?;

        println_serial!("LOADED ELF BYTES");
        let mut paging_manager_lock = KERNEL_PAGING_MANAGER.lock();
        let paging_manager = paging_manager_lock.as_mut()?;

        if let Some(segments) = elf_bytes.segments() {
            for program_header in segments {
                if program_header.p_type == elf::abi::PT_TLS {
                    self.tls_start = program_header.p_vaddr;
                }
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

                    let mut prot = ProtFlags::empty();
                    if program_header.p_flags & elf::abi::PF_R != 0 {
                        prot.insert(ProtFlags::READ);
                    }
                    if program_header.p_flags & elf::abi::PF_W != 0 {
                        prot.insert(ProtFlags::WRITE);
                    }
                    if program_header.p_flags & elf::abi::PF_X != 0 {
                        prot.insert(ProtFlags::EXEC);
                    }

                    let mut flags = MapFlags::PRIVATE;
                    if is_bss {
                        flags.insert(MapFlags::ANONYMOUS);
                    }

                    let vma = VMA {
                        start: virt_addr.as_u64(),
                        end: virt_addr.as_u64() + mem_size as u64,
                        prot,
                        flags,
                        file: if is_bss {
                            None
                        } else {
                            Some(Arc::new(spin::Mutex::new(RamFile { data: data.clone() })))
                        },
                        offset: 0,
                        phys_addr: None,
                    };

                    self.vmas.push(vma);
                }
            }
        }

        self.brk_start = self.vmas.iter().map(|v| v.end).max().unwrap_or(0);
        self.brk_start = align_up(self.brk_start, 4096);
        Some(VirtAddr::new(elf_bytes.ehdr.e_entry))
    }

    pub fn get_page_table(&self) -> &ProcessPageTable {
        &self.page_table
    }

    pub fn get_vmas(&self) -> &Vec<VMA> {
        &self.vmas
    }

    pub fn get_brk_start(&self) -> u64 {
        self.brk_start
    }
}
