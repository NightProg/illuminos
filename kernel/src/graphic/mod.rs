use core::ops::{Add, Deref, Div, Mul, Sub};
// 0xFFFF_A000_0000_0000
use alloc::vec;
use alloc::vec::Vec;
use lazy_static::lazy_static;
use spin::Mutex;

pub mod console;
pub mod font;
pub mod framebuffer;
pub mod text_buffer;
pub mod vram;
pub mod window;

use framebuffer::PixelFormat;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub const fn from_rgb(rgb: u32) -> Self {
        Self {
            r: ((rgb >> 16) & 0xFF) as u8,
            g: ((rgb >> 8) & 0xFF) as u8,
            b: (rgb & 0xFF) as u8,
            a: 0xFF,
        }
    }

    pub const fn from_bgr(bgr: u32) -> Self {
        Self {
            b: ((bgr >> 16) & 0xFF) as u8,
            g: ((bgr >> 8) & 0xFF) as u8,
            r: (bgr & 0xFF) as u8,
            a: 0xFF,
        }
    }

    pub const fn red() -> Self {
        Self::new(0xFF, 0x00, 0x00, 0xFF)
    }
    pub const fn green() -> Self {
        Self::new(0x00, 0xFF, 0x00, 0xFF)
    }
    pub const fn blue() -> Self {
        Self::new(0x00, 0x00, 0xFF, 0xFF)
    }
    pub const fn white() -> Self {
        Self::new(0xFF, 0xFF, 0xFF, 0xFF)
    }
    pub const fn black() -> Self {
        Self::new(0x00, 0x00, 0x00, 0xFF)
    }
    pub const fn yellow() -> Self {
        Self::new(0xFF, 0xFF, 0x00, 0xFF)
    }
    pub const fn cyan() -> Self {
        Self::new(0x00, 0xFF, 0xFF, 0xFF)
    }
    pub const fn magenta() -> Self {
        Self::new(0xFF, 0x00, 0xFF, 0xFF)
    }
    pub const fn transparent() -> Self {
        Self::new(0x00, 0x00, 0x00, 0x00)
    }
    pub const fn grey() -> Self {
        Self::new(0x80, 0x80, 0x80, 0xFF)
    }
    pub const fn light_blue() -> Self {
        Self::new(0x00, 0x80, 0xFF, 0xFF)
    }
    pub const fn orange() -> Self {
        Self::new(0xFF, 0x80, 0x00, 0xFF)
    }
    pub const fn dark_grey() -> Self {
        Self::new(0x40, 0x40, 0x40, 0xFF)
    }

    pub fn from_pixel_fmt(fmt: PixelFormat, data: &[u8]) -> Self {
        match fmt {
            PixelFormat::Bgr => Self {
                b: data[0],
                g: data[1],
                r: data[2],
                a: 0xFF,
            },
            PixelFormat::Rgb => Self {
                r: data[0],
                g: data[1],
                b: data[2],
                a: 0xFF,
            },
            PixelFormat::Other {
                red_shift,
                green_shift,
                blue_shift,
            } => {
                let r = data.get((red_shift / 8) as usize).copied().unwrap_or(0);
                let g = data.get((green_shift / 8) as usize).copied().unwrap_or(0);
                let b = data.get((blue_shift / 8) as usize).copied().unwrap_or(0);
                Self::new(r, g, b, 0xFF)
            }
        }
    }

    #[inline]
    pub fn to_pixel_fmt_u32(&self, fmt: PixelFormat) -> u32 {
        fmt.encode(self.r, self.g, self.b)
    }

    #[inline]
    pub const fn to_rgb(self) -> (u8, u8, u8) {
        (self.r, self.g, self.b)
    }

    #[inline]
    pub const fn to_u32(self) -> u32 {
        ((self.r as u32) << 16) | ((self.g as u32) << 8) | (self.b as u32)
    }

    #[inline]
    pub const fn is_uniform(self) -> bool {
        self.r == self.g && self.g == self.b
    }

    pub fn blend(self, background: Self) -> Self {
        if self.a == 0xFF {
            return self;
        }
        if self.a == 0x00 {
            return background;
        }
        let a = self.a as u32;
        let ia = 255 - a;
        Self {
            r: ((self.r as u32 * a + background.r as u32 * ia) / 255) as u8,
            g: ((self.g as u32 * a + background.g as u32 * ia) / 255) as u8,
            b: ((self.b as u32 * a + background.b as u32 * ia) / 255) as u8,
            a: 0xFF,
        }
    }
}
