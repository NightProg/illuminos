use alloc::vec::Vec;
use x86_64::instructions::port::Port;
use x86_64::{PhysAddr, VirtAddr};
use x86_64::structures::paging::PageTableFlags;
use crate::allocator::paging::KERNEL_PAGING_MANAGER;
use crate::println;

const PCI_CONFIG_ADDRESS: Port<u32> = Port::new(0xCF8);
const PCI_CONFIG_DATA: Port<u32> = Port::new(0xCFC);

pub unsafe fn pci_read(bus: u8, device: u8, function: u8, offset: u8) -> u32 {
    let address = 0x80000000
        | ((bus as u32) << 16)
        | ((device as u32) << 11)
        | ((function as u32) << 8)
        | (offset as u32 & 0xFC);
    PCI_CONFIG_ADDRESS.write(address);
    PCI_CONFIG_DATA.read()
}

pub unsafe fn pci_write(bus: u8, device: u8, function: u8, offset: u8, value: u32) {
    let address = 0x80000000
        | ((bus as u32) << 16)
        | ((device as u32) << 11)
        | ((function as u32) << 8)
        | (offset as u32 & 0xFC);

    PCI_CONFIG_ADDRESS.write(address);
    PCI_CONFIG_DATA.write(value);
}

pub unsafe fn pci_read_bar(bus: u8, device: u8, function: u8, bar_index: usize) -> u32 {
    let offset = 0x10 + (bar_index as u8 * 4);
    pci_read(bus, device, function, offset)
}

pub unsafe fn get_vendor_id(bus: u8, device: u8, function: u8) -> u16 {
    (pci_read(bus, device, function, 0x00) & 0xFFFF) as u16
}

pub unsafe fn get_device_id(bus: u8, device: u8, function: u8) -> u16 {
    ((pci_read(bus, device, function, 0x00) >> 16) & 0xFFFF) as u16
}

#[derive(Debug, Clone, Copy)]
pub struct PciDevice {
    pub bus: u8,
    pub device: u8,
    pub function: u8,
    pub vendor_id: u16,
    pub device_id: u16,
    pub class_code: u8,
    pub subclass: u8,
    pub prog_if: u8,
    pub header_type: u8,
    pub mmio_base: Option<*mut u8>,
}

impl PciDevice {

    pub unsafe fn scan_virtio_devices() -> Vec<PciDevice> {
        let mut devices = Vec::new();
        for bus in 0..255 {
            for device in 0..32 {
                for function in 0..8 {
                    if let Some(device) = Self::new(bus, device, function) {
                        if device.is_virtio() {
                            devices.push(device);
                        }
                    }
                }
            }
        }
        devices
    }
    pub unsafe fn new(bus: u8, device: u8, function: u8) -> Option<Self> {
        let vendor_id = pci_read(bus, device, function, 0x00) as u16;
        if vendor_id == 0xFFFF {
            return None;
        }

        let device_id = (pci_read(bus, device, function, 0x00) >> 16) as u16;
        let class_reg = pci_read(bus, device, function, 0x08);
        let header_type = ((pci_read(bus, device, function, 0x0C) >> 16) & 0xFF) as u8;

        let mmio_base = if header_type & 0x80 != 0 {
            let bar = pci_read_bar(bus, device, function, 0);
            if bar != 0 {
                Some((bar & 0xFFFFFFF0) as u64)
            } else {
                None
            }
        } else {
            None
        }.map(|addr| {
            let size_of_mmio = {
                let original = pci_read(bus, device, function, 0x10);
                pci_write(bus, device, function, 0x10, 0xFFFFFFFF);
                let size_encoded = pci_read(bus, device, function, 0x10);
                pci_write(bus, device, function, 0x10, original);
                let size_masked = size_encoded & 0xFFFF_FFF0;
                if size_masked == 0 {
                    0
                } else {
                    (!size_masked + 1) as usize
                }
            };

            let virt_addr = VirtAddr::new(addr);
            let phys_addr = PhysAddr::new(addr);

            let mut paging_manager_lock = KERNEL_PAGING_MANAGER.lock();
            if let Some(paging_manager) = paging_manager_lock.as_mut() {
                paging_manager.map_memory(virt_addr, size_of_mmio, phys_addr, PageTableFlags::PRESENT | PageTableFlags::WRITABLE);
                return virt_addr.as_mut_ptr();
            } else {
                return core::ptr::null_mut();
            }
        });

        Some(Self {
            bus,
            device,
            function,
            vendor_id,
            device_id,
            class_code: ((class_reg >> 24) & 0xFF) as u8,
            subclass: ((class_reg >> 16) & 0xFF) as u8,
            prog_if: ((class_reg >> 8) & 0xFF) as u8,
            header_type,
            mmio_base,
        })
    }

    pub unsafe fn read_byte(&self, offset: u8) -> u8 {
        (pci_read(self.bus, self.device, self.function, offset) & 0xFF) as u8
    }


    pub unsafe fn read_dword(&self, offset: u8) -> u32 {
        pci_read(self.bus, self.device, self.function, offset)
    }

    pub unsafe fn read_config_mmio(&self, offset: u16) -> Option<u32> {
        println!("MMIO: {:?}", self.mmio_base);
        let base: *const u32 = self.mmio_base? as *const u32;
        Some(core::ptr::read_volatile(base.add((offset / 4) as usize)))
    }

    pub unsafe fn write_dword(&self, offset: u8, value: u32) {
        pci_write(self.bus, self.device, self.function, offset, value);
    }

    pub unsafe fn get_bar(&self, index: u8) -> Option<u64> {
        if index >= 6 {
            return None;
        }
        let offset = 0x10 + index * 4;
        let value = self.read_dword(offset);

        if value == 0 {
            return None;
        }

        if value & 0x1 == 0x1 {
            Some((value & 0xFFFFFFFC) as u64)
        } else {
            let is_64bit = (value >> 1) & 0b11 == 0b10;
            if is_64bit {
                let upper = self.read_dword(offset + 4);
                Some(((upper as u64) << 32) | ((value & 0xFFFFFFF0) as u64))
            } else {
                Some((value & 0xFFFFFFF0) as u64)
            }
        }
    }

    pub fn is_multifunction(&self) -> bool {
        self.header_type & 0x80 != 0
    }

    pub fn is_bridge(&self) -> bool {
        self.class_code == 0x06 && self.subclass == 0x04
    }

    pub unsafe fn is_virtio(&self) -> bool {
        let virtio_vendor_id = 0x1AF4;

        self.vendor_id == virtio_vendor_id && (0x1000..=0x107F).contains(&self.device_id)
    }
}
