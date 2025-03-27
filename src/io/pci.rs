use x86_64::instructions::port::Port;

const PCI_CONFIG_ADDRESS: Port<u32> = Port::new(0xCF8);
const PCI_CONFIG_DATA: Port<u32> = Port::new(0xCFC);


pub unsafe fn pci_read(bus: u8, device: u8, function: u8, offset: u8) -> u32 {
    let address = 0x80000000 | ((bus as u32) << 16) | ((device as u32) << 11) | ((function as u32) << 8) | (offset as u32 & 0xFC);
    PCI_CONFIG_ADDRESS.write(address);
    PCI_CONFIG_DATA.read()
}

pub unsafe fn pci_write(bus: u8, device: u8, function: u8, offset: u8, value: u32) {
    let address = 0x80000000 | ((bus as u32) << 16) | ((device as u32) << 11) | ((function as u32) << 8) | (offset as u32 & 0xFC);
    
    PCI_CONFIG_ADDRESS.write(address);
    PCI_CONFIG_DATA.write(value);
}
