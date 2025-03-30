use alloc::boxed::Box;
use lazy_static::lazy_static;
use spin::Mutex;

use crate::graphic::framebuffer::FrameBuffer;

pub static  GLOBAL_CONTEXT: Mutex<Context> = Mutex::new(Context::none());



pub fn init_global_context(framebuffer: FrameBuffer) {
    GLOBAL_CONTEXT.lock().framebuffer = Some(Box::new(framebuffer));
}

pub fn app_ready() {
    GLOBAL_CONTEXT.lock().is_app_initialized = true;
}

pub struct Context {
    pub framebuffer: Option<Box<FrameBuffer>>,
    pub is_app_initialized: bool,
}

impl Context {
    pub const fn none() -> Self {
        Self { framebuffer: None, is_app_initialized: false }
    }

    pub fn new(framebuffer: FrameBuffer) -> Self {
        Self {
            framebuffer: Some(Box::new(framebuffer)),
            is_app_initialized: false,
        }
    }

    pub fn is_framebuffer_initialized(&self) -> bool {
        self.framebuffer.is_some()
    }
    
    pub fn is_app_initialized(&self) -> bool {
        self.is_app_initialized
    }
    
}