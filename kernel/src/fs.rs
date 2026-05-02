pub mod ramfs;
pub mod illfs;
pub mod diskfs;

use alloc::boxed::Box;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::fmt::Debug;
use ::illfs::{Error, InOutDevice};
use bitflags::bitflags;
use spin::Mutex;
use crate::println;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InodeKind {
    File,
    Directory,
    Symlink,
}

pub type Result<T> = core::result::Result<T, String>;

pub trait Inode {
    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> Result<usize>;
    fn write_at(&mut self, offset: u64, buf: &[u8]) -> Result<usize>;
    fn size(&mut self) -> u64;
    fn kind(&self) -> InodeKind;

    fn as_directory(&mut self) -> Option<&dyn DirectoryInode> {
        None
    }
}

bitflags! {
    pub struct OpenFlags: u64 {
        const READ = 0b0001;
        const WRITE = 0b0010;
        const APPEND = 0b0100;
        const CREATE = 0b1000;
    }
}

pub struct OpenFile {
    pub inode: Arc<Mutex<dyn Inode>>,
    pub offset: u64,
    pub flags: OpenFlags,
}

impl InOutDevice for OpenFile {
    fn read(&mut self, offset: u64, buf: &mut [u8]) -> core::result::Result<(), ::illfs::Error> {
        let read_bytes = self.inode.lock().read_at(offset, buf)
            .map_err(|_| ::illfs::Error::DeviceError)?;
        if read_bytes < buf.len() {
            for i in read_bytes..buf.len() {
                buf[i] = 0;
            }
        }
        Ok(())
    }

    fn write(&mut self, offset: u64, buf: &[u8]) -> core::result::Result<(), ::illfs::Error> {
        self.inode.lock().write_at(offset, buf)
            .map_err(|_| ::illfs::Error::DeviceError)?;
        Ok(())
    }

    fn size(&self) -> u64 {
        self.inode.lock().size()
    }

    fn close(&self) -> core::result::Result<(), Error> {
        Ok(())
    }
}

pub trait FileSystem  {
    fn root(&self) -> Arc<Mutex<dyn Inode>>;
    fn name(&self) -> &'static str;

    fn umount(&self) -> Result<()> {
        Ok(())
    }
}

pub trait DirectoryInode: Inode {
    fn lookup(&self, name: &str) -> Result<Arc<Mutex<dyn Inode>>>;
    fn mkdir(&self, name: &str) -> Result<()>;
    fn create_file(&self, name: &str) -> Result<()>;

    fn list_entries(&self) -> Result<Vec<String>> {
        Err("list_entries not implemented".to_string())
    }
}


#[derive(Clone)]
pub struct Mount {
    pub fs: Arc<Mutex<dyn FileSystem>>,
    pub path: String,
}


impl Debug for Mount {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Mount {{ fs: {}, path: {} }}", self.fs.lock().name(), self.path)
    }
}


#[derive(Default, Debug, Clone)]
pub struct VFS {
    pub mounts: Vec<Mount>,
}

impl VFS {
    pub const fn new() -> Self {
        Self {
            mounts: Vec::new(),
        }
    }
    pub fn root(&self) -> Arc<Mutex<dyn Inode>> {
        for mount in &self.mounts {
            if mount.path == "/" {
                return mount.fs.lock().root();
            }
        }
        panic!("No root filesystem mounted");
    }

    pub fn lookup(&self, path: &str) -> Result<Arc<Mutex<dyn Inode>>> {
        let mut current_inode = self.root();
        let mut current_path = String::new();

        let components: Vec<&str> = path.split('/').filter(|c| !c.is_empty()).collect();

        for component in components {
            current_path.push('/');
            current_path.push_str(component);

            if let Some(mount) = self.mounts
                .iter()
                .filter(|m| m.path == current_path)
                .max_by_key(|m| m.path.len())
            {
                current_inode = mount.fs.lock().root();
                continue;
            }

            let dir = current_inode
                .lock()
                .as_directory()
                .ok_or("Not a directory")?;

            current_inode = current_inode
                .clone()
                .lock()
                .as_directory()
                .ok_or("Not a directory")?.lookup(component)?;
        }

        Ok(current_inode)
    }


    pub fn mkdir(&self, path: &str) -> Result<()> {
        let components: Vec<&str> = path.split('/').collect();
        println!("Creating directory: {:?}", components);
        if components.is_empty() {
            return Err("Invalid path".to_string());
        }

        let parent_path = &components[..components.len() - 1].join("/");
        println!("Creating directory at parent path: {}", parent_path);
        let mut s = self.lookup(parent_path)?;
        s.lock().as_directory().ok_or("Not a directory")?.mkdir(components[components.len() - 1])
    }

    pub fn create_file(&self, path: &str) -> Result<()> {
        let components: Vec<&str> = path.split('/').filter(|c| !c.is_empty()).collect();
        if components.is_empty() {
            return Err("Invalid path".to_string());
        }

        let parent_path = &components[..components.len() - 1].join("/");
        let mut s = self.lookup(parent_path)?;
        s.lock().as_directory().ok_or("Not a directory")?.create_file(components[components.len() - 1])
    }

    pub fn list_entries(&self, path: &str) -> Result<Vec<String>> {
        let mut inode = self.lookup(path)?;
        inode.lock().as_directory().ok_or("Not a directory".to_string())?.list_entries()
    }

    pub fn open_file(&self, path: &str, flags: OpenFlags) -> Result<OpenFile> {
        let inode = self.lookup(path)?;
        Ok(OpenFile {
            inode,
            offset: 0,
            flags,
        })
    }

    pub fn umount(&mut self, path: &str) -> Result<()> {
        if let Some(index) = self.mounts.iter().position(|m| m.path == path) {
            // call umount on the filesystem
            self.mounts[index].fs.lock().umount()?;
            self.mounts.remove(index);
            Ok(())
        } else {
            Err("Mount point not found".to_string())
        }
    }
}