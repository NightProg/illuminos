use crate::modern_graphic::image::ImageFormat;

pub mod image;
pub mod renderer;
pub mod command;
pub mod queue;
pub mod surface;
mod windows;
pub mod font;
pub mod app;

#[derive(Debug, Clone, Copy, PartialOrd, PartialEq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub fn red() -> Self {
        Self {
            r: 255,
            g: 0,
            b: 0,
            a: 255,
        }
    }
    pub fn green() -> Self {
        Self {
            r: 0,
            g: 255,
            b: 0,
            a: 255,
        }
    }
    pub fn blue() -> Self {
        Self {
            r: 0,
            g: 0,
            b: 255,
            a: 255,
        }
    }
    pub fn white() -> Self {
        Self {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        }
    }
    pub fn black() -> Self {
        Self {
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        }
    }

    pub fn from_format_bytes(format: ImageFormat, bytes: &[u8]) -> Self {
        match format {
            ImageFormat::RGB32 => Self {
                r: bytes[0],
                g: bytes[1],
                b: bytes[2],
                a: 255,
            },
            ImageFormat::RGBA32 => Self {
                r: bytes[0],
                g: bytes[1],
                b: bytes[2],
                a: bytes[3],
            },
            ImageFormat::BGR32 => Self {
                r: bytes[2],
                g: bytes[1],
                b: bytes[0],
                a: 255,
            },
            ImageFormat::BGRA32 => Self {
                r: bytes[2],
                g: bytes[1],
                b: bytes[0],
                a: bytes[3],
            },
        }
    }


    pub fn from_hex_str(hex: &str) -> Self {
        let hex = hex.trim_start_matches('#');
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap();
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap();
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap();
        let a = if hex.len() == 8 {
            u8::from_str_radix(&hex[6..8], 16).unwrap()
        } else {
            255
        };
        Self { r, g, b, a }
    }

    pub fn from_hex(hex: u32) -> Self {
        let r = ((hex >> 16) & 0xFF) as u8;
        let g = ((hex >> 8) & 0xFF) as u8;
        let b = (hex & 0xFF) as u8;
        let a = ((hex >> 24) & 0xFF) as u8;
        Self { r, g, b, a }
    }
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub fn to_bgr_bytes(&self) -> [u8; 4] {
        [self.b, self.g, self.r, self.a]
    }

}

impl core::ops::Add for Color {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            r: self.r.saturating_add(other.r),
            g: self.g.saturating_add(other.g),
            b: self.b.saturating_add(other.b),
            a: self.a.saturating_add(other.a),
        }
    }
}

impl core::ops::Index<usize> for Color {
    type Output = u8;

    fn index(&self, index: usize) -> &Self::Output {
        match index {
            0 => &self.r,
            1 => &self.g,
            2 => &self.b,
            3 => &self.a,
            _ => panic!("Index out of bounds"),
        }
    }
}


#[derive(Debug, Clone, Copy)]
pub struct Vertex {
    pub x: f32,
    pub y: f32,
    pub color: Color,
}

impl Vertex {
    pub fn new(x: f32, y: f32, color: Color) -> Self {
        Self {
            x, y, color
        }
    }
}

impl core::ops::Add for Vertex {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
            color: self.color,
        }
    }
}