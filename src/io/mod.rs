use x86_64::instructions::port::Port;

pub mod pci;
pub mod serial;

pub fn outb(port: u16, value: u8) {
    unsafe {
        Port::new(port).write(value);
    }
}
pub fn inb(port: u16) -> u8 {
    unsafe {
        Port::new(port).read()
    }
}

pub fn inw(port: u16) -> u16 {
    unsafe {
        Port::new(port).read()
    }
}

pub fn outw(port: u16, value: u16) {
    unsafe {
        Port::new(port).write(value);
    }
}

pub fn inl(port: u16) -> u32 {
    unsafe {
        Port::new(port).read()
    }
}

pub fn outl(port: u16, value: u32) {
    unsafe {
        Port::new(port).write(value);
    }
}
