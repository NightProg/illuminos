use core::ops::{Deref, DerefMut};

use bootloader_api::info::{FrameBuffer as BootFrameBuffer, FrameBufferInfo, PixelFormat};

use super::font::FONT_DEFAULT;



pub struct FrameBuffer {
    pub frame: BootFrameBuffer
}

impl FrameBuffer {
    pub fn new(frame: BootFrameBuffer) -> Self {
        FrameBuffer {
            frame
        }
    }

    pub fn pixel_format(&self) -> PixelFormat {
        self.frame.info().pixel_format
    }

    pub fn byte_per_pixel(&self) -> usize {
        self.frame.info().bytes_per_pixel
    }

    pub fn stride(&self) -> usize {
        self.frame.info().stride
    }

    pub fn width(&self) -> usize {
        self.frame.info().width
    }

    pub fn height(&self) -> usize {
        self.frame.info().height
    }

    pub fn buffer(&self) -> &[u8] {
        self.frame.buffer()
    }
    pub fn buffer_mut(&mut self) -> &mut [u8] {
        self.frame.buffer_mut()
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

    pub fn draw_text(&mut self, text: &str, x: usize, y: usize, color: u32) {
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