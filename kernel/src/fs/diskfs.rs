use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use illfs::InOutDevice;
use spin::Mutex;
use crate::drivers::disk::Disk;
use crate::fs::{DirectoryInode, Inode};


#[derive(Clone)]
pub struct DiskFs {
    pub disks: Vec<Disk>,
}

pub struct DiskFsInode {
    pub fs: DiskFs,
    pub disk_index: usize,
}

impl Inode for DiskFsInode {
    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> crate::fs::Result<usize> {
        self.fs.disks[self.disk_index].read(offset, buf)
            .map_err(|_| "Failed to read from disk device inode")?;
        Ok(buf.len())
    }

    fn write_at(&mut self, offset: u64, buf: &[u8]) -> crate::fs::Result<usize> {
        self.fs.disks[self.disk_index].write(offset, buf)
            .map_err(|_| "Failed to write to disk device inode")?;
        Ok(buf.len())
    }
    
    fn inner_as_any(&mut self) -> &dyn core::any::Any {
        self
    }
    
    fn inner_as_any_mut(&mut self) -> &mut dyn core::any::Any {
        self
    }

    fn size(&mut self) -> u64 {
        self.fs.disks[self.disk_index].size()
    }

    fn kind(&self) -> crate::fs::InodeKind {
        crate::fs::InodeKind::File
    }
}


#[derive(Clone)]
pub struct DiskFsRoot {
    pub fs: DiskFs,
}

impl Inode for DiskFsRoot {
    fn read_at(&mut self, _offset: u64, _buf: &mut [u8]) -> crate::fs::Result<usize> {
        Err("Cannot read from disk filesystem root".to_string())
    }

    fn write_at(&mut self, _offset: u64, _buf: &[u8]) -> crate::fs::Result<usize> {
        Err("Cannot write to disk filesystem root".to_string())
    }

    fn size(&mut self) -> u64 {
        self.fs.disks.len() as u64
    }

    fn kind(&self) -> crate::fs::InodeKind {
        crate::fs::InodeKind::Directory
    }
    
    fn inner_as_any(&mut self) -> &dyn core::any::Any {
        self
    }
    
    fn inner_as_any_mut(&mut self) -> &mut dyn core::any::Any {
        self
    }

    fn as_directory(&mut self) -> Option<&dyn DirectoryInode> {
        Some(self)
    }
}

impl DirectoryInode for DiskFsRoot {
    fn lookup(&self, name: &str) -> crate::fs::Result<Arc<Mutex<dyn Inode>>> {
        let disk_index: usize = name.parse().map_err(|_| "Invalid disk index".to_string())?;
        if disk_index >= self.fs.disks.len() {
            return Err("Disk index out of bounds".to_string());
        }
        Ok(Arc::new(Mutex::new(DiskFsInode {
            fs: self.fs.clone(),
            disk_index,
        })))
    }

    fn mkdir(&self, name: &str) -> crate::fs::Result<()> {
        Err("Cannot create directories in disk filesystem root".to_string())
    }

    fn create_file(&self, name: &str) -> crate::fs::Result<()> {
        Err("Cannot create files in disk filesystem root".to_string())
    }

    fn list_entries(&self) -> crate::fs::Result<Vec<String>> {
        let mut entries = Vec::new();
        for i in 0..self.fs.disks.len() {
            entries.push(i.to_string());
        }
        Ok(entries)
    }
}


impl super::FileSystem for DiskFs {
    fn root(&self) -> Arc<Mutex<dyn Inode>> {
        Arc::new(Mutex::new(DiskFsRoot {
            fs: self.clone(),
        }))
    }

    fn name(&self) -> &'static str {
        "diskfs"
    }
}