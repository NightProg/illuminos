pub mod paging;
pub mod memory;

use core::ops::{Deref, DerefMut};

// 0: its a memory size in octets
#[derive(Debug, Clone, Copy)]
pub struct MemSize(u64);

impl MemSize {

    pub fn new(size: u64) -> Self {
        MemSize(size)
    }
    
    pub fn ko(&self) -> u64 {
        self.0 / 1024
    }

    pub fn mo(&self) -> u64 {
        self.ko() / 1024
    }

    pub fn go(&self) -> u64 {
        self.mo() / 1024
    }


}

impl Deref for MemSize {
    type Target = u64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for MemSize {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}


