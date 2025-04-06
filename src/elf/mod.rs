use alloc::vec::Vec;

pub type ElfAddr = u64;
pub type ElfOff = u64;
pub type ElfWord = u32;
pub type ElfHalf = u16;
pub type ElfXWord = u64;
pub type ElfSWord = i32;
pub type ElfSXWord = i64;

#[derive(Debug, Clone, Copy)]
pub enum ElfFileKind {
    Executable,
    SharedObject,
    Relocatable,
}

#[derive(Debug, Clone, Copy)]
pub enum ElfClassKind {
    Class32,
    Class64,
}

#[derive(Debug, Clone, Copy)]
pub enum ElfOsAbi {
    SysV,
    HpuX,
    Standalone,
}

#[derive(Debug, Clone, Copy)]
pub enum ElfEndianness {
    LittleEndian,
    BigEndian,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ElfHeader64 {
    pub magic: [u8; 4],
    pub class: u8,
    pub data: u8,
    pub version: u8,
    pub os_abi: u8,
    pub abi_version: u8,
    pub pad: [u8; 7],
    pub e_type: ElfHalf,
    pub e_machine: ElfHalf,
    pub e_version: ElfWord,
    pub e_entry: ElfAddr,
    pub e_phoff: ElfOff,
    pub e_shoff: ElfOff,
    pub e_flags: ElfWord,
    pub e_ehsize: ElfHalf,
    pub e_phentsize: ElfHalf,
    pub e_phnum: ElfHalf,
    pub e_shentsize: ElfHalf,
    pub e_shnum: ElfHalf,
    pub e_shstrndx: ElfHalf,
}

pub const ELF_MAGIC: [u8; 4] = [0x7f, b'E', b'L', b'F'];
pub const ELF_CLASS_32: u8 = 1;
pub const ELF_CLASS_64: u8 = 2;

pub const ELF_DATA_LSB: u8 = 1;
pub const ELF_DATA_MSB: u8 = 2;
pub const ELF_VERSION_CURRENT: u32 = 1;

impl ElfHeader64 {
    pub fn parse<'c>(buffer: impl AsRef<[u8]>) -> Option<ElfHeader64> {
        let x = buffer.as_ref();
        let elf_length = size_of::<ElfHeader64>();
        if x.len() < elf_length {
            return None;
        }
        let header = unsafe { &*(x.as_ptr() as *const ElfHeader64) };

        if header.magic != ELF_MAGIC {
            return None;
        }
        Some(*header)
    }
    pub fn new() -> Self {
        ElfHeader64 {
            magic: ELF_MAGIC,
            class: ELF_CLASS_64,
            data: ELF_DATA_LSB,
            version: ELF_VERSION_CURRENT as u8,
            os_abi: 0,
            abi_version: 0,
            pad: [0; 7],
            e_type: 0,
            e_machine: 0,
            e_version: ELF_VERSION_CURRENT,
            e_entry: 0,
            e_phoff: 0,
            e_shoff: 0,
            e_flags: 0,
            e_ehsize: 0,
            e_phentsize: 0,
            e_phnum: 0,
            e_shentsize: 0,
            e_shnum: 0,
            e_shstrndx: 0,
        }
    }
    pub fn get_class(&self) -> ElfClassKind {
        match self.class {
            ELF_CLASS_64 => ElfClassKind::Class64,
            ELF_CLASS_32 => ElfClassKind::Class32,
            _ => panic!("Unknown ELF class: {}", self.class),
        }
    }

    pub fn get_kind(&self) -> ElfFileKind {
        match self.e_type {
            0x02 => ElfFileKind::Executable,
            0x03 => ElfFileKind::SharedObject,
            0x04 => ElfFileKind::Relocatable,
            _ => panic!("Unknown ELF type: {}", self.e_type),
        }
    }

    pub fn get_endianness(&self) -> ElfEndianness {
        match self.data {
            ELF_DATA_LSB => ElfEndianness::LittleEndian,
            ELF_DATA_MSB => ElfEndianness::BigEndian,
            _ => panic!("Unknown ELF endianness: {}", self.data),
        }
    }

    pub fn get_os_abi(&self) -> ElfOsAbi {
        match self.os_abi {
            0x0 => ElfOsAbi::SysV,
            0x1 => ElfOsAbi::HpuX,
            255 => ElfOsAbi::Standalone,
            e => panic!("Unknown ELF OS ABI: {}", e),
        }
    }

    pub fn get_elf_program_header(&self, buffer: impl AsRef<[u8]>) -> ElfProgramHeader64 {
        let x = buffer.as_ref();
        let phsize = self.e_phentsize as usize;
        let buf = &x[self.e_phoff as usize..self.e_phoff as usize + phsize];

        let program_header = unsafe {
            (buf.as_ptr() as *const ElfProgramHeader64)
                .as_ref()
                .expect("Failed tp cast")
        };

        *program_header
    }

    pub fn get_elf_segment_table(&self, buffer: impl AsRef<[u8]>) -> Vec<&ElfSectionEntry> {
        let mut segment_table = Vec::new();
        let mut off = self.e_shoff as usize;
        let size = self.e_shentsize as usize;
        let buffer = buffer.as_ref();
        for i in 0..self.e_shnum {
            let entry: &[u8] = &buffer[off..off + size];
            off += size;
            let entry = unsafe {
                (entry.as_ptr() as *const ElfSectionEntry)
                    .as_ref()
                    .expect("Failed to cast")
            };
            segment_table.push(entry);
        }

        segment_table
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ElfProgramHeader64 {
    pub p_type: u32,
    pub p_offset: u64,
    pub p_vaddr: u64,
    pub p_paddr: u64,
    pub p_filesz: u64,
    pub p_memsz: u64,
    pub p_flags: u32,
    pub p_align: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ElfSectionEntry {
    pub sh_index: ElfHalf,
    pub sh_entry: ELfSection,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ELfSection {
    pub sh_name: ElfWord,
    pub sh_type: ElfWord,
    pub sh_flags: ElfXWord,
    pub sh_addr: ElfAddr,
    pub sh_offset: ElfOff,
    pub sh_size: ElfXWord,
    pub sh_link: ElfWord,
    pub sh_info: ElfWord,
    pub sh_addralign: ElfXWord,
    pub sh_entsize: ElfXWord,
}
