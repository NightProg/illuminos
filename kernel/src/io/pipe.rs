use alloc::{collections::vec_deque::VecDeque, string::ToString, sync::Arc};
use spin::Mutex;

use crate::fs::Inode;

pub struct Pipe {
    buffer: Mutex<VecDeque<u8>>,
}

pub struct PipeWriter {
    inner: Arc<Mutex<VecDeque<u8>>>,
}
pub struct PipeReader {
    inner: Arc<Mutex<VecDeque<u8>>>,
}

impl Pipe {
    pub fn new() -> (PipeReader, PipeWriter) {
        let buffer = Arc::new(Mutex::new(VecDeque::new()));
        (
            PipeReader {
                inner: buffer.clone(),
            },
            PipeWriter {
                inner: buffer.clone(),
            },
        )
    }
}

impl Inode for PipeReader {
    fn read_at(&mut self, _offset: u64, buf: &mut [u8]) -> crate::fs::Result<usize> {
        let mut buffer = self.inner.lock();
        let mut bytes_read = 0;
        while bytes_read < buf.len() && !buffer.is_empty() {
            if let Some(byte) = buffer.pop_front() {
                buf[bytes_read] = byte;
                bytes_read += 1;
            }
        }
        Ok(bytes_read)
    }

    fn write_at(&mut self, _offset: u64, _buf: &[u8]) -> crate::fs::Result<usize> {
        Err("Cannot write to a pipe reader".to_string())
    }

    fn inner_as_any(&mut self) -> &dyn core::any::Any {
        self
    }

    fn inner_as_any_mut(&mut self) -> &mut dyn core::any::Any {
        self
    }

    fn size(&mut self) -> u64 {
        0
    }

    fn kind(&self) -> crate::fs::InodeKind {
        crate::fs::InodeKind::Pipe
    }
}

impl Inode for PipeWriter {
    fn read_at(&mut self, _offset: u64, _buf: &mut [u8]) -> crate::fs::Result<usize> {
        Err("Cannot read from a pipe writer".to_string())
    }

    fn write_at(&mut self, _offset: u64, buf: &[u8]) -> crate::fs::Result<usize> {
        let mut buffer = self.inner.lock();
        for &byte in buf {
            buffer.push_back(byte);
        }
        Ok(buf.len())
    }

    fn size(&mut self) -> u64 {
        0
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
}
