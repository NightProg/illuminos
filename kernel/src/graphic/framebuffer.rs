use core::{
    alloc::Layout,
    ops::{Deref, DerefMut},
};

use crate::geo::*;
use crate::math::*;
use alloc::boxed::Box;
use alloc::{alloc::alloc, vec::Vec};
use core::fmt::Debug;
use spin::Mutex;

use super::Color;
use super::font::{FONT_DEFAULT, Psf1Font, Psf2Font, PsfFont};
use crate::context::GLOBAL_CONTEXT;
use crate::{context::Context, error, info};

// ── Pixel format ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PixelFormat {
    /// 0xXXRRGGBB  (red_shift=16, green_shift=8, blue_shift=0)
    Bgr,
    /// 0xXXBBGGRR  (red_shift=0,  green_shift=8, blue_shift=16)
    #[default]
    Rgb,
    /// Arbitrary channel layout described by mask_shift values
    Other {
        red_shift: u8,
        green_shift: u8,
        blue_shift: u8,
    },
}

impl PixelFormat {
    pub fn from_limine(fb: &limine::framebuffer::Framebuffer) -> Self {
        match (fb.red_mask_shift, fb.green_mask_shift, fb.blue_mask_shift) {
            (16, 8, 0) => PixelFormat::Bgr,
            (0, 8, 16) => PixelFormat::Rgb,
            (r, g, b) => PixelFormat::Other {
                red_shift: r,
                green_shift: g,
                blue_shift: b,
            },
        }
    }

    /// Encode an (r,g,b) triplet into a u32 pixel value for this format.
    #[inline]
    pub fn encode(&self, r: u8, g: u8, b: u8) -> u32 {
        match *self {
            PixelFormat::Bgr => ((r as u32) << 16) | ((g as u32) << 8) | (b as u32),
            PixelFormat::Rgb => ((b as u32) << 16) | ((g as u32) << 8) | (r as u32),
            PixelFormat::Other {
                red_shift,
                green_shift,
                blue_shift,
            } => {
                ((r as u32) << red_shift) | ((g as u32) << green_shift) | ((b as u32) << blue_shift)
            }
        }
    }

    #[inline]
    pub fn write_pixel(&self, buf: &mut [u8], color: u32) {
        for (i, byte) in buf.iter_mut().enumerate() {
            *byte = ((color >> (i * 8)) & 0xFF) as u8;
        }
    }

    pub fn get_red_shift(&self) -> u8 {
        match *self {
            PixelFormat::Bgr => 16,
            PixelFormat::Rgb => 0,
            PixelFormat::Other { red_shift, .. } => red_shift,
        }
    }

    pub fn get_green_shift(&self) -> u8 {
        match *self {
            PixelFormat::Bgr => 8,
            PixelFormat::Rgb => 8,
            PixelFormat::Other { green_shift, .. } => green_shift,
        }
    }

    pub fn get_blue_shift(&self) -> u8 {
        match *self {
            PixelFormat::Bgr => 0,
            PixelFormat::Rgb => 16,
            PixelFormat::Other { blue_shift, .. } => blue_shift,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct RawFrameBuffer {
    pub phys_addr: u64,
    pub addr: u64,
    pub width: usize,
    pub height: usize,
    pub pitch: usize,
    pub bpp: usize,
    pub pixel_format: PixelFormat,
}

unsafe impl Send for RawFrameBuffer {}
unsafe impl Sync for RawFrameBuffer {}

impl RawFrameBuffer {
    pub const fn none() -> Self {
        RawFrameBuffer {
            phys_addr: 0,
            addr: 0,
            width: 0,
            height: 0,
            pitch: 0,
            bpp: 0,
            pixel_format: PixelFormat::Rgb, // default to something
        }
    }
    pub fn from_limine(fb: &limine::framebuffer::Framebuffer, phys_addr: u64) -> Self {
        let bpp = (fb.bpp / 8) as usize;
        RawFrameBuffer {
            phys_addr,
            addr: fb.address() as u64,
            width: fb.width as usize,
            height: fb.height as usize,
            pitch: fb.pitch as usize,
            bpp,
            pixel_format: PixelFormat::from_limine(fb),
        }
    }

    pub fn from_limine_at(
        addr: u64,
        fb: &limine::framebuffer::Framebuffer,
        phys_addr: u64,
    ) -> Self {
        let mut raw = Self::from_limine(fb, phys_addr);
        raw.addr = addr;
        raw
    }

    pub unsafe fn alloc_same_size(&self) -> Self {
        let size = self.pitch * self.height;
        let ptr = unsafe { alloc(Layout::from_size_align(size, 4096).expect("bad layout")) };
        assert!(!ptr.is_null(), "framebuffer allocation failed");
        RawFrameBuffer {
            addr: ptr as u64,
            width: self.width,
            height: self.height,
            pitch: self.pitch,
            bpp: self.bpp,
            pixel_format: self.pixel_format,
            phys_addr: 0, // not mapped to a physical address
        }
    }

    pub unsafe fn free(&mut self) {
        let size = self.pitch * self.height;
        unsafe {
            alloc::alloc::dealloc(
                self.addr as *mut u8,
                Layout::from_size_align(size, 4096).unwrap(),
            );
        }
        self.addr = 0;
    }

    pub fn byte_len(&self) -> usize {
        self.pitch * self.height
    }

    pub fn stride_pixels(&self) -> usize {
        self.pitch / self.bpp
    }

    pub fn buffer(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self.addr as *const u8, self.byte_len()) }
    }

    pub fn buffer_mut(&mut self) -> &mut [u8] {
        unsafe { core::slice::from_raw_parts_mut(self.addr as *mut u8, self.byte_len()) }
    }

    #[inline]
    fn pixel_offset(&self, x: usize, y: usize) -> usize {
        y * self.pitch + x * self.bpp
    }

    pub fn clear_screen(&mut self, color: Color) {
        for x in 0..self.width {
            for y in 0..self.height {
                self.set_pixel(x, y, color);
            }
        }
    }

    pub fn set_pixel(&mut self, x: usize, y: usize, color: Color) -> Option<()> {
        if x >= self.width || y >= self.height {
            return None;
        }

        let off = self.pixel_offset(x, y);
        let bytes = color.to_pixel_fmt_u32(self.pixel_format).to_le_bytes();
        let buf = self.buffer_mut();

        buf[off] = bytes[0];
        buf[off + 1] = bytes[1];
        buf[off + 2] = bytes[2];
        buf[off + 3] = bytes[3];

        Some(())
    }

    pub fn get_pixel(&self, x: usize, y: usize) -> Option<Color> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let off = self.pixel_offset(x, y);
        let buf = self.buffer();
        let bytes = &buf[off..off + 3];
        Some(Color::from_pixel_fmt(self.pixel_format, bytes))
    }
}
