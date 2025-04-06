pub mod ext2;
pub mod fat32;

use alloc::vec::Vec;

pub enum Error {
    FileNotFound,
}

pub trait FileSystem {
    fn read(&self, path: impl AsRef<str>) -> Result<Vec<u8>, Error>;
    fn write(&self, path: impl AsRef<str>, data: &[u8]) -> Result<(), Error>;
    fn delete(&self, path: impl AsRef<str>) -> Result<(), Error>;
    fn rename(&self, old_path: impl AsRef<str>, new_path: impl AsRef<str>) -> Result<(), Error>;
    fn create_directory(&self, path: impl AsRef<str>) -> Result<(), Error>;
    fn create_file(&self, path: impl AsRef<str>) -> Result<(), Error>;
}
