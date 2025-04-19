use alloc::vec::Vec;
use bootloader_api::info::FrameBufferInfo;
use x86_64::structures::paging::frame;

use super::{font::FONT_DEFAULT, framebuffer::{FrameBuffer, VirtualFrameBuffer}};
use alloc::vec;


pub struct Widget {
    x: usize,
    y: usize,
    color: u32,
    buffer: Vec<u8>,
    info: FrameBufferInfo
}

impl Widget {
    pub fn new(info: FrameBufferInfo, x: usize, y: usize) -> Self {
        let buffer = vec![0; info.width * info.height * 4];
        Widget {
            x,
            y,
            color: 0xFFFFFF,
            buffer,
            info
        }
    }

    pub fn rect(&mut self) -> &mut Self {
        for i in 0..self.info.height {
            for j in 0..self.info.width {
                let index = (i * self.info.width + j) * self.info.bytes_per_pixel;
                for k in 0..self.info.bytes_per_pixel {
                    self.buffer[index + k] = (self.color >> (8 * k)) as u8;
                }
            }
        }

        self
    }


    pub fn character(&mut self, c: char) -> &mut Self {
        let glyph = FONT_DEFAULT.get_glyph(c);

        if let Some(glyph) = glyph {
            for row in 0..self.info.height {
                for col in 0..self.info.width {
                    if glyph[row] & (1 << (7 - col)) != 0 {
                        if self.x + col < self.info.width && self.y + row < self.info.height {
                            let index = (self.y + row) * self.info.width + (self.x + col);
                            let pixel_index = index * self.info.bytes_per_pixel;
                            for k in 0..self.info.bytes_per_pixel {
                                self.buffer[pixel_index + k] = (self.color >> (8 * k)) as u8;
                            }
                        }
                    }
                }
            }
        }

        self
    }


    pub fn draw(&self, framebuffer: &mut FrameBuffer) {
        for i in 0..self.info.height {
            for j in 0..self.info.width {
                let index = (i * self.info.width + j) * self.info.bytes_per_pixel;
                framebuffer.draw_pixel(self.x + j, self.y + i, u32::from_le_bytes([
                    self.buffer[index],
                    self.buffer[index + 1],
                    self.buffer[index + 2],
                    self.buffer[index + 3],
                ]));
            }
        }
    } 
}