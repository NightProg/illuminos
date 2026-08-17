use crate::fs::{DirectoryInode, FileSystem, Inode};
use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::Mutex;

pub struct RamFile {
    pub data: Vec<u8>,
}

impl Inode for RamFile {
    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> crate::fs::Result<usize> {
        let offset = offset as usize;
        if offset >= self.data.len() {
            return Err("Offset out of bounds".to_string());
        }
        let end = core::cmp::min(offset + buf.len(), self.data.len());
        let read_len = end - offset;
        buf[..read_len].copy_from_slice(&self.data[offset..end]);
        Ok(read_len)
    }

    fn write_at(&mut self, offset: u64, buf: &[u8]) -> crate::fs::Result<usize> {
        let offset = offset as usize;
        let end = offset + buf.len();
        if end > self.data.len() {
            self.data.resize(end, 0);
        }
        self.data[offset..end].copy_from_slice(buf);
        Ok(buf.len())
    }

    fn inner_as_any(&mut self) -> &dyn core::any::Any {
        self
    }

    fn inner_as_any_mut(&mut self) -> &mut dyn core::any::Any {
        self
    }

    fn size(&mut self) -> u64 {
        self.data.len() as u64
    }

    fn kind(&self) -> crate::fs::InodeKind {
        crate::fs::InodeKind::File
    }
}

pub struct RamDir {
    pub children: Mutex<BTreeMap<String, Arc<Mutex<dyn Inode>>>>,
}

impl Inode for RamDir {
    fn read_at(&mut self, _offset: u64, _buf: &mut [u8]) -> crate::fs::Result<usize> {
        Err("Cannot read from a directory".to_string())
    }

    fn write_at(&mut self, _offset: u64, _buf: &[u8]) -> crate::fs::Result<usize> {
        Err("Cannot write to a directory".to_string())
    }

    fn size(&mut self) -> u64 {
        self.children.lock().len() as u64
    }

    fn kind(&self) -> crate::fs::InodeKind {
        crate::fs::InodeKind::Directory
    }

    fn as_directory(&mut self) -> Option<&dyn DirectoryInode> {
        Some(self)
    }

    fn inner_as_any(&mut self) -> &dyn core::any::Any {
        self
    }

    fn inner_as_any_mut(&mut self) -> &mut dyn core::any::Any {
        self
    }
}

impl DirectoryInode for RamDir {
    fn lookup(&self, name: &str) -> crate::fs::Result<Arc<Mutex<dyn Inode>>> {
        self.children
            .lock()
            .get(name)
            .cloned()
            .ok_or("Entry not found".to_string())
    }

    fn mkdir(&self, name: &str) -> crate::fs::Result<()> {
        if self.children.lock().contains_key(name) {
            return Err("Directory already exists".to_string());
        }
        self.children.lock().insert(
            name.to_string(),
            Arc::new(Mutex::new(RamDir {
                children: Mutex::new(BTreeMap::new()),
            })),
        );
        Ok(())
    }

    fn create_file(&self, name: &str) -> crate::fs::Result<()> {
        if self.children.lock().contains_key(name) {
            return Err("File already exists".to_string());
        }
        self.children.lock().insert(
            name.to_string(),
            Arc::new(Mutex::new(RamFile { data: Vec::new() })),
        );
        Ok(())
    }

    fn list_entries(&self) -> crate::fs::Result<Vec<String>> {
        Ok(self.children.lock().keys().cloned().collect())
    }
}

pub struct RamFs {
    root: Arc<Mutex<RamDir>>,
}

impl RamFs {
    pub fn new() -> Self {
        RamFs {
            root: Arc::new(Mutex::new(RamDir {
                children: Mutex::new(BTreeMap::new()),
            })),
        }
    }

    pub fn root(&self) -> Arc<Mutex<dyn Inode>> {
        self.root.clone()
    }
}

impl FileSystem for RamFs {
    fn root(&self) -> Arc<Mutex<dyn Inode>> {
        self.root.clone()
    }

    fn name(&self) -> &'static str {
        "ramfs"
    }
}
