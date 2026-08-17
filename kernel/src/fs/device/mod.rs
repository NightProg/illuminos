use alloc::{
    collections::btree_map::BTreeMap,
    format,
    string::{String, ToString},
    sync::Arc,
};
use spin::Mutex;

use crate::fs::{DirectoryInode, FileSystem, Inode};

pub mod fb;
pub mod null;
pub mod tty;

pub struct DevFsRoot {
    nodes: BTreeMap<String, Arc<Mutex<dyn Inode>>>,
}

impl Inode for DevFsRoot {
    fn read_at(&mut self, _offset: u64, _buf: &mut [u8]) -> crate::fs::Result<usize> {
        Err("cannot read from a directory".to_string())
    }

    fn write_at(&mut self, _offset: u64, _buf: &[u8]) -> crate::fs::Result<usize> {
        Err("cannot write to a directory".to_string())
    }

    fn size(&mut self) -> u64 {
        0
    }

    fn inner_as_any(&mut self) -> &dyn core::any::Any {
        self
    }

    fn inner_as_any_mut(&mut self) -> &mut dyn core::any::Any {
        self
    }

    fn kind(&self) -> crate::fs::InodeKind {
        crate::fs::InodeKind::Directory
    }

    fn as_directory(&mut self) -> Option<&dyn DirectoryInode> {
        Some(self)
    }
}

impl DirectoryInode for DevFsRoot {
    fn create_file(&self, name: &str) -> super::Result<()> {
        Err("cannot create a file".to_string())
    }
    fn list_entries(&self) -> super::Result<alloc::vec::Vec<String>> {
        Ok(self.nodes.keys().cloned().collect())
    }
    fn mkdir(&self, name: &str) -> super::Result<()> {
        Err("cannot create a dir".to_string())
    }

    fn lookup(&self, name: &str) -> super::Result<Arc<Mutex<dyn Inode>>> {
        self.nodes
            .get(name)
            .cloned()
            .ok_or_else(|| format!("device '{}' not found", name))
    }
}

pub struct DevFs {
    root: Arc<Mutex<DevFsRoot>>,
}

impl DevFs {
    pub fn new() -> Self {
        DevFs {
            root: Arc::new(Mutex::new(DevFsRoot {
                nodes: BTreeMap::new(),
            })),
        }
    }

    pub fn add_device<I: Inode + 'static>(&self, name: &str, inode: I) {
        self.root
            .lock()
            .nodes
            .insert(name.to_string(), Arc::new(Mutex::new(inode)));
    }

    pub fn root(&self) -> Arc<Mutex<dyn Inode>> {
        self.root.clone()
    }
}

impl FileSystem for DevFs {
    fn name(&self) -> &'static str {
        "devfs"
    }

    fn umount(&self) -> super::Result<()> {
        Ok(())
    }

    fn root(&self) -> Arc<Mutex<dyn Inode>> {
        self.root()
    }
}
