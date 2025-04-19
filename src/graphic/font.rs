
use lazy_static::lazy_static;

use crate::io::serial::SerialPortWriter;

use super::framebuffer::FrameBuffer;



pub const FONT_DEFAULT_DATA: &[u8] = include_bytes!("../../assets/font/default8x16.psfu");


pub const FONT_DEFAULT_WIDTH: usize = 8;
pub const FONT_DEFAULT_HEIGHT: usize = 16;


lazy_static! {
    pub static ref FONT_DEFAULT: PsfFont = {
        PsfFont::new(FONT_DEFAULT_DATA, FONT_DEFAULT_WIDTH, FONT_DEFAULT_HEIGHT)
    };
}

pub struct PsfFont {
    pub data: &'static [u8],
    pub width: usize,
    pub height: usize,
}

impl PsfFont {
    pub fn new(data: &'static [u8], width: usize, height: usize) -> Self {
        PsfFont {
            data,
            width,
            height,
        }
    }

    // On suppose que chaque glyphe est de `height` octets, donc pour un fichier PSF1 de 8x16, c'est 16 octets par glyphe.
    pub fn get_glyph(&self, c: char) -> Option<&[u8]> {
        // Le caractère doit être dans la plage des caractères ASCII
        let index = c as usize + 2;
        let glyph_start = index * self.height;

        // Vérifie si l'index ne dépasse pas la longueur des données
        if glyph_start + self.height <= self.data.len() {
            Some(&self.data[glyph_start..glyph_start + self.height])
        } else {
            None
        }
    }

    pub fn draw_char(&self, c: char, x: usize, y: usize, framebuffer: &mut FrameBuffer, color: u32) {
        if let Some(glyph) = self.get_glyph(c) {
            for row in 0..self.height {
                for col in 0..self.width {
                    // Vérifie si le bit est allumé pour ce pixel (glyph[row] représente une ligne)
                    if glyph[row] & (1 << (7 - col)) != 0 {
                        if x + col < framebuffer.width() && y + row < framebuffer.height() {
                            framebuffer.draw_pixel(x + col, y + row, color);
                        }
                    }
                }
            }
        }
    }

    pub fn draw_string(&self, s: &str, x: usize, y: usize, framebuffer: &mut FrameBuffer, color: u32) {
        let mut offset_x = x;
        for c in s.chars() {
            self.draw_char(c, offset_x, y, framebuffer, color);
            offset_x += self.width;
        }
    }

    pub fn clear_char(&self, x: usize, y: usize, framebuffer: &mut FrameBuffer) {
        for row in 0..self.height {
            for col in 0..self.width {
                if x + col < framebuffer.width() && y + row < framebuffer.height() {
                    framebuffer.draw_pixel(x + col, y + row, 0x000000); // Couleur noire
                }
            }
        }
    }
}


pub struct Psf2Font {
    pub data: &'static [u8],  // Données de la police PSF2
    pub width: usize,         // Largeur des glyphes
    pub height: usize,        // Hauteur des glyphes
    pub num_glyphs: usize,    // Nombre de glyphes
}

impl Psf2Font {
    // Lecture de l'en-tête PSF2 pour extraire les métadonnées
    pub fn new(data: &'static [u8]) -> Option<Self> {

        use core::fmt::Write;
        write!(SerialPortWriter, "PSF2 Font Data: {:?}", &data[0..2]).unwrap();
        write!(SerialPortWriter, "{} {}", 0x36, 0x04).unwrap();
        // Vérification du format PSF2
        if &data[0..2] != &[0x36, 0x04] { // 0x8604 est la signature PSF2
            return None; // Ce n'est pas un fichier PSF2
        }

        // Récupérer les informations sur la police à partir de l'en-tête
        let width = data[8] as usize;     // Largeur des glyphes
        let height = data[9] as usize;    // Hauteur des glyphes
        let num_glyphs = data[6] as usize; // Nombre de glyphes (2 octets possible)
        
        // Le début des glyphes dans les données
        Some(Psf2Font {
            data: &data[32..], // Les glyphes commencent après 32 octets (en-tête)
            width,
            height,
            num_glyphs,
        })
    }

    // Obtenir un glyphe spécifique en fonction du caractère
    pub fn get_glyph(&self, c: char) -> Option<&[u8]> {
        let index = c as usize;
        
        if index < self.num_glyphs {
            let glyphe_start = index * self.height;
            let glyphe_end = glyphe_start + self.height;
            Some(&self.data[glyphe_start..glyphe_end])
        } else {
            None
        }
    }

    // Dessiner un caractère sur le framebuffer
    pub fn draw_char(&self, c: char, x: usize, y: usize, framebuffer: &mut FrameBuffer, color: u32) {
        if let Some(glyph) = self.get_glyph(c) {
            for row in 0..self.height {
                for col in 0..self.width {
                    if glyph[row] & (1 << (7 - col)) != 0 {
                        if x + col < framebuffer.width() && y + row < framebuffer.height() {
                            framebuffer.draw_pixel(x + col, y + row, color);
                        }
                    }
                }
            }
        }
    }

    // Dessiner une chaîne de caractères
    pub fn draw_string(&self, s: &str, x: usize, y: usize, framebuffer: &mut FrameBuffer, color: u32) {
        let mut offset_x = x;
        for c in s.chars() {
            self.draw_char(c, offset_x, y, framebuffer, color);
            offset_x += self.width;
        }
    }
}
