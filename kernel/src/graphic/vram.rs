use crate::io::{inl, outl};
use crate::io::pci::{pci_read, pci_read_bar};

pub const VRAM_VIRT_ADDR: u64 = 0xAFF0_0000;

pub fn find_gpu() -> Option<(u8, u8, u8)> {
    for bus in 0..=255 {
        for device in 0..=31 {
            for function in 0..=7 {
                let vendor_id = unsafe {
                    pci_read(bus, device, function, 0x00) & 0xFFFF
                };
                if vendor_id == 0xFFFF { continue; } 

                let class_code = unsafe {
                    pci_read(bus, device, function, 0x08) >> 24
                };
                if class_code == 0x03 {
                    return Some((bus, device, function));
                }
            }
        }
    }
    None
}

pub fn get_vram_addr((bus, device, function): (u8, u8, u8)) -> Option<u32> {

    for bar_index in 0..6 {
        let bar = unsafe {
            pci_read_bar(bus, device, function, bar_index)
        };
        if (bar & 0x1) == 0 {
            let addr = bar & !0xF;
            return Some(addr);
        }
    }
    
    None
}


pub fn unmap_vram(addr: u32) {
    unsafe {
        outl(0xCF8, 0x80000000 | (addr >> 16));
        outl(0xCFC, 0);
    }
}