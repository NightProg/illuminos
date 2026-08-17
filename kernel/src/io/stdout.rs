use alloc::collections::VecDeque;
use spin::Mutex;

use crate::fs::Inode;

pub static STDOUT_BUFFER: Mutex<VecDeque<u8>> = Mutex::new(VecDeque::new());

pub fn write(buf: &[u8]) {
    STDOUT_BUFFER.lock().extend(buf.iter());
}

pub fn flush(out: &mut impl core::fmt::Write) {
    let mut temp_buf = VecDeque::new();
    if let Some(mut lock) = STDOUT_BUFFER.try_lock() {
        core::mem::swap(&mut *lock, &mut temp_buf);
    }

    while let Some(b) = temp_buf.pop_front() {
        let c = b as char;
        let _ = out.write_char(c);
    }
}

pub struct StdoutDevice;

impl Inode for StdoutDevice {
    fn as_directory(&mut self) -> Option<&dyn crate::fs::DirectoryInode> {
        None
    }

    fn kind(&self) -> crate::fs::InodeKind {
        crate::fs::InodeKind::Pipe
    }

    fn inner_as_any(&mut self) -> &dyn core::any::Any {
        self
    }

    fn inner_as_any_mut(&mut self) -> &mut dyn core::any::Any {
        self
    }

    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> crate::fs::Result<usize> {
        Ok(0)
    }

    fn size(&mut self) -> u64 {
        STDOUT_BUFFER.lock().len() as u64
    }

    fn write_at(&mut self, offset: u64, buf: &[u8]) -> crate::fs::Result<usize> {
        write(buf);
        Ok(buf.len())
    }
}
