use crate::drivers::disk::Disk;
use crate::fs::{DirectoryInode, FileSystem, InodeKind, OpenFile};
use crate::println;
use alloc::boxed::Box;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::cmp::PartialEq;
use illfs::inode::Directory;
use illfs::InOutDevice;
use spin::Mutex;

pub struct IllFS(pub Arc<Mutex<illfs::IllFs<OpenFile>>>);

pub struct IllInode {
    fs: Arc<Mutex<illfs::IllFs<OpenFile>>>,
    inode_id: usize,
    abs_path: String,
}

impl super::Inode for IllInode {
    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> crate::fs::Result<usize> {
        let read = self
            .fs
            .lock()
            .inode_read_at(self.inode_id, offset as usize, buf)
            .map_err(|_| "Failed to read inode data")?;
        Ok(read)
    }

    fn write_at(&mut self, offset: u64, buf: &[u8]) -> crate::fs::Result<usize> {
        let written = self
            .fs
            .lock()
            .inode_write_at(self.inode_id, offset as usize, buf)
            .map_err(|_| "Failed to write inode data")?;

        self.fs
            .lock()
            .sync()
            .map_err(|_| "Failed to sync inode data")?;
        Ok(written)
    }

    fn size(&mut self) -> u64 {
        println!("Getting size of inode {}", self.inode_id);
        self.fs.lock().inode_size(self.inode_id).unwrap() as u64
    }

    fn kind(&self) -> InodeKind {
        let inode = self.fs.lock().inode_table[self.inode_id];
        match inode.inode_type {
            illfs::inode::InodeType::File => InodeKind::File,
            illfs::inode::InodeType::Directory => InodeKind::Directory,
        }
    }

    fn as_directory(&mut self) -> Option<&dyn DirectoryInode> {
        println!("AS_DIR : Getting inode {}", self.inode_id);
        println!("self.kind() = {:?}", self.kind());
        if self.kind() == InodeKind::Directory {
            let dir = self.fs.lock().directory_open(self.inode_id).unwrap();
            let ill_dir = IllDir {
                fs: self.fs.clone(),
                dir,
                abs_path: self.abs_path.clone(),
            };
            // We need to return a reference, so we store it in a Box and leak it.
            let boxed_dir: Box<dyn DirectoryInode> = Box::new(ill_dir);
            let dir_ref: &dyn DirectoryInode = Box::leak(boxed_dir);
            Some(dir_ref)
        } else {
            None
        }
    }
}

pub struct IllDir {
    fs: Arc<Mutex<illfs::IllFs<OpenFile>>>,
    dir: Directory,
    abs_path: String,
}

impl super::Inode for IllDir {
    fn read_at(&mut self, _offset: u64, _buf: &mut [u8]) -> crate::fs::Result<usize> {
        Err("Cannot read from a directory".to_string())
    }

    fn write_at(&mut self, _offset: u64, _buf: &[u8]) -> crate::fs::Result<usize> {
        Err("Cannot write to a directory".to_string())
    }

    fn size(&mut self) -> u64 {
        0
    }

    fn kind(&self) -> InodeKind {
        InodeKind::Directory
    }

    fn as_directory(&mut self) -> Option<&dyn DirectoryInode> {
        Some(self)
    }
}

impl DirectoryInode for IllDir {
    fn lookup(&self, name: &str) -> crate::fs::Result<Arc<Mutex<dyn super::Inode>>> {
        for entry in self.dir.entries.iter() {
            if entry.name_as_str() == name {
                let inode = IllInode {
                    fs: self.fs.clone(),
                    inode_id: entry.inode as usize,
                    abs_path: self.abs_path.clone() + "/" + name,
                };
                return Ok(Arc::new(Mutex::new(inode)));
            }
        }
        Err(format!("Entry '{}' not found in directory", name))
    }

    fn mkdir(&self, name: &str) -> crate::fs::Result<()> {
        let mut path = self.abs_path.clone() + "/";
        path.push_str(name);
        self.fs
            .lock()
            .create_directory(&*path)
            .as_ref()
            .map_err(|e| format!("Failed to create directory in directory: {:?}", e))?;
        Ok(())
    }

    fn create_file(&self, name: &str) -> crate::fs::Result<()> {
        let mut path = self.abs_path.clone() + "/";
        path.push_str(name);
        self.fs
            .lock()
            .create_file(&*path)
            .map_err(|e| format!("Failed to create file in directory: {:?}", e))?;
        Ok(())
    }

    fn list_entries(&self) -> crate::fs::Result<Vec<String>> {
        let mut entries = Vec::new();
        for entry in self.dir.entries.iter() {
            entries.push(entry.name_as_str().to_string());
        }
        Ok(entries)
    }
}

impl FileSystem for IllFS {
    fn root(&self) -> Arc<Mutex<dyn super::Inode>> {
        let root_inode = IllDir {
            fs: self.0.clone(),
            dir: self.0.lock().directory_open(1).unwrap(),
            abs_path: "/".to_string(),
        };
        Arc::new(Mutex::new(root_inode))
    }

    fn name(&self) -> &'static str {
        "illfs"
    }

    fn umount(&self) -> crate::fs::Result<()> {
        self.0
            .lock()
            .sync()
            .map_err(|e| format!("Failed to umount IllFS: {:?}", e))?;
        Ok(())
    }
}
