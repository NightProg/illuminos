use bootloader_api::info::FrameBufferInfo;
use crate::modern_graphic::image::Image;
use crate::modern_graphic::image::ImageFormat;
use crate::modern_graphic::Color;
use crate::modern_graphic::command::Command;


#[derive(Clone)]
pub struct ScreenFrameBuffer {
    info: FrameBufferInfo,
    addr: u64
}

impl ScreenFrameBuffer {
    pub fn new(info: FrameBufferInfo, addr: u64) -> Self {
        Self {
            info,
            addr
        }
    }

    pub fn info(&self) -> &FrameBufferInfo {
        &self.info
    }
}


#[derive(Clone)]
pub struct Surface {
    pub screen: ScreenFrameBuffer
}

impl Surface {
    pub fn new(info: FrameBufferInfo, addr: u64) -> Self {
        Self {
            screen: ScreenFrameBuffer {
                info,
                addr
            }
        }
    }

    pub fn pixel_length(&self) -> usize {
        self.screen.info.bytes_per_pixel
    }

    pub fn length(&self) -> usize {
        self.screen.info.width * self.screen.info.height * self.pixel_length()
    }

    pub fn buffer(&self) -> &[u8] {
        unsafe {
            core::slice::from_raw_parts(
                self.screen.addr as *const u8,
                self.length()
            )
        }
    }

    pub fn buffer_mut(&mut self) -> &mut [u8] {
        unsafe {
            core::slice::from_raw_parts_mut(
                self.screen.addr as *mut u8,
                self.length()
            )
        }
    }

    pub fn present(&mut self, image: &Image) {
        self.buffer_mut().copy_from_slice(&image.data)
    }


}