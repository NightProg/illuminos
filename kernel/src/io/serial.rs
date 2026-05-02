use core::{arch::asm, fmt::Write};

use x86_64::instructions::port::Port;


const SERIAL_PORT: u16 = 0x3F8; // Adresse du port série (COM1)

pub struct SerialPortWriter;

impl SerialPortWriter {

    pub fn write_string(&mut self, str: &str) {
        for byte in str.bytes() {
            self.write_byte(byte);
        }
    }
    pub fn write_byte(&mut self, byte: u8) {
        serial_write(byte);
    }
}

impl Write for SerialPortWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.write_string(s);
        Ok(())
    }
}


fn serial_write(byte: u8) {
    unsafe {
        // Attendre que le port soit prêt à recevoir des données
        while !is_serial_transmit_empty() {}

        // Envoyer le byte dans le port
        outb(SERIAL_PORT, byte);
    }
}

/// Vérifie si le port série est prêt à envoyer des données
fn is_serial_transmit_empty() -> bool {
    unsafe { inb(SERIAL_PORT + 5) & 0x20 != 0 }
}

/// Lecture d'un octet depuis le port série (facultatif si vous en avez besoin)
fn serial_read() -> u8 {
    unsafe {
        while is_serial_data_ready() == false {}

        return inb(SERIAL_PORT);
    }
}

/// Vérifie si des données sont prêtes à être lues depuis le port série
fn is_serial_data_ready() -> bool {
    unsafe { inb(SERIAL_PORT + 5) & 1 != 0 }
}

/// Fonctions pour écrire/ lire des données depuis les ports d'IO
unsafe fn outb(port: u16, value: u8) {
    Port::new(port).write(value);
}

unsafe fn inb(port: u16) -> u8 {
    Port::new(port).read()
}



#[macro_export]
macro_rules! print_serial {
    ($($arg:tt)*) => ({
        use core::fmt::Write;
        let mut writer = $crate::io::serial::SerialPortWriter;
        write!(writer, $($arg)*).unwrap();
    });
}
#[macro_export]
macro_rules! println_serial {
    ($($arg:tt)*) => ({
        use core::fmt::Write;
        let mut writer = $crate::io::serial::SerialPortWriter;
        writeln!(writer, $($arg)*).unwrap();
    });
}