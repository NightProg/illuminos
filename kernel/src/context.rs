use alloc::boxed::Box;
use alloc::rc::Rc;
use alloc::string::ToString;
use lazy_static::lazy_static;
use spin::{Mutex, RwLock};

use crate::drivers::disk;
use crate::drivers::disk::Disk;
use crate::drivers::keyboard::KeyboardStream;
use crate::fs;
use crate::graphic::font::{Psf2Font, PsfFont, FONT_DEFAULT};
use crate::graphic::framebuffer::{FrameBuffer, RawFrameBuffer, SwapBuffer};
use crate::graphic::Color;
use crate::io::port::{new_port, Fd};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::cell::RefCell;
use core::marker::PhantomData;
use pc_keyboard::KeyEvent;

type ContextFramebuffer = RawFrameBuffer;

pub static mut GLOBAL_CONTEXT: Context<ContextFramebuffer> = Context::none();

pub fn init_global_context(framebuffer: ContextFramebuffer) {
    unsafe {
        GLOBAL_CONTEXT.framebuffer = Some(Box::new(framebuffer));
    }
}

pub fn app_ready() {
    unsafe {
        GLOBAL_CONTEXT.is_app_initialized = true;
    }
}

#[derive(Clone)]
pub struct Context<F: FrameBuffer> {
    pub framebuffer: Option<Box<F>>,
    pub is_app_initialized: bool,
    pub keyboard_stream: KeyboardStream,
    pub fs: fs::VFS,
    pub open_files: alloc::collections::BTreeMap<usize, crate::io::port::Descriptor>,
}

impl<F: FrameBuffer> core::fmt::Debug for Context<F> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Context")
            .field("is_app_initialized", &self.is_app_initialized)
            .finish()
    }
}

impl<F: FrameBuffer> Context<F> {
    pub const fn none() -> Self {
        Self {
            framebuffer: None,
            is_app_initialized: false,
            keyboard_stream: KeyboardStream::new(),
            fs: fs::VFS::new(),
            open_files: alloc::collections::BTreeMap::new(),
        }
    }

    pub fn init_vfs(&mut self, disks: Vec<Disk>) {
        self.fs.mounts.push(fs::Mount {
            fs: Arc::new(Mutex::new(fs::ramfs::RamFs::new())),
            path: "/".to_string(),
        });

        self.fs.mkdir("/disk");
        self.fs.mkdir("/mnt");
        self.fs.mounts.push(fs::Mount {
            fs: Arc::new(Mutex::new(fs::diskfs::DiskFs { disks })),
            path: "/disk".to_string(),
        });
    }

    pub fn is_framebuffer_initialized(&self) -> bool {
        self.framebuffer.is_some()
    }

    pub fn open(&mut self, path: &str, flags: crate::fs::OpenFlags) -> crate::fs::Result<Fd> {
        let file = self.fs.open_file(path, flags)?;
        let fd = Fd::new();
        self.open_files.insert(
            fd.0,
            crate::io::port::Descriptor::File(Arc::new(Mutex::new(file))),
        );
        Ok(fd)
    }

    pub fn create_port_text_buffer(&'static mut self, fg: Color, bg: Color) -> Option<Fd> {
        if let Some(framebuffer) = self.framebuffer.as_mut() {
            let font = &*FONT_DEFAULT;
            let mut text_buffer =
                crate::graphic::text_buffer::TextBuffer::new(&mut **framebuffer, font, fg, bg);
            let port = new_port(
                move || Vec::new(),
                move |data: &[u8]| {
                    for &b in data {
                        text_buffer.put_char(b as char);
                    }
                },
            );
            Some(port)
        } else {
            None
        }
    }

    pub fn add_key(&mut self, key: KeyEvent) {
        self.keyboard_stream.push_key(key);
    }

    pub fn pop_key(&mut self) -> Option<KeyEvent> {
        self.keyboard_stream.pop()
    }

    pub fn is_app_initialized(&self) -> bool {
        self.is_app_initialized && self.is_framebuffer_initialized()
    }

    pub fn map_framebuffer<Func: FnOnce(&mut F)>(&mut self, f: Func) {
        if let Some(framebuffer) = self.framebuffer.as_mut() {
            f(framebuffer);
        }
    }

    pub fn framebuffer(&self) -> &F {
        self.framebuffer.as_ref().unwrap()
    }

    pub fn framebuffer_mut(&mut self) -> &mut F {
        self.framebuffer.as_mut().unwrap()
    }
}
