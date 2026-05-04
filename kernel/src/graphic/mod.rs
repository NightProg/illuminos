use core::ops::{Add, Deref, Div, Mul, Sub};
// 0xFFFF_A000_0000_0000
use alloc::vec;
use alloc::vec::Vec;
use bootloader_api::info::PixelFormat;
use lazy_static::lazy_static;
use spin::Mutex;

pub mod console;
pub mod font;
pub mod framebuffer;
pub mod text_buffer;
pub mod vram;
pub mod window;

#[derive(Debug, Clone, Copy)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
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

    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub fn from_pixel_fmt(pixel_format: PixelFormat, data: &[u8]) -> Self {
        match pixel_format {
            PixelFormat::U8 => Color::new(data[0], data[1], data[2], 0xFF),
            PixelFormat::Rgb => Color::from_rgb(u32::from_be_bytes([data[0], data[1], data[2], 0])),
            PixelFormat::Bgr => Color::from_bgr(u32::from_be_bytes([data[0], data[1], data[2], 0])),
            PixelFormat::Unknown {
                red_position,
                green_position,
                blue_position,
            } => Color::new(
                data[red_position as usize],
                data[green_position as usize],
                data[blue_position as usize],
                0xFF,
            ),
            e => todo!("Add support for pixel format: {:?}", e),
        }
    }

    pub fn is_uniform(&self) -> bool {
        self.r == self.g && self.g == self.b && self.b == self.r
    }

    pub fn to_pixel_fmt(&self, pixel_format: PixelFormat) -> Vec<u8> {
        match pixel_format {
            PixelFormat::U8 => vec![self.r, self.g, self.b, self.a],
            PixelFormat::Rgb => vec![self.r, self.g, self.b, self.a],
            PixelFormat::Bgr => vec![self.b, self.g, self.r, self.a],
            PixelFormat::Unknown {
                red_position,
                green_position,
                blue_position,
            } => {
                let mut result = Vec::new();
                result.insert(red_position as usize, self.r);
                result.insert(green_position as usize, self.g);
                result.insert(blue_position as usize, self.b);
                result
            }
            e => todo!("Add support for pixel format: {:?}", e),
        }
    }

    pub fn to_u32(&self) -> u32 {
        u32::from_be_bytes([00, self.r, self.g, self.b])
    }
}
