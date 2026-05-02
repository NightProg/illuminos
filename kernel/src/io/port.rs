use crate::warn;
use alloc::boxed::Box;
use alloc::rc::Rc;
use alloc::vec::Vec;
use core::cell::RefCell;
use core::sync::atomic::Ordering;
use core::{fmt::Write, sync::atomic::AtomicUsize};
use lazy_static::lazy_static;
use spin::Mutex;

pub static FD_COUNTER: AtomicUsize = AtomicUsize::new(3); // 1 and 2 are reserved for stdout

pub static PORTS: Mutex<Ports> = Mutex::new(Ports::new());

pub static STDOUT: AtomicFd = AtomicFd::new();

#[derive(Clone)]
pub enum Descriptor {
    Port(Port),
    File(alloc::sync::Arc<spin::Mutex<crate::fs::OpenFile>>),
}

pub fn new_port<R, W>(read: R, write: W) -> Fd
where
    R: FnMut() -> Vec<u8> + 'static,
    W: FnMut(&[u8]) + 'static,
{
    let fd = FD_COUNTER.fetch_add(1, Ordering::Relaxed);
    PORTS.lock().add_port(Port::new(read, write, fd));
    Fd(fd)
}

pub struct Ports {
    ports: Vec<Port>,
}

impl Ports {
    pub const fn new() -> Self {
        Ports { ports: Vec::new() }
    }

    pub fn new_port<R, W>(read: R, write: W) -> Fd
    where
        R: FnMut() -> Vec<u8> + 'static,
        W: FnMut(&[u8]) + 'static,
    {
        let fd = FD_COUNTER.fetch_add(1, Ordering::Relaxed);
        PORTS.lock().add_port(Port::new(read, write, fd));
        Fd(fd)
    }

    pub fn add_port(&mut self, port: Port) {
        self.ports.push(port);
    }

    pub fn get_port(&self, fd: usize) -> Option<&Port> {
        self.ports.iter().find(|port| port.fd() == fd)
    }

    pub fn remove_port(&mut self, fd: usize) -> Option<Port> {
        self.ports
            .iter()
            .position(|port| port.fd() == fd)
            .map(|pos| self.ports.remove(pos))
    }

    pub fn len(&self) -> usize {
        self.ports.len()
    }
}

#[derive(Clone)]
pub struct Port {
    read: Rc<RefCell<dyn FnMut() -> Vec<u8>>>,
    write: Rc<RefCell<dyn FnMut(&[u8])>>,
    fd: usize,
}

unsafe impl Send for Port {}
unsafe impl Sync for Port {}

impl core::fmt::Debug for Port {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Port").field("fd", &self.fd).finish()
    }
}

impl Port {
    pub fn from_fd(fd: usize) -> Option<Self> {
        PORTS.lock().get_port(fd).cloned()
    }
    fn new<R, W>(read: R, write: W, fd: usize) -> Self
    where
        R: FnMut() -> Vec<u8> + 'static,
        W: FnMut(&[u8]) + 'static,
    {
        Port {
            read: Rc::new(RefCell::new(read)),
            write: Rc::new(RefCell::new(write)),
            fd,
        }
    }

    pub fn fd(&self) -> usize {
        self.fd
    }

    pub fn write(&mut self, buf: &[u8]) {
        (self.write.borrow_mut())(buf);
    }

    pub fn read(&mut self) -> Vec<u8> {
        (self.read.borrow_mut())()
    }
}

impl Write for Port {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        (self.write.borrow_mut())(s.as_bytes());
        Ok(())
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Fd(pub usize);

impl Fd {
    pub fn new() -> Self {
        let fd = FD_COUNTER.fetch_add(1, Ordering::Relaxed);
        Fd(fd)
    }

    pub fn fd(&self) -> usize {
        self.0
    }

    pub fn write(&self, buf: &[u8]) {
        let desc = unsafe {
            crate::context::GLOBAL_CONTEXT
                .open_files
                .get(&self.0)
                .cloned()
        };
        match desc {
            Some(Descriptor::Port(mut port)) => {
                port.write(buf);
            }
            Some(Descriptor::File(file)) => {
                use illfs::InOutDevice;
                let mut f = file.lock();
                let offset = f.offset;
                let _ = f.write(offset, buf);
                f.offset += buf.len() as u64;
            }
            None => {
                let port = Port::from_fd(self.0);
                if let Some(mut port) = port {
                    port.write(buf);
                } else {
                    warn!("no port found");
                }
            }
        }
    }

    pub fn read(&self) -> Option<Vec<u8>> {
        let desc = unsafe {
            crate::context::GLOBAL_CONTEXT
                .open_files
                .get(&self.0)
                .cloned()
        };
        match desc {
            Some(Descriptor::Port(mut port)) => Some(port.read()),
            Some(Descriptor::File(file)) => {
                use illfs::InOutDevice;
                let mut f = file.lock();
                let size = f.size();
                let offset = f.offset;
                if offset >= size {
                    return Some(Vec::new());
                }
                let mut buf = alloc::vec![0; (size - offset) as usize];
                let _ = f.read(offset, &mut buf);
                f.offset += buf.len() as u64;
                Some(buf)
            }
            None => {
                let port = Port::from_fd(self.0);
                if let Some(mut port) = port {
                    Some(port.read())
                } else {
                    None
                }
            }
        }
    }
}

impl Write for Fd {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let port = Port::from_fd(self.0);
        if let Some(mut port) = port {
            port.write_str(s);
            Ok(())
        } else {
            Err(core::fmt::Error)
        }
    }
}

pub struct AtomicFd(AtomicUsize);

impl AtomicFd {
    pub const fn new() -> Self {
        AtomicFd(AtomicUsize::new(0))
    }

    pub fn set(&self, fd: usize) {
        self.0.store(fd, Ordering::Relaxed);
    }

    pub fn fd(&self) -> usize {
        self.0.load(Ordering::Relaxed)
    }
    pub fn write_string(&self, s: &str) {
        let port = Port::from_fd(self.fd());
        if let Some(mut port) = port {
            port.write_str(s).ok();
        } else {
            warn!("no port found");
        }
    }
}

impl Write for AtomicFd {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let port = Port::from_fd(self.fd());
        if let Some(mut port) = port {
            port.write_str(s);
            Ok(())
        } else {
            Err(core::fmt::Error)
        }
    }
}

pub struct AtomicPort {
    port: AtomicFd,
}

impl AtomicPort {
    pub fn new() -> Self {
        AtomicPort {
            port: AtomicFd::new(),
        }
    }

    pub fn fd(&self) -> usize {
        self.port.fd()
    }
}

impl Write for AtomicPort {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let port = Port::from_fd(self.fd());
        if let Some(mut port) = port {
            port.write_str(s);
            Ok(())
        } else {
            Err(core::fmt::Error)
        }
    }
}
