use alloc::vec::Vec;
use lazy_static::lazy_static;

use super::framebuffer::RawFrameBuffer;
use crate::geo::{Points, Rect};
use crate::math::{Arithmetic, Vec2};
use crate::{io::serial::SerialPortWriter, math::vec2};

pub const FONT_DEFAULT_DATA: &[u8] = include_bytes!("../../assets/font/default8x16.psfu");
pub const FONT_UNI2_TERMINUS32X16_DATA: &[u8] =
    include_bytes!("../../assets/font/Uni2-Terminus32x16.psf");
pub const FONT_DEFAULT_WIDTH: usize = 8;
pub const FONT_DEFAULT_HEIGHT: usize = 16;

pub const FONT_UNI2_TERMINUS32X16_WIDTH: usize = 32;
pub const FONT_UNI2_TERMINUS32X16_HEIGHT: usize = 16;

lazy_static! {
    pub static ref FONT_DEFAULT: Psf2Font = { Psf2Font::new(FONT_DEFAULT_DATA).unwrap() };
}
lazy_static! {
    pub static ref FONT_UNI2_TERMINUS32x16: Psf2Font =
        { Psf2Font::new(FONT_UNI2_TERMINUS32X16_DATA).unwrap() };
}

pub trait PsfFont: Clone {
    fn get_glyph(&self, c: char) -> Option<PsfGlyph>;
    fn width(&self) -> usize;
    fn height(&self) -> usize;
}
#[derive(Clone)]
pub struct Psf1Font<'a> {
    pub data: &'a [u8],
    pub glyph_data: &'a [u8],
    pub width: usize,
    pub height: usize,
    pub magic: u16,
    pub mode: u8,
    pub charsize: u8,
}

impl<'a> PsfFont for Psf1Font<'a> {
    fn get_glyph(&self, c: char) -> Option<PsfGlyph> {
        self.get_glyph(c)
    }

    fn width(&self) -> usize {
        self.width
    }

    fn height(&self) -> usize {
        self.height
    }
}

impl<'a> Psf1Font<'a> {
    pub fn new(data: &'a [u8]) -> Option<Self> {
        if data.len() < 4 || data[0] != 0x36 || data[1] != 0x04 {
            return None;
        }

        let magic = u16::from_le_bytes([data[0], data[1]]);
        let mode = data[2];
        let charsize = data[3];

        let glyph_data = &data[4..];

        Some(Psf1Font {
            data,
            glyph_data,
            width: 8,
            height: charsize as usize,
            magic,
            mode,
            charsize,
        })
    }

    pub fn get_glyph(&self, c: char) -> Option<PsfGlyph<'a>> {
        let index = c as usize;
        let glyph_start = index.checked_mul(self.height)?;
        let glyph_end = glyph_start.checked_add(self.height)?;
        if glyph_end > self.glyph_data.len() {
            return None;
        }

        Some(PsfGlyph {
            data: &self.glyph_data[glyph_start..glyph_end],
            width: self.width,
            height: self.height,
            version: 1,
        })
    }
}

#[derive(Debug, Clone)]
pub struct Psf2Font {
    pub data: Vec<u8>,
    pub width: usize,
    pub height: usize,
    pub num_glyphs: usize,
    pub charsize: usize,
}

impl PsfFont for Psf2Font {
    fn get_glyph(&self, c: char) -> Option<PsfGlyph> {
        self.get_glyph(c)
    }

    fn width(&self) -> usize {
        self.width
    }

    fn height(&self) -> usize {
        self.height
    }
}

#[derive(Debug, Copy, Clone)]
pub struct PsfGlyph<'a> {
    pub data: &'a [u8],
    pub width: usize,
    pub height: usize,
    pub version: u8,
}

impl Psf2Font {
    pub fn new(data: &[u8]) -> Option<Self> {
        let magic = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        if magic != 0x864ab572 {
            return None;
        }

        let num_glyphs =
            u32::from_le_bytes([data[0x10], data[0x11], data[0x12], data[0x13]]) as usize;
        let charsize =
            u32::from_le_bytes([data[0x14], data[0x15], data[0x16], data[0x17]]) as usize;
        let height = u32::from_le_bytes([data[0x18], data[0x19], data[0x1A], data[0x1B]]) as usize;
        let width = u32::from_le_bytes([data[0x1C], data[0x1D], data[0x1E], data[0x1F]]) as usize;

        Some(Psf2Font {
            data: data[32..].to_vec(),
            width,
            height,
            num_glyphs,
            charsize,
        })
    }

    pub fn get_glyph(&self, c: char) -> Option<PsfGlyph> {
        let index = c as usize;
        if index >= self.num_glyphs {
            return None;
        }

        let glyph_start = index * self.charsize;
        let glyph_end = glyph_start + self.charsize;
        let data = &self.data[glyph_start..glyph_end];

        Some(PsfGlyph {
            data,
            width: self.width,
            height: self.height,
            version: 2,
        })
    }
}
