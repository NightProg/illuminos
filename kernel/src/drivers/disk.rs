pub mod ata;

use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::fmt::Display;
use illfs::Error;

pub trait DiskAction: Send + Sync + Clone {
    fn all_disk() -> Vec<Self> where Self: Sized;
    fn read_sector(&mut self, sector: u64, out: &mut [u8]);
    fn write_sector(&mut self, sector: u64, data: &[u8]);

    fn total_sectors(&self) -> u64;
}


#[derive(Clone)]
pub enum Disk {
    AtaPio(ata::AtaPio),
}

impl Disk {
    pub fn all_disk() -> Vec<Self> {
        let mut disks = Vec::new();
        for ata_disk in ata::AtaPio::detect_disks() {
            disks.push(Disk::AtaPio(ata_disk));
        }
        disks
    }

    pub fn read_sector(&mut self, sector: u64, out: &mut [u8]) {
        match self {
            Disk::AtaPio(disk) => {
                let mut buffer = [0u8; 512];
                disk.read_sector8(sector as u32, &mut buffer);
                out.copy_from_slice(&buffer);
            }
        }
    }

    pub fn write_sector(&mut self, sector: u64, data: &[u8]) {
        match self {
            Disk::AtaPio(disk) => {
                let mut v = Vec::new();
                for i in 0..data.len() {
                    v.push(data[i]);
                }

                v.resize(512, 0);
                disk.write_sector8(sector as u32, &v.try_into().unwrap());

                disk.flush_cache();
            }
        }
    }

    pub fn total_sectors(&self) -> u64 {
        match self {
            Disk::AtaPio(disk) => disk.total_sectors as u64,
        }
    }
}

impl illfs::InOutDevice for Disk {
    fn read(&mut self, offset: u64, buf: &mut [u8]) -> Result<(), Error> {
        let sector_size = 512;
        let start_sector = offset / sector_size;
        let end_sector = (offset + buf.len() as u64 + sector_size as u64 - 1) / sector_size;

        for sector in start_sector..end_sector {
            let mut sector_data = [0u8; 512];
            self.read_sector(sector, &mut sector_data);

            let buf_start = if sector == start_sector {
                (offset % sector_size) as usize
            } else {
                0
            };
            let buf_end = if sector == end_sector - 1 {
                ((offset + buf.len() as u64) % sector_size) as usize
            } else {
                sector_size as usize
            };

            let buf_offset = (sector - start_sector) * sector_size + buf_start as u64;
            buf[buf_offset as usize..(buf_offset as usize + (buf_end - buf_start))]
                .copy_from_slice(&sector_data[buf_start..buf_end]);
        }

        Ok(())
    }

    fn write(&mut self, offset: u64, buf: &[u8]) -> Result<(), Error> {
        let sector_size = 512;
        let start_sector = offset / sector_size;
        let end_sector = (offset + buf.len() as u64 + sector_size - 1) / sector_size;

        for sector in start_sector..end_sector {
            let mut sector_data = [0u8; 512];
            self.read_sector(sector, &mut sector_data);

            let buf_start = if sector == start_sector {
                (offset % sector_size) as usize
            } else {
                0
            };
            let buf_end = if sector == end_sector - 1 {
                ((offset + buf.len() as u64) % sector_size) as usize
            } else {
                sector_size as usize
            };

            let buf_offset = (sector - start_sector) * sector_size + buf_start as u64;
            sector_data[buf_start..buf_end]
                .copy_from_slice(&buf[buf_offset as usize..(buf_offset as usize + (buf_end - buf_start))]);

            self.write_sector(sector, &sector_data);
        }

        Ok(())
    }

    fn size(&self) -> u64 {
        self.total_sectors() * 512
    }

    fn close(&self) -> Result<(), Error> {
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiskKind {
    AtaPio
}

impl Display for DiskKind {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            DiskKind::AtaPio => write!(f, "ata"),
        }
    }
}