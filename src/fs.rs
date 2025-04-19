pub mod ext2;
pub mod fat32;

use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use bitflags::bitflags;
use crate::io::port::*;

use core::cell::RefCell;
use alloc::rc::Rc;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Error {
    FileNotFound,
    CantUseRelPath,
    RootDirNotFound,
    NotADirectory,
    NotAFile,
    NotEnoughSpace,
    KernelError(String),
}

pub trait FileSystem where Self: 'static {
    fn read(&mut self, path: Path) -> Result<Vec<u8>, Error>;
    fn write(&mut self, path: Path, data: &[u8]) -> Result<(), Error>;
    fn delete(&mut self, path: Path) -> Result<(), Error>;
    fn rename(&mut self, old_path: Path, new_path: Path) -> Result<(), Error>;
    fn create_directory(&mut self, path: Path) -> Result<(), Error>;
    fn create_file(&mut self, path: Path) -> Result<(), Error>;
    fn is_exist(&mut self, path: Path) -> bool {
        return self.read(path).is_ok();
    }

    fn open(fs: Rc<RefCell<Self>>, path: &'static str, flags: OpenFlags) -> Result<File, Error> {
        let fs_clone = Rc::clone(&fs);

        if !fs_clone.borrow_mut().is_exist(Path::new(path)) {
            return Err(Error::FileNotFound);
        }

        let read_clone = Rc::clone(&fs);
        let write_clone = Rc::clone(&fs);

        Ok(
            File {
                fd: new_port(move || read_clone.borrow_mut().read(Path::new(path)).unwrap(),
                             move |buf| write_clone.borrow_mut().write(Path::new(path), buf).unwrap()),
                flag: flags
            }
        )

    }
}


#[derive(Debug)]
pub struct File {
    fd: Fd,
    flag: OpenFlags
}

impl File {
    pub fn read(&self) -> Option<Vec<u8>> {
        if !self.flag.contains(OpenFlags::READ) {
            return None;
        }

        self.fd.read()
    }

    #[must_use]
    pub fn write(&self, buf: &[u8]) -> Option<()> {
        if !self.flag.contains(OpenFlags::WRITE) {
            return None;
        }

        self.fd.write(buf);

        Some(())
    }
}





bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct OpenFlags: u64 {
        const READ = 1;
        const WRITE = 1 << 1;
        const APPEND = 1 << 2;
        const BINARY = 1 << 3;
    }
}







#[derive(Debug, Clone)]
pub struct Path {
    parts: Vec<String>,
    absolute: bool,
}

impl Path {
    pub fn new(path: &str) -> Self {
        let absolute = path.starts_with('/');
        let parts = path
            .split('/')
            .filter(|s| !s.is_empty() && *s != ".")
            .map(|s| s.to_string())
            .collect();

        Path { parts, absolute }
    }

    pub fn is_absolute(&self) -> bool {
        self.absolute
    }

    pub fn is_relative(&self) -> bool {
        !self.absolute
    }

    pub fn components(&self) -> &[String] {
        &self.parts
    }

    pub fn join(&self, segment: &str) -> Path {
        let mut parts = self.parts.clone();

        for part in segment.split('/') {
            match part {
                "" | "." => {}
                ".." => {
                    parts.pop();
                }
                s => {
                    parts.push(s.to_string());
                }
            }
        }

        Path {
            parts,
            absolute: self.absolute,
        }
    }

    pub fn to_string(&self) -> String {
        let mut result = String::new();
        if self.absolute {
            result.push('/');
        }

        result.push_str(&self.parts.join("/"));
        result
    }
}
