pub mod ata;

use alloc::vec::Vec;

pub trait Disk {
    fn read_sector(&mut self, sector: u64) -> Vec<u8>;
    fn write_sector(&mut self, sector: u64, data: &[u8]);
}
