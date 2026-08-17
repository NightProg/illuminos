use alloc::boxed::Box;
use alloc::rc::Rc;
use alloc::string::ToString;
use lazy_static::lazy_static;
use spin::{Mutex, RwLock};

use crate::drivers::disk;
use crate::drivers::disk::Disk;
use crate::drivers::keyboard::{KEYBOARD, KeyboardStream};
use crate::fs::device::DevFs;
use crate::fs::device::tty::TtyInode;
use crate::graphic::Color;
use crate::graphic::font::{FONT_DEFAULT, Psf2Font, PsfFont};
use crate::graphic::framebuffer::RawFrameBuffer;
use crate::sync::mutex::TimeoutMutex;
use crate::tty::session::Sessions;
use crate::{fs, println_serial};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::cell::RefCell;
use core::marker::PhantomData;
use pc_keyboard::KeyEvent;

lazy_static! {
    pub static ref GLOBAL_CONTEXT: Context = Context::none();
}
pub fn init_global_context(framebuffer: RawFrameBuffer) {
    *GLOBAL_CONTEXT.framebuffer.lock() = framebuffer;
}

pub struct Context {
    pub framebuffer: Arc<TimeoutMutex<RawFrameBuffer>>,
    pub keyboard_stream: Mutex<KeyboardStream>,
    pub fs: Mutex<fs::VFS>,
    pub sessions: spin::RwLock<Sessions>,
}

impl core::fmt::Debug for Context {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Context").finish()
    }
}

impl Context {
    pub fn none() -> Self {
        Self {
            framebuffer: Arc::new(TimeoutMutex::new(RawFrameBuffer::none())),
            keyboard_stream: Mutex::new(KeyboardStream::new()),
            fs: Mutex::new(fs::VFS::new()),
            sessions: RwLock::new(Sessions::new()),
        }
    }

    pub fn init_vfs(&self, disks: Vec<Disk>) {
        let mut fs_lock = self.fs.lock();
        fs_lock.mounts.push(fs::Mount {
            fs: Arc::new(Mutex::new(fs::ramfs::RamFs::new())),
            path: "/".to_string(),
        });

        let fb = self.framebuffer.lock();

        let mut devfs = DevFs::new();
        devfs.add_device("fb0", fs::device::fb::FramebufferInode::new(*fb));
        devfs.add_device("null", fs::device::null::NullInode);
        devfs.add_device("tty", TtyInode);

        fs_lock.mkdir("/disk");
        fs_lock.mkdir("/mnt");
        fs_lock.mkdir("/dev");
        fs_lock.mounts.push(fs::Mount {
            fs: Arc::new(Mutex::new(fs::diskfs::DiskFs { disks })),
            path: "/disk".to_string(),
        });
        fs_lock.mounts.push(fs::Mount {
            fs: Arc::new(Mutex::new(devfs)),
            path: "/dev".to_string(),
        });
    }

    pub fn add_key(&self, key: KeyEvent) {
        println_serial!("key: {:?}", key);

        self.sessions.read().current().tty.add_key(key.clone());

        self.keyboard_stream.lock().push_key(key);
    }

    pub fn pop_key(&self) -> Option<KeyEvent> {
        self.keyboard_stream.lock().pop()
    }
}
