pub mod cache;
pub mod device;
pub mod diskfs;
pub mod illfs;
pub mod ramfs;

use crate::allocator::vma::VMA;
use crate::io::stdin::StdinDevice;
use crate::io::stdout::StdoutDevice;
use crate::println;
use ::illfs::{Error, InOutDevice};
use alloc::boxed::Box;
use alloc::collections::btree_map::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use bitflags::bitflags;
use core::any::Any;
use core::fmt::Debug;
use spin::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InodeKind {
    File,
    Directory,
    Symlink,
    Pipe,
    Device,
}

pub type Result<T> = core::result::Result<T, String>;

pub trait Inode: Send + Sync + Any {
    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> Result<usize>;
    fn write_at(&mut self, offset: u64, buf: &[u8]) -> Result<usize>;
    fn size(&mut self) -> u64;
    fn kind(&self) -> InodeKind;

    fn inner_as_any(&mut self) -> &dyn Any;
    fn inner_as_any_mut(&mut self) -> &mut dyn Any;

    fn mmap(&mut self, vma: &mut VMA) -> Result<()> {
        Err("mmap not supported for this inode".to_string())
    }

    fn as_directory(&mut self) -> Option<&dyn DirectoryInode> {
        None
    }

    fn ioctl(&mut self, _request: u64, _arg: u64) -> Result<u64> {
        Err("ioctl not supported for this inode".to_string())
    }
}


#[repr(u64)]
pub enum IOCtlRequest {
    GetSizeOf = 10
}

impl TryFrom<u64> for IOCtlRequest {
    type Error = String;
    fn try_from(value: u64) -> Result<Self> {
        match value {
            10 => Ok(IOCtlRequest::GetSizeOf),
            _ => Err("Invalid IOCtl request".to_string()),
        }
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct OpenFlags: u64 {
        const READ = 0b0001;
        const WRITE = 0b0010;
        const APPEND = 0b0100;
        const CREATE = 0b1000;
    }
}

pub struct OpenFile {
    pub inode: Arc<Mutex<dyn Inode>>,
    pub flags: OpenFlags,
    inner: Mutex<OpenFileInner>,
}
impl core::fmt::Debug for OpenFile {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("OpenFile")
            .field("inode", &"Arc<Mutex<dyn Inode>>")
            .field("flags", &self.flags)
            .finish()
    }
}

impl OpenFile {
    pub fn new(inode: Arc<Mutex<dyn Inode>>, flags: OpenFlags) -> Self {
        Self {
            inode,
            flags,
            inner: Mutex::new(OpenFileInner { offset: 0 }),
        }
    }

    pub fn offset(&self) -> u64 {
        self.inner.lock().offset
    }

    pub fn set_offset(&self, offset: u64) {
        self.inner.lock().offset = offset;
    }

    pub fn add_offset(&self, delta: u64) {
        self.inner.lock().offset += delta;
    }

    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let offset = self.offset();
        let read_bytes = self
            .inode
            .lock()
            .read_at(offset, buf)
            .map_err(|e| format!("Failed to read from file: {:?}", e))?;
        self.add_offset(read_bytes as u64);
        Ok(read_bytes)
    }

    pub fn write(&mut self, buf: &[u8]) -> Result<usize> {
        let offset = self.offset();
        let written_bytes = self
            .inode
            .lock()
            .write_at(offset, buf)
            .map_err(|e| format!("Failed to write to file: {:?}", e))?;
        self.add_offset(written_bytes as u64);
        Ok(written_bytes)
    }
}
struct OpenFileInner {
    offset: u64,
}

impl InOutDevice for OpenFile {
    fn read(&mut self, offset: u64, buf: &mut [u8]) -> core::result::Result<(), ::illfs::Error> {
        let inner = self.inner.lock();
        let read_bytes = self
            .inode
            .lock()
            .read_at(offset, buf)
            .map_err(|_| ::illfs::Error::DeviceError)?;
        if read_bytes < buf.len() {
            for i in read_bytes..buf.len() {
                buf[i] = 0;
            }
        }
        Ok(())
    }

    fn write(&mut self, offset: u64, buf: &[u8]) -> core::result::Result<(), ::illfs::Error> {
        self.inode
            .lock()
            .write_at(offset, buf)
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
#[derive(Clone, Debug)]
pub struct FdTable {
    inner: Arc<Mutex<FdTableInner>>,
}

impl FdTable {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(FdTableInner {
                next_fd: 0,
                fds: BTreeMap::new(),
            })),
        }
    }

    pub fn new_std() -> Self {
        let mut table = Self::new();
        table.open(Arc::new(Mutex::new(OpenFile::new(
            Arc::new(Mutex::new(StdinDevice)),
            OpenFlags::READ,
        ))));
        table.open(Arc::new(Mutex::new(OpenFile::new(
            Arc::new(Mutex::new(StdoutDevice)),
            OpenFlags::WRITE,
        ))));
        table.open(Arc::new(Mutex::new(OpenFile::new(
            Arc::new(Mutex::new(StdoutDevice)),
            OpenFlags::WRITE,
        ))));
        table
    }

    pub fn len(&self) -> usize {
        let inner = self.inner.lock();
        inner.fds.len()
    }

    pub fn open(&self, file: Arc<Mutex<OpenFile>>) -> usize {
        let mut inner = self.inner.lock();
        let fd = inner.next_fd;
        inner.next_fd += 1;
        inner.fds.insert(fd, file);
        fd
    }

    pub fn get(&self, fd: usize) -> Option<Arc<Mutex<OpenFile>>> {
        let inner = self.inner.lock();
        inner.fds.get(&fd).cloned()
    }

    pub fn close(&self, fd: usize) -> Option<()> {
        let mut inner = self.inner.lock();
        inner.fds.remove(&fd).map(|_| ())
    }
    pub fn dup(&self, fd: usize) -> Option<usize> {
        let inner = self.inner.lock();
        if let Some(file) = inner.fds.get(&fd).cloned() {
            drop(inner);
            Some(self.open(file))
        } else {
            None
        }
    }

    pub fn insert_at(&self, fd: usize, file: Arc<Mutex<OpenFile>>) -> Option<()> {
        let mut inner = self.inner.lock();
        if inner.fds.contains_key(&fd) {
            inner.fds.insert(fd, file);
            Some(())
        } else {
            None
        }
    }
}

#[derive(Clone, Debug)]
struct FdTableInner {
    next_fd: usize,
    fds: BTreeMap<usize, Arc<Mutex<OpenFile>>>,
}

pub trait FileSystem: Send + Sync {
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
        write!(
            f,
            "Mount {{ fs: {}, path: {} }}",
            self.fs.lock().name(),
            self.path
        )
    }
}

#[derive(Default, Debug, Clone)]
pub struct VFS {
    pub mounts: Vec<Mount>,
}

impl VFS {
    pub const fn new() -> Self {
        Self { mounts: Vec::new() }
    }
    pub fn root(&self) -> Arc<Mutex<dyn Inode>> {
        for mount in &self.mounts {
            if mount.path == "/" {
                return mount.fs.lock().root();
            }
        }
        panic!("No root filesystem mounted");
    }

    pub fn mount<F: FileSystem + 'static>(&mut self, path: &str, fs: F) {
        let mount = Mount {
            path: path.to_string(),
            fs: Arc::new(Mutex::new(fs)),
        };

        self.mounts.push(mount);
    }

    pub fn lookup(&self, path: &str) -> Result<Arc<Mutex<dyn Inode>>> {
        let mut current_inode = self.root();
        let mut current_path = String::new();

        let components: Vec<&str> = path.split('/').filter(|c| !c.is_empty()).collect();

        for component in components {
            current_path.push('/');
            current_path.push_str(component);

            if let Some(mount) = self
                .mounts
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
                .ok_or("Not a directory")?
                .lookup(component)?;
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
        s.lock()
            .as_directory()
            .ok_or("Not a directory")?
            .mkdir(components[components.len() - 1])
    }

    pub fn create_file(&self, path: &str) -> Result<()> {
        let components: Vec<&str> = path.split('/').filter(|c| !c.is_empty()).collect();
        if components.is_empty() {
            return Err("Invalid path".to_string());
        }

        let parent_path = &components[..components.len() - 1].join("/");
        let mut s = self.lookup(parent_path)?;
        s.lock()
            .as_directory()
            .ok_or("Not a directory")?
            .create_file(components[components.len() - 1])
    }

    pub fn list_entries(&self, path: &str) -> Result<Vec<String>> {
        let mut inode = self.lookup(path)?;
        inode
            .lock()
            .as_directory()
            .ok_or("Not a directory".to_string())?
            .list_entries()
    }

    pub fn open_file(&self, path: &str, flags: OpenFlags) -> Result<OpenFile> {
        let inode = self.lookup(path)?;
        Ok(OpenFile {
            inode,
            flags,
            inner: Mutex::new(OpenFileInner { offset: 0 }),
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
