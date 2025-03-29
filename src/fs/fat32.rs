use core::convert::TryInto;

use alloc::string::ToString;
use alloc::vec::Vec;
use alloc::vec;

use crate::drivers::disk::ata::AtaPio;
use crate::info;

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct BootSector {
    jump_boot: [u8; 3],
    oem_name: [u8; 8],
    bytes_per_sector: u16,
    sectors_per_cluster: u8,
    reserved_sectors: u16,
    fat_count: u8,
    root_entry_count: u16,
    total_sectors_16: u16,
    media: u8,
    fat_size_16: u16,
    sectors_per_track: u16,
    heads_per_cylinder: u16,
    hidden_sectors: u32,
    total_sectors_32: u32,
    fat_size_32: u32,
    flags: u16,
    version: u16,
    pub root_cluster: u32,
    fs_info_sector: u16,
    backup_boot_sector: u16,
    reserved: [u8; 12],
    drive_number: u8,
    reserved1: u8,
    boot_signature: u8,
    volume_id: u32,
    volume_label: [u8; 11],
    fs_type: [u8; 8],
}

impl BootSector {
    pub fn from_sector(sector: &[u8]) -> Self {
        unsafe { core::ptr::read(sector.as_ptr() as *const _) }
    }
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct DirectoryEntry {
    name: [u8; 11],
    attr: u8,
    reserved: u8,
    ctime_tenth: u8,
    ctime: u16,
    cdate: u16,
    adate: u16,
    first_cluster_high: u16,
    mtime: u16,
    mdate: u16,
    first_cluster_low: u16,
    size: u32,
}


impl DirectoryEntry {
    pub fn is_valid(&self) -> bool {
        self.name[0] != 0x00 && self.name[0] != 0xE5
    }

    pub fn cluster(&self) -> u32 {
        let high = (self.first_cluster_high as u32) << 16;
        let low = self.first_cluster_low as u32;
        high | low
    }
}

pub struct FAT32 {
    pub boot_sector: BootSector,
    pub fat_start: u32,
    pub data_start: u32,
    pub cluster_size: u32,
}

impl FAT32 {
    pub fn new(disk: &mut AtaPio) -> Self {
        let mut sector = [0u8; 512];
        disk.read_sector8(0, &mut sector);
        let boot_sector = BootSector::from_sector(&sector);
        
        let fat_start = boot_sector.reserved_sectors as u32;
        let data_start = fat_start + (boot_sector.fat_count as u32 * boot_sector.fat_size_32);
        let cluster_size = boot_sector.sectors_per_cluster as u32 * boot_sector.bytes_per_sector as u32;
        
        Self { boot_sector, fat_start, data_start, cluster_size }
    }

    pub fn read_cluster(&self, disk: &mut AtaPio, cluster: u32) -> Vec<u8> {
        let mut final_buffer = Vec::new();
        for i in 0..self.boot_sector.sectors_per_cluster {
            let sector = self.data_start + (cluster - 2) * self.boot_sector.sectors_per_cluster as u32 + i as u32;
            let mut buffer = [0u8; 512];
            disk.read_sector8(sector, &mut buffer);
            final_buffer.extend_from_slice(&buffer);
        }

        final_buffer

    }

    pub fn read_fat_entry(&mut self, ata: &mut AtaPio, cluster: u32) -> u32 {
        let fat_sector = self.cluster_to_sector(cluster);
        let mut buffer = [0u8; 512];
        ata.read_sector8(fat_sector, &mut buffer);
        let offset = (cluster * 4) % 512;
        u32::from_le_bytes(buffer[offset as usize..offset as usize + 4].try_into().unwrap())
    }

    pub fn write_fat_entry(&mut self, ata: &mut AtaPio, cluster: u32, value: u32) {
        let fat_sector = self.cluster_to_sector(cluster);
        let mut buffer = [0u8; 512];
        ata.read_sector8(fat_sector, &mut buffer);
        let offset = (cluster * 4) % 512;
        buffer[offset as usize..offset as usize + 4].copy_from_slice(&value.to_le_bytes());
        ata.write_sector8(fat_sector, &buffer);
    }

    pub fn find_free_cluster(&mut self, ata: &mut AtaPio) -> Option<u32> {
        let mut buffer = [0u8; 512];
        for cluster in 2..261627 { // On commence après les clusters réservés
            let fat_sector = self.fat_start + (cluster * 4 / 512);
            let offset = (cluster * 4) % 512;
    
            ata.read_sector8(fat_sector, &mut buffer);
            let entry = u32::from_le_bytes([
                buffer[offset as usize],
                buffer[offset as usize + 1],
                buffer[offset as usize + 2],
                buffer[offset as usize + 3],
            ]);
    
            if entry == 0 {
                return Some(cluster);
            }
        }
        None
    }
    

    pub fn read_directory(&self, disk: &mut AtaPio, cluster: u32) -> Vec<DirectoryEntry> {
        let data = self.read_cluster(disk, cluster);
        let mut entries = Vec::new();
        for i in (0..data.len()).step_by(32) {
            let entry: DirectoryEntry = unsafe { core::ptr::read(data[i..].as_ptr() as *const _) };
            if entry.is_valid() {
                entries.push(entry);
            }
        }
        entries
    }


    pub fn create_file(&mut self, ata: &mut AtaPio,  dir_cluster: u32, name: &str, is_directory: bool) -> Option<DirectoryEntry> {
        let mut buffer = [0u8; 512];
        let sector = self.data_start + (dir_cluster - 2) * self.cluster_size;
        ata.read_sector8(sector, &mut buffer);
        for i in (0..512).step_by(size_of::<DirectoryEntry>()) {
            let entry: DirectoryEntry = unsafe { core::ptr::read(buffer.as_ptr().add(i) as *const _) };
            if entry.name[0] == 0x00 {
                let mut new_entry = DirectoryEntry {
                    name: [0x20; 11],
                    attr: if is_directory { 0x10 } else { 0x20 },
                    reserved: 0,
                    ctime: 0,
                    cdate: 0,
                    adate: 0,
                    first_cluster_high: 0,
                    mtime: 0,
                    mdate: 0,
                    first_cluster_low: 0,
                    size: 0,
                    ctime_tenth: 0,
                };
                if is_directory {
                    new_entry.name = [0x2E; 11];
                    
                    let name = name.as_bytes();
                    let mut name_bytes = name.to_vec();
                    name_bytes.resize(11, 0x20);
                    new_entry.name.copy_from_slice(&name_bytes);

                } else {
                    let name_bytes = name.as_bytes();
                    let mut parts = name.splitn(2, '.');
                    let base_name = parts.next().unwrap_or("");
                    let extension = parts.next().unwrap_or("");
                    let mut name_bytes = base_name.as_bytes().to_vec();
                    name_bytes.resize(8, 0x20);
                    let mut ext_bytes = extension.as_bytes().to_vec();
                    ext_bytes.resize(3, 0x20);
                    new_entry.name[..8].copy_from_slice(&name_bytes);
                    new_entry.name[8..11].copy_from_slice(&ext_bytes);
                }
                unsafe {
                    core::ptr::copy_nonoverlapping(&new_entry as *const _ as *const u8, buffer.as_mut_ptr().add(i), size_of::<DirectoryEntry>());
                }
                ata.write_sector8(sector, &buffer);
                ata.read_sector8(sector, &mut buffer);

                return Some(new_entry);
            }
        }
        None
    }

    pub fn create_directory(&mut self, ata: &mut AtaPio, dir_cluster: u32, name: &str) -> Option<DirectoryEntry> {
        let entrie = self.create_file(ata, dir_cluster, name, true)?;

        self.write_fat_entry(ata, cluster, value);
   
    }
    pub fn delete_file(&mut self, ata: &mut AtaPio, dir_cluster: u32, name: &str) -> bool {
        let entries = self.read_directory(ata, dir_cluster);
        let entry_index = entries.iter().position(|e| {
            let mut name = [0u8; 11];
            name.copy_from_slice(&e.name);
            let mut name = name.to_vec();
            name.retain(|&x| x != 0x20);
            let name = core::str::from_utf8(&name).unwrap_or("").trim();
            // remove filename extension
            let mut parts = name.splitn(2, '.');
            let base_name = parts.next().unwrap_or("");
            let extension = parts.next().unwrap_or("");
            let filename = base_name.to_string() + extension;
            name == filename.as_str()
        });

        if entry_index.is_none() {
            return false; // File not found
        }

        let entry_index = entry_index.unwrap();
        let mut entry = entries[entry_index];

        entry.name[0] = 0xE5;
        self.update_directory_entry(ata, dir_cluster, entry_index, &entry);

        true
    }

    pub fn cluster_to_sector(&self, cluster: u32) -> u32 {
        self.data_start + (cluster - 2) * self.boot_sector.sectors_per_cluster as u32
    }

    pub fn write_file(&mut self, ata: &mut AtaPio, dir_cluster: u32, filename: &str, data: &[u8]) -> bool {
        let mut entries = self.read_directory(ata, dir_cluster);
        let entry_index = entries.iter().position(|e| {
            let mut name = [0u8; 11];
            name.copy_from_slice(&e.name);
            let mut name = name.to_vec();
            name.retain(|&x| x != 0x20);
            let name = core::str::from_utf8(&name).unwrap_or("").trim();
            // remove filename extension
            let mut parts = filename.splitn(2, '.');
            let base_name = parts.next().unwrap_or("");
            let extension = parts.next().unwrap_or("");
            let filename = base_name.to_string() + extension;
            name == filename.as_str()
        });

        if entry_index.is_none() {
            return false; // File not found
        }

        let entry_index = entry_index.unwrap();
        let mut entry = entries[entry_index];

        let mut remaining_data = data;
        let mut current_cluster = self.find_free_cluster(ata).unwrap_or(0);
        if current_cluster == 0 {
            return false; // No free cluster
        }



        info!("Current cluster: {:#X}", current_cluster);
        entry.first_cluster_low = (current_cluster & 0xFFFF) as u16;
        entry.first_cluster_high = ((current_cluster >> 16) & 0xFFFF) as u16;

        while !remaining_data.is_empty() {
            let sector = self.data_start + (current_cluster - 2) * self.boot_sector.sectors_per_cluster as u32;
            let mut buffer = [0u8; 512];
            let len = remaining_data.len().min(512);
            buffer[..len].copy_from_slice(&remaining_data[..len]);
            ata.write_sector8(sector, &buffer);

            remaining_data = &remaining_data[len..];

            let next_cluster = if !remaining_data.is_empty() {
                if let Some(x) = self.find_free_cluster(ata) {
                    x
                } else {
                    return false; // No free cluster
                }
            } else {
                0x0FFFFFFF // End of file marker
            };

            info!("Current cluster: {:#X}, Next cluster: {:#X}", current_cluster, next_cluster);

            self.write_fat_entry(ata, current_cluster, next_cluster);
            current_cluster = next_cluster;

            if current_cluster == 0 {
                return false; // Disk full
            }
        }

        entry.size = data.len() as u32;
        self.update_directory_entry(ata, dir_cluster, entry_index, &entry);

        true
    }

    pub fn read_file(&mut self, ata: &mut AtaPio, dir_cluster: u32, filename: &str) -> Option<Vec<u8>> {
        let entries = self.read_directory(ata, dir_cluster);
        let entry_index = entries.iter().position(|e| {
            let mut name = [0u8; 11];
            name.copy_from_slice(&e.name);
            let mut name = name.to_vec();
            name.retain(|&x| x != 0x20);
            let name = core::str::from_utf8(&name).unwrap_or("").trim();
            // remove filename extension
            let mut parts = filename.splitn(2, '.');
            let base_name = parts.next().unwrap_or("");
            let extension = parts.next().unwrap_or("");
            let filename = base_name.to_string() + extension;
            name == filename.as_str()
        });


        if entry_index.is_none() {
            return None; // File not found
        }

        let entry_index = entry_index.unwrap();
        let entry = entries[entry_index];

        let mut data = Vec::new();
        let mut current_cluster = entry.cluster();

        while current_cluster != 0x0FFFFFFF {
            info!("BREAKPOINT");
            info!("Current cluster: {:#X}", current_cluster);
            let cluster_data = self.read_cluster(ata, current_cluster);
            data.extend_from_slice(&cluster_data);

            current_cluster = self.read_fat_entry(ata, current_cluster);
        }

        Some(data)
    }

    pub fn update_directory_entry(&mut self, ata: &mut AtaPio, dir_cluster: u32, index: usize, entry: &DirectoryEntry) {
        let sector = self.data_start + (dir_cluster - 2) * self.cluster_size;
        let mut buffer = [0u8; 512];
        ata.read_sector8(sector, &mut buffer);
        unsafe {
            core::ptr::copy_nonoverlapping(entry as *const _ as *const u8, buffer.as_mut_ptr().add(index * 32), 32);
        }
        ata.write_sector8(sector, &buffer);
    }

}
