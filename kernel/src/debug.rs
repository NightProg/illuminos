use alloc::{
    boxed::Box,
    string::{String, ToString},
};
use core::arch::asm;
use elf::{ElfBytes, abi::STT_FUNC, endian::AnyEndian, symbol::Elf64_Sym};

pub static KERNEL_ELF_FILE: spin::Once<ElfBytes<'static, AnyEndian>> = spin::Once::new();

fn init_symbols() -> Option<impl Iterator<Item = (u64, u64, &'static str)>> {
    let elf = KERNEL_ELF_FILE.r#try()?;
    let (symtab, strtab) = elf.symbol_table().unwrap()?;

    Some(
        symtab
            .iter()
            .filter(|sym| sym.st_symtype() == STT_FUNC)
            .filter_map(move |sym| {
                let name = strtab.get(sym.st_name as usize).ok()?;
                Some((sym.st_value, sym.st_size, name))
            }),
    )
}

pub fn addr_to_name(addr: u64) -> Option<&'static str> {
    init_symbols()?
        .find(|(base, size, _)| addr >= *base && addr < base + size)
        .map(|(_, _, name)| name)
}

pub fn addr_to_demangle_name(addr: u64) -> Option<String> {
    addr_to_name(addr).map(|s| rustc_demangle::demangle(s).to_string())
}

#[repr(C)]
pub struct StackFrame {
    pub rbp: *mut StackFrame,
    pub rip: u64,
}

impl StackFrame {
    pub unsafe fn rbp() -> u64 {
        let rbp: u64;
        asm!("mov {}, rbp", out(reg) rbp);
        rbp
    }
    pub unsafe fn get() -> alloc::vec::Vec<u64> {
        Self::from_rbp(StackFrame::rbp())
    }

    pub unsafe fn from_rbp(rbp: u64) -> alloc::vec::Vec<u64> {
        let mut rbp = rbp as *mut StackFrame;
        let mut backtrace = alloc::vec::Vec::new();
        while !rbp.is_null() {
            let frame = unsafe { &*rbp };
            if frame.rip == 0 {
                break;
            }
            backtrace.push(frame.rip);
            rbp = frame.rbp;
        }
        backtrace
    }

    pub unsafe fn get_resolved() -> Option<alloc::vec::Vec<&'static str>> {
        Self::get().iter().map(|addr| addr_to_name(*addr)).collect()
    }
}
