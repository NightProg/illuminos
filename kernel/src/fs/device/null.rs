use crate::fs::Inode;

pub struct NullInode;

impl Inode for NullInode {
    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> crate::fs::Result<usize> {
        Ok(0)
    }

    fn write_at(&mut self, offset: u64, buf: &[u8]) -> crate::fs::Result<usize> {
        Ok(buf.len())
    }

    fn size(&mut self) -> u64 {
        0
    }

    fn kind(&self) -> crate::fs::InodeKind {
        crate::fs::InodeKind::Device
    }

    fn inner_as_any(&mut self) -> &dyn core::any::Any {
        self
    }

    fn inner_as_any_mut(&mut self) -> &mut dyn core::any::Any {
        self
    }
}
