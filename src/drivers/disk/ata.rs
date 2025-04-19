use alloc::vec::Vec;
use core::ptr::read_volatile;
use core::ptr::write_volatile;
use x86_64::instructions::port::{Port, PortRead, PortWrite};

const ATA_PRIMARY_IO: u16 = 0x1F0;
const ATA_PRIMARY_CONTROL: u16 = 0x3F6;
const ATA_SECONDARY_IO: u16 = 0x170;
const ATA_SECONDARY_CONTROL: u16 = 0x376;

#[derive(Debug, Clone)]
pub struct AtaPio {
    data: Port<u16>,
    error: Port<u8>,
    sector_count: Port<u8>,
    lba_lo: Port<u8>,
    lba_mid: Port<u8>,
    lba_hi: Port<u8>,
    drive_select: Port<u8>,
    command: Port<u8>,
    status: Port<u8>,
    base: u16,
    control: u16,
    drive: u8,
    total_sectors: u32,
}

impl AtaPio {
    pub const fn new(base: u16, control: u16, drive: u8) -> Self {
        Self {
            data: Port::new(base),
            error: Port::new(base + 1),
            sector_count: Port::new(base + 2),
            lba_lo: Port::new(base + 3),
            lba_mid: Port::new(base + 4),
            lba_hi: Port::new(base + 5),
            drive_select: Port::new(base + 6),
            command: Port::new(base + 7),
            status: Port::new(base + 7),
            base,
            control,
            drive,
            total_sectors: 0,
        }
    }

    pub fn detect_disks() -> Vec<AtaPio> {
        let mut disks = Vec::new();
        let controllers = [
            (ATA_PRIMARY_IO, ATA_PRIMARY_CONTROL),
            (ATA_SECONDARY_IO, ATA_SECONDARY_CONTROL),
        ];
        let drives = [0xA0, 0xB0]; // Maître et Esclave

        for &(base, control) in &controllers {
            for &drive in &drives {
                let mut drive_select = Port::<u8>::new(base + 6);
                unsafe {
                    drive_select.write(drive);
                }

                // Lire le status
                let mut status = Port::<u8>::new(base + 7);
                let value = unsafe { status.read() };

                if value != 0xFF {
                    // Vérifier si un disque est présent
                    let mut disk = AtaPio::new(base, control, drive);
                    disk.total_sectors = disk.get_total_sectors();
                    disks.push(disk);
                }
            }
        }
        disks
    }

    pub fn get_info(&self) -> (u16, u16, u8, u32) {
        (self.base, self.control, self.drive, self.total_sectors)
    }

    pub fn get_total_sectors(&mut self) -> u32 {
        unsafe {
            self.drive_select.write(0xE0);
            self.command.write(0xEC);
        }

        while unsafe { self.status.read() } & 0x80 != 0 {}

        let mut buffer = [0u16; 256];
        for i in 0..256 {
            buffer[i] = unsafe { self.data.read() };
        }

        let total_sectors = ((buffer[61] as u32) << 16) | (buffer[60] as u32);
        total_sectors
    }

    pub fn read_sector(&mut self, lba: u32, buffer: &mut [u16; 256]) {
        unsafe {
            self.drive_select.write(0xE0 | ((lba >> 24) as u8 & 0x0F));
            self.sector_count.write(1);
            self.lba_lo.write(lba as u8);
            self.lba_mid.write((lba >> 8) as u8);
            self.lba_hi.write((lba >> 16) as u8);
            self.command.write(0x20);
        }

        while unsafe { self.status.read() } & 0x80 != 0 {}

        for i in 0..256 {
            buffer[i] = unsafe { self.data.read() };
        }
    }

    pub fn read_sector8(&mut self, lba: u32, buffer: &mut [u8; 512]) {
        unsafe {
            self.drive_select.write(0xE0 | ((lba >> 24) as u8 & 0x0F));
            self.sector_count.write(1);
            self.lba_lo.write(lba as u8);
            self.lba_mid.write((lba >> 8) as u8);
            self.lba_hi.write((lba >> 16) as u8);
            self.command.write(0x20);
        }

        while unsafe { self.status.read() } & 0x80 != 0 {}

        for i in 0..256 {
            let word = unsafe { self.data.read() };
            buffer[i * 2] = (word & 0xFF) as u8;
            buffer[i * 2 + 1] = (word >> 8) as u8;
        }
    }

    pub fn write_sector(&mut self, lba: u32, buffer: &[u16; 256]) {
        unsafe {
            self.drive_select.write(0xE0 | ((lba >> 24) as u8 & 0x0F));
            self.sector_count.write(1);
            self.lba_lo.write(lba as u8);
            self.lba_mid.write((lba >> 8) as u8);
            self.lba_hi.write((lba >> 16) as u8);
            self.command.write(0x30);
        }

        while unsafe { self.status.read() } & 0x80 != 0 {}

        for i in 0..256 {
            unsafe { self.data.write(buffer[i]) };
        }
    }

    pub fn write_sector8(&mut self, lba: u32, buffer: &[u8; 512]) {
        unsafe {
            self.drive_select.write(0xE0 | ((lba >> 24) as u8 & 0x0F));
            self.sector_count.write(1);
            self.lba_lo.write(lba as u8);
            self.lba_mid.write((lba >> 8) as u8);
            self.lba_hi.write((lba >> 16) as u8);
            self.command.write(0x30);
        }

        while unsafe { self.status.read() } & 0x80 != 0 {}

        for i in 0..256 {
            let word = (buffer[i * 2] as u16) | ((buffer[i * 2 + 1] as u16) << 8);
            unsafe { self.data.write(word) };
        }
    }

    pub fn get_sector_size(&mut self) -> u32 {
        // Envoie la commande IDENTIFY DEVICE
        unsafe {
            self.drive_select.write(0xE0);
            self.command.write(0xEC); // IDENTIFY DEVICE
        }

        // Attendre que le statut indique que l'opération est terminée
        while unsafe { self.status.read() } & 0x80 != 0 {}

        let mut buffer = [0u16; 256]; // Le buffer est de 512 octets (256 mots de 16 bits)

        // Lire les 256 mots (512 octets) dans le buffer
        for i in 0..256 {
            buffer[i] = unsafe { self.data.read() };
        }

        // La taille du secteur est généralement stockée à l'index 212 et 213 dans le buffer.
        // La valeur à cet endroit représente la taille du secteur en octets (généralement 512).
        let sector_size = buffer[212] as u32;

        sector_size
    }

    pub fn get_identify_buffer(&mut self) -> [u16; 256] {
        unsafe {
            self.drive_select.write(0xE0);
            self.command.write(0xEC); // IDENTIFY DEVICE
        }

        while unsafe { self.status.read() } & 0x80 != 0 {}

        let mut buffer = [0u16; 256];
        for i in 0..256 {
            buffer[i] = unsafe { self.data.read() };
        }

        buffer
    }

    pub fn flush_cache(&mut self) {
        unsafe {
            self.drive_select.write(0xE0);
            self.command.write(0xE7); // FLUSH CACHE
        }

        while unsafe { self.status.read() } & 0x80 != 0 {}
    }
}

impl super::Disk for AtaPio {
    fn read_sector(&mut self, sector: u64) -> Vec<u8> {
        let mut buffer = [0; 512];

        self.read_sector8(sector as u32, &mut buffer);
        buffer.to_vec()
    }

    fn write_sector(&mut self, sector: u64, data: &[u8]) {
        let mut v = Vec::new();
        for i in 0..data.len() {
            v.push(data[i]);
        }

        v.resize(512, 0);
        self.write_sector8(sector as u32, &v.try_into().unwrap());

        self.flush_cache();
    }
}
