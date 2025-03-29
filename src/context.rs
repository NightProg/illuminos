use alloc::boxed::Box;
use lazy_static::lazy_static;
use spin::Mutex;

use crate::graphic::framebuffer::FrameBuffer;

pub static  GLOBAL_CONTEXT: Mutex<Context> = Mutex::new(Context::none());



pub fn init_global_context(framebuffer: FrameBuffer) {
    GLOBAL_CONTEXT.lock().framebuffer = Some(Box::new(framebuffer));
}

pub struct Context {
    pub framebuffer: Option<Box<FrameBuffer>>,
}

impl Context {
    pub const fn none() -> Self {
        Self { framebuffer: None }
    }

    pub fn new(framebuffer: FrameBuffer) -> Self {
        Self {
            framebuffer: Some(Box::new(framebuffer)),
        }
    }

    pub fn is_context_initialized(&self) -> bool {
        self.framebuffer.is_some()
    }
}