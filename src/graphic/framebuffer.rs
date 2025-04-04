use core::{alloc::Layout, ops::{Deref, DerefMut}};

use alloc::alloc::alloc;
use alloc::boxed::Box;
use spin::Mutex;
use core::fmt::Debug;
use bootloader_api::info::{FrameBuffer as BootFrameBuffer, FrameBufferInfo, PixelFormat};

use crate::{context::Context, error, info};
use crate::context::GLOBAL_CONTEXT;
use super::{font::FONT_DEFAULT, widget::Widget};



#[derive(Debug, Clone)]
pub struct FrameBuffer {
    addr: u64,
    info: FrameBufferInfo,
}

unsafe impl Send for FrameBuffer {}
unsafe impl Sync for FrameBuffer {}

impl FrameBuffer {
    pub unsafe fn create_from_raw_addr(addr: u64, info: FrameBufferInfo) -> Self {
        let size = info.width * info.height * info.bytes_per_pixel;
        info!("Creating framebuffer from raw address: size: {}, width: {}, height: {}, bytes_per_pixel: {}", size, info.width, info.height, info.bytes_per_pixel);
        FrameBuffer {
            addr,
            info
        }
    }

    pub unsafe fn alloc(info: FrameBufferInfo) -> Self {
        let size = info.width * info.height * info.bytes_per_pixel;
        info!("Allocating framebuffer: size: {}, width: {}, height: {}, bytes_per_pixel: {}", size, info.width, info.height, info.bytes_per_pixel);
        let addr = unsafe {
            let ptr = alloc(Layout::from_size_align(size as usize, 4).unwrap());
            if ptr.is_null() {
                panic!("Allocation de framebuffer échouée");
            }
            ptr as u64
        };
        let mut info = info;
        info.byte_len = size as usize;
        info.stride = info.width;
        unsafe {
            FrameBuffer::create_from_raw_addr(addr, info)
        }
    }

    pub fn pixel_format(&self) -> PixelFormat {
        self.info().pixel_format
    }

    pub fn byte_per_pixel(&self) -> usize {
        self.info().bytes_per_pixel
    }

    pub fn stride(&self) -> usize {
        self.info().stride
    }

    pub fn width(&self) -> usize {
        self.info().width
    }

    pub fn height(&self) -> usize {
        self.info().height
    }


    pub fn buffer(&self) -> &[u8] {
        let buffer = self.addr as *const u8;
        unsafe {
            core::slice::from_raw_parts(buffer, self.info().byte_len)
        }
    }
    pub fn buffer_mut(&mut self) -> &mut [u8] {
        let buffer = self.addr as *mut u8;
        unsafe {
            core::slice::from_raw_parts_mut(buffer, self.info().byte_len)
        }
    }

    pub fn info(&self) -> FrameBufferInfo {
        self.info
    }

    pub fn get_pixel(&mut self, x: usize, y: usize) -> u32 {
        let pixel_format = self.pixel_format();
        let bytes_per_pixel = self.byte_per_pixel();
        let stride = self.stride();
        let width = self.width();
        let height = self.height();

        if x >= width || y >= height {
            error!("Coordinates out of bounds: x: {}, y: {}, width: {}, height: {}", x, y, width, height);
            return 0;
        }

        let offset = (y * stride + x) as usize * bytes_per_pixel;
        let buffer = self.buffer_mut();
        if offset + bytes_per_pixel > buffer.len() {
            error!("Buffer overflow: offset {}, bytes_per_pixel {}, buffer length {} buffer stride {}", offset, bytes_per_pixel, buffer.len(), stride);
            return 0; 
        }

        match pixel_format {
            PixelFormat::Bgr => {
                u32::from_ne_bytes(buffer[offset..offset + bytes_per_pixel].try_into().unwrap())
            }
            PixelFormat::Rgb => {
                u32::from_ne_bytes(buffer[offset..offset + bytes_per_pixel].try_into().unwrap())
            }
            _ => 0,
        }
    }

    pub fn draw_pixel(&mut self, x: usize, y: usize, color: u32) {
        let pixel_format = self.pixel_format();
        let bytes_per_pixel = self.byte_per_pixel();
        let stride = self.stride();
        let width = self.width();
        let height = self.height();

        if x >= width || y >= height {
            return;
        }

        let offset = (y * stride + x) as usize * bytes_per_pixel;
        let buffer = self.buffer_mut();
        if offset + bytes_per_pixel > buffer.len() {
            error!("Buffer overflow: offset {}, bytes_per_pixel {}, buffer length {}", offset, bytes_per_pixel, buffer.len());
            return; 
        }
        match pixel_format {
            PixelFormat::Bgr => {
                for i in 0..bytes_per_pixel {
                    buffer[offset + i] = ((color >> (i * 8)) & 0xFF) as u8;
                }
            }
            PixelFormat::Rgb => {
                for i in 0..bytes_per_pixel {
                    buffer[offset + i] = ((color >> (8 * (bytes_per_pixel - 1 - i))) & 0xFF) as u8;
                }
            }
            _ => {}
        }
    }

    pub fn clear_screen(&mut self, color: u32) {
        let pixel_format = self.pixel_format();
        let bytes_per_pixel = self.byte_per_pixel();
        let stride = self.stride();
        let width = self.width();
        let height = self.height();

        for y in 0..height {
            for x in 0..width {
                let offset = (y * stride + x) as usize * bytes_per_pixel;

                match pixel_format {
                    PixelFormat::Bgr => {
                        for i in 0..bytes_per_pixel {
                            self.buffer_mut()[offset + i] = ((color >> (i * 8)) & 0xFF) as u8;
                        }
                    }
                    PixelFormat::Rgb => {
                        for i in 0..bytes_per_pixel {
                            self.buffer_mut()[offset + i] = ((color >> (8 * (bytes_per_pixel - 1 - i))) & 0xFF) as u8;
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    pub fn draw_rect(&mut self, x: usize, y: usize, width: usize, height: usize, color: u32) {
        let screen_width = self.width();
        let screen_height = self.height();

        for j in y..y + height {
            for i in x..x + width {
                if i < screen_width && j < screen_height {
                    self.draw_pixel(i, j, color);
                }
            }
        }
    }

    pub fn draw_rect_no_fill(&mut self, x: usize, y: usize, width: usize, height: usize, color: u32) {
        let screen_width = self.width();
        let screen_height = self.height();

        // Draw top and bottom borders
        for i in x..x + width {
            if i < screen_width {
                if y < screen_height {
                    self.draw_pixel(i, y, color); // Top border
                }
                if y + height - 1 < screen_height {
                    self.draw_pixel(i, y + height - 1, color); // Bottom border
                }
            }
        }

        // Draw left and right borders
        for j in y..y + height {
            if j < screen_height {
                if x < screen_width {
                    self.draw_pixel(x, j, color); // Left border
                }
                if x + width - 1 < screen_width {
                    self.draw_pixel(x + width - 1, j, color); // Right border
                }
            }
        }
    }

    pub fn draw_widget(&mut self, widget: Widget) {
        widget.draw(
            self
        );
    }
        

    pub fn draw_string(&mut self, text: &str, x: usize, y: usize, color: u32) {
        FONT_DEFAULT.draw_string(text, x, y, self, color);
    }

    pub fn draw_char(&mut self, c: char, x: usize, y: usize, color: u32) {
        FONT_DEFAULT.draw_char(c, x, y, self, color);
    }


    pub fn clear_char(&mut self, x: usize, y: usize) {
        FONT_DEFAULT.clear_char(x, y, self);
    }


}



#[derive(Debug, Clone, Copy)]
pub enum GraphicMode {
    Text, 
    Graphics 
}


pub struct DoubleBuffer {
    pub framebuffer: FrameBuffer,
    pub framebuffer2: FrameBuffer,
    pub current_framebuffer: usize,
}
impl DoubleBuffer {
    pub fn new(framebuffer: FrameBuffer, framebuffer2: FrameBuffer) -> Self {
        DoubleBuffer {
            framebuffer,
            framebuffer2,
            current_framebuffer: 0,
        }
    }

    pub fn swap(&mut self) {
        self.current_framebuffer = 1 - self.current_framebuffer;
    }

    pub fn current_mut(&mut self) -> &mut FrameBuffer {
        if self.current_framebuffer == 0 {
            &mut self.framebuffer
        } else {
            &mut self.framebuffer2
        }
    }
    pub fn current(&self) -> &FrameBuffer {
        if self.current_framebuffer == 0 {
            &self.framebuffer
        } else {
            &self.framebuffer2
        }
    }

}

impl Deref for DoubleBuffer {
    type Target = FrameBuffer;

    fn deref(&self) -> &Self::Target {
        self.current()
    }
}

impl DerefMut for DoubleBuffer {    
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.current_mut()
    }
}

pub struct VirtualFrameBuffer {
    x: usize,
    y: usize,
    framebuffer: Box<FrameBuffer>,
}

impl VirtualFrameBuffer {

    pub fn global() -> Self {
        let mut context  = GLOBAL_CONTEXT.lock();
        VirtualFrameBuffer {
            x: 0,
            y: 0,
            framebuffer: context.framebuffer.clone().unwrap()
        }
    }
    pub fn new(framebuffer: FrameBuffer) -> Self {
        VirtualFrameBuffer {
            x: 0,
            y: 0,
            framebuffer: Box::new(framebuffer)
        }
    }

    pub fn set_width(&mut self, width: usize) {
        self.framebuffer.info.width = width;
    }

    pub fn set_height(&mut self, height: usize) {
        self.framebuffer.info.height = height;
    }
    pub fn set_position(&mut self, x: usize, y: usize) {
        self.x = x;
        self.y = y;
    }

    pub fn get_position(&self) -> (usize, usize) {
        (self.x, self.y)
    }

    pub fn render(&mut self, head_framebuffer: &mut FrameBuffer) {
        let width = self.framebuffer.width();
        let height = self.framebuffer.height();
        let screen_width = head_framebuffer.width();
        let screen_height = head_framebuffer.height();
        for j in 0..height {
            for i in 0..width {
                if self.x + i >= screen_width || self.y + j >= screen_height {
                    error!("Virtual framebuffer out of bounds: x: {}, y: {}", self.x + i, self.y + j);
                    continue;
                }
                let pixel = &self.framebuffer.get_pixel(i, j);

                head_framebuffer.draw_pixel(self.x + i, self.y + j, *pixel);


            }

        }
    }

    pub fn frame_buffer_mut(&mut self) -> &mut FrameBuffer {
        &mut self.framebuffer
    }

    pub fn frame_buffer(&self) -> &FrameBuffer {
        &self.framebuffer
    }
}
