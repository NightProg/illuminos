use crate::{info, io::{inb, inw, outb, outw}};
pub mod ata;

const ATA_PRIMARY_IO: u16 = 0x1F0;
const ATA_PRIMARY_CONTROL: u16 = 0x3F6;
const ATA_CMD_READ: u8 = 0x20;

pub fn ata_check_status() -> u8 {
    unsafe { inb(0x1F7) } // Lecture du registre de statut
}


pub fn ata_read_sector(lba: u32, buffer: &mut [u8]) {
    use x86_64::instructions::port::Port;

    unsafe {
        let mut port_data = Port::<u16>::new(ATA_PRIMARY_IO);
        let mut port_cmd = Port::<u8>::new(ATA_PRIMARY_IO + 7);
        let mut port_status = Port::<u8>::new(ATA_PRIMARY_IO + 7);

        outb(ATA_PRIMARY_IO + 6, 0xE0 | ((lba >> 24) & 0x0F) as u8);
        while ata_check_status() & 0x80 != 0 {}

        // Envoyer le LBA
        outb(ATA_PRIMARY_IO + 2, 1); // Nombre de secteurs à lire
        outb(ATA_PRIMARY_IO + 3, (lba & 0xFF) as u8);
        outb(ATA_PRIMARY_IO + 4, ((lba >> 8) & 0xFF) as u8);
        outb(ATA_PRIMARY_IO + 5, ((lba >> 16) & 0xFF) as u8);

        // Envoyer la commande READ SECTORS
        outb(ATA_PRIMARY_IO + 7, ATA_CMD_READ);

        // Attendre que le disque soit prêt
        while inb(ATA_PRIMARY_IO + 7) & 0x08 == 0 {}

        // Lire les 512 octets du secteur
        for i in 0..256 {
            let word = inw(ATA_PRIMARY_IO);
            buffer[i * 2] = (word & 0xFF) as u8;
            buffer[i * 2 + 1] = (word >> 8) as u8;
        }

        let status = ata_check_status();
        if status & 0x01 != 0 {
            info!("Erreur de lecture du disque");
        } else if status & 0x20 == 0 {
            info!("Disque prêt");
        } else {
            info!("Disque non prêt");
        }
    }
}

pub fn ata_write_lba28_u8(
    lba: u32,           // Adresse LBA
    buffer: &[u8],      // Buffer de données à écrire (en u8)
    sector_count: u8,  // Nombre de secteurs à écrire
) {
    assert!(buffer.len() >= 256 * sector_count as usize, "Buffer trop petit");

    unsafe {
        // Attendre que le disque ne soit plus occupé
        while inb(0x1F7) & 0x80 != 0 {} // Tant que BSY = 1, on attend

        // Envoyer le nombre de secteurs à écrire
        outb(0x1F2, sector_count);

        // Envoyer l'adresse LBA
        outb(0x1F3, (lba & 0xFF) as u8);         // Byte 0
        outb(0x1F4, ((lba >> 8) & 0xFF) as u8);  // Byte 1
        outb(0x1F5, ((lba >> 16) & 0xFF) as u8); // Byte 2
        outb(0x1F6, 0xE0 | ((lba >> 24) & 0x0F) as u8); // Byte 3 + LBA mode

        outb(0x1F7, 0x30); // Commande ATA WRITE SECTORS

        for sector in 0..sector_count as usize {
            // Attendre que le disque soit prêt pour les données
            while inb(0x1F7) & 0x80 != 0 {} // Tant que BSY = 1, on attend

            // Écrire 256 mots (512 octets) dans le port de données
            for i in 0..256 {
                let word = (buffer[sector * 256 + i * 2] as u16) | ((buffer[sector * 256 + i * 2 + 1] as u16) << 8);
                outw(0x1F0, word);
            }
        }
        while inb(0x1F7) & 0x80 != 0 {} // Tant que BSY = 1, on attend
    }
}

fn wait_for_irq() {
    loop {
        let status = inb(0x1F7);
        if status & 0x08 != 0 {
            break; 
        }
    }
}


pub fn ata_write_lba28_u16(lba: u32, sector_count: u8, buffer: &[u16]) {
    assert!(buffer.len() >= 256 * sector_count as usize, "Buffer trop petit");

    // Attendre que le disque ne soit plus occupé
    while inb(0x1F7) & 0x80 != 0 {} // Tant que BSY = 1, on attend

    // Envoyer le nombre de secteurs à écrire
    outb(0x1F2, sector_count);

    // Envoyer l'adresse LBA
    outb(0x1F3, (lba & 0xFF) as u8);         // Byte 0 (lba )
    outb(0x1F4, ((lba >> 8) & 0xFF) as u8);  // Byte 1
    outb(0x1F5, ((lba >> 16) & 0xFF) as u8); // Byte 2
    outb(0x1F6, 0xE0 | ((lba >> 24) & 0x0F) as u8); // Byte 3 + LBA mode

    // Envoyer la commande d'écriture
    outb(0x1F7, 0x30); // Commande ATA WRITE SECTORS

    for sector in 0..sector_count as usize {
        // Attendre que le disque soit prêt pour les données
        while inb(0x1F7) & 0x08 == 0 {} // Attendre DRQ = 1

        // Écrire 256 mots (512 octets) dans le port de données
        for i in 0..256 {
            outw(0x1F0, buffer[sector * 256 + i]);
        }
    }

    // Attendre que l'écriture soit terminée
    while inb(0x1F7) & 0x80 != 0 {} // Tant que BSY = 1, on attend
}