pub mod ext2;
pub mod fat32;

use alloc::{
    string::{String, ToString},
    vec::Vec,
};

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

pub trait FileSystem {
    fn read(&mut self, path: Path) -> Result<Vec<u8>, Error>;
    fn write(&mut self, path: Path, data: &[u8]) -> Result<(), Error>;
    fn delete(&mut self, path: Path) -> Result<(), Error>;
    fn rename(&mut self, old_path: Path, new_path: Path) -> Result<(), Error>;
    fn create_directory(&mut self, path: Path) -> Result<(), Error>;
    fn create_file(&mut self, path: Path) -> Result<(), Error>;
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
