
use alloc::vec;
use alloc::vec::Vec;
use crate::math::{Arithmetic, Vec2};
use crate::modern_graphic::Color;
use crate::modern_graphic::command::CommandBuffer;
use crate::modern_graphic::image::{Image, ImageFormat, ImageLayout, ImageRef};

#[derive(Debug, Clone)]
pub struct PsfGlyph {
    pub data: Vec<u8>,
    pub width: usize,
    pub height: usize,
    pub version: u8,
    pub pos: [f32; 2]
}

#[derive(Debug, Clone)]
pub struct Psf2Font {
    pub data: Vec<u8>,
    pub width: usize,
    pub height: usize,
    pub num_glyphs: usize,
    pub charsize: usize,
}

impl Psf2Font {
    pub fn new(data: &[u8]) -> Option<Self> {
        let magic = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        if magic != 0x864ab572 {
            return None;
        }

        let num_glyphs = u32::from_le_bytes([data[0x10], data[0x11], data[0x12], data[0x13]]) as usize;
        let charsize   = u32::from_le_bytes([data[0x14], data[0x15], data[0x16], data[0x17]]) as usize;
        let height     = u32::from_le_bytes([data[0x18], data[0x19], data[0x1A], data[0x1B]]) as usize;
        let width      = u32::from_le_bytes([data[0x1C], data[0x1D], data[0x1E], data[0x1F]]) as usize;

        Some(Psf2Font {
            data: Vec::from(&data[32..]),
            width,
            height,
            num_glyphs,
            charsize,
        })
    }

    pub fn get_glyph(&self, c: char, pos: [f32; 2]) -> Option<PsfGlyph> {
        let index = c as usize;
        if index >= self.num_glyphs {
            return None;
        }

        let glyph_start = index * self.charsize;
        let glyph_end = glyph_start + self.charsize;
        let data = &self.data[glyph_start..glyph_end];

        Some(PsfGlyph {
            data: data.to_vec(),
            width: self.width,
            height: self.height,
            version: 2,
            pos
        })
    }

}

impl PsfGlyph {
    pub fn image(&self, color: Color) -> Image {
        let image_layout = ImageLayout::new(ImageFormat::RGBA32, self.width as u32, self.height as u32);
        let mut image = Image::new(image_layout);
        let mut x = 0;
        let mut y = 0;
        for byte in &self.data {
            for i in 0..8 {
                if (byte >> (7 - i)) & 1u8 == 1u8 {
                    image.write_pixel(x, y, color);
                } else {
                    image.write_pixel(x, y, Color::black());
                }
                x += 1;
                if x >= self.width as u32 {
                    x = 0;
                    y += 1;
                }
            }
        }

        image
    }

    pub fn vertexs(&self, color: Color) -> Vec<crate::modern_graphic::Vertex> {
        // get a quad of the glyph with width and height
        let mut vertexs = vec![
            crate::modern_graphic::Vertex::new(self.pos[0], self.pos[1], color),
            crate::modern_graphic::Vertex::new(self.pos[0] + self.width as f32, self.pos[1], color),
            crate::modern_graphic::Vertex::new(self.pos[0] + self.width as f32, self.pos[1] + self.height as f32, color),
            crate::modern_graphic::Vertex::new(self.pos[0], self.pos[1] + self.height as f32, color),
        ];

        return vertexs;
    }

    pub fn indices(&self) -> Vec<u32> {
        let mut indices = vec![
            0, 1, 2,
            2, 3, 0,
        ];

        indices
    }

    pub fn uv(&self) -> Vec<(f32, f32)> {
        let mut uv = vec![
            (0.0, 0.0),
            (1.0, 0.0),
            (1.0, 1.0),
            (0.0, 1.0),
        ];

        uv
    }

    pub fn draw(&self, command_buffer: &mut CommandBuffer, color: Color) {
        let image = self.image(color);
        let vertexs = self.vertexs(color);
        let indices = self.indices();
        let uv = self.uv();

        let image_ref = command_buffer
            .add_image(image);

        command_buffer
            .bind_vertex(vertexs)
            .bind_index(indices.clone())
            .bind_uv(uv)
            .bind_texture(image_ref)
            .draw_texture_indexed(indices.len() as u32)
            .unwrap();

    }

    pub fn draw_on(&self, command_buffer: &mut CommandBuffer, color: Color, image_ref: ImageRef) {
        let vertexs = self.vertexs(color);
        let indices = self.indices();
        let uv = self.uv();

        command_buffer
            .bind_vertex(vertexs)
            .bind_index(indices.clone())
            .bind_uv(uv)
            .bind_texture(image_ref)
            .draw_texture_indexed_on(indices.len() as u32, image_ref);
    }
}
