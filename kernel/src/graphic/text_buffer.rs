use alloc::vec;
use alloc::vec::Vec;
use crate::graphic::Color;
use crate::graphic::font::PsfFont;
use crate::graphic::framebuffer::FrameBuffer;
use crate::math::{vec2, Vec2};

pub struct Cursor {
    pub position: Vec2<usize>,
    pub size: Vec2<usize>,
    pub visible: bool,
}
pub struct TextBuffer<'a, F: FrameBuffer, P: PsfFont + 'a> {
    pub framebuffer: &'a mut F,
    pub font: &'a P,
    pub cursor: Vec2<usize>,
    pub fg: Color,
    pub bg: Color,
    pub buffer: Vec<Vec<(char, Color, Color)>>,
    pub tab_width: usize,
    pub cursor_obj:  Cursor,
}

impl <'a, F: FrameBuffer, P: PsfFont> TextBuffer<'a, F, P> {
    pub fn new(framebuffer: &'a mut F, font: &'a P, fg: Color, bg: Color) -> Self {
        let cols = framebuffer.width() / font.width();
        let rows = framebuffer.height() / font.height();
        let buffer = vec![vec![(' ', fg, bg); cols]; rows];
        Self {
            framebuffer,
            font,
            cursor: Vec2::new(0, 0),
            fg,
            bg,
            buffer,
            tab_width: 4,
            cursor_obj: Cursor {
                position: Vec2::new(0, 0),
                size: Vec2::new(font.width(), font.height()),
                visible: true,
            },
        }
    }

    pub fn clear(&mut self) {
        self.framebuffer.clear_screen(self.bg);
        for row in &mut self.buffer {
            for cell in row.iter_mut() {
                *cell = (' ', self.fg, self.bg);
            }
        }
        self.cursor = Vec2::new(0, 0);
    }

    pub fn update_cursor(&mut self) {
        if self.cursor_obj.visible {
            let x = self.cursor.x * self.font.width();
            let y = self.cursor.y * self.font.height();
            for row in 0..self.font.height() {
                for col in 0..self.font.width() {
                    self.framebuffer.set_pixel(vec2(x + col, y + row), self.fg);
                }
            }
        }
    }
    pub fn remove_cursor(&mut self) {
        let x = self.cursor.x * self.font.width();
        let y = self.cursor.y * self.font.height();
        for row in 0..self.font.height() {
            for col in 0..self.font.width() {
                self.framebuffer.set_pixel(vec2(x + col, y + row), self.bg);
            }
        }
    }

    pub fn put_char(&mut self, c: char) {
        match c {
            '\n' => {
                self.remove_cursor();
                self.cursor.x = 0;
                self.cursor.y += 1;
                if self.cursor.y >= self.buffer.len() {
                    self.scroll();
                    self.cursor.y = self.buffer.len() - 1;
                }
            }
            '\r' => {
                self.cursor.x = 0;
            }
            '\t' => {
                self.cursor.x += self.tab_width - (self.cursor.x % self.tab_width);
                if self.cursor.x >= self.buffer[0].len() {
                    self.cursor.x = 0;
                    self.cursor.y += 1;
                    if self.cursor.y >= self.buffer.len() {
                        self.scroll();
                        self.cursor.y = self.buffer.len() - 1;
                    }
                }
            }
            _ => {
                if self.cursor.y >= self.buffer.len() {
                    self.scroll();
                    self.cursor.y = self.buffer.len() - 1;
                }
                if self.cursor.x >= self.buffer[0].len() {
                    self.cursor.x = 0;
                    self.cursor.y += 1;
                    if self.cursor.y >= self.buffer.len() {
                        self.scroll();
                        self.cursor.y = self.buffer.len() - 1;
                    }
                }
                self.buffer[self.cursor.y][self.cursor.x] = (c, self.fg, self.bg);
                if let Some(glyph) = self.font.get_glyph(c, Vec2::new(0, 0)) {
                    for row_idx in 0..glyph.height {
                        for col_idx in 0..glyph.width {
                            let byte_index = (row_idx * ((glyph.width + 7) / 8)) + (col_idx / 8);
                            let bit_index = 7 - (col_idx % 8);
                            if byte_index < glyph.data.len() {
                                let byte = glyph.data[byte_index];
                                let pixel_on = (byte >> bit_index) & 1 == 1;
                                let color = if pixel_on { self.fg } else { self.bg };
                                let x = self.cursor.x * self.font.width() + col_idx;
                                let y = self.cursor.y * self.font.height() + row_idx;
                                self.framebuffer.set_pixel(vec2(x, y), color);
                            }
                        }
                    }
                }
                self.cursor.x += 1;
            }
        }
        self.update_cursor();
    }
    pub fn put_str(&mut self, s: &str) {
        for c in s.chars() {
            self.put_char(c);
        }
    }

    pub fn backspace(&mut self) {
        self.remove_cursor();
        if self.cursor.x > 0 {
            self.cursor.x -= 1;
        } else if self.cursor.y > 0 {
            self.cursor.y -= 1;
            self.cursor.x = self.buffer[0].len() - 1;
        }
        self.buffer[self.cursor.y][self.cursor.x] = (' ', self.fg, self.bg);
        let x = self.cursor.x * self.font.width();
        let y = self.cursor.y * self.font.height();
        for row in 0..self.font.height() {
            for col in 0..self.font.width() {
                self.framebuffer.set_pixel(vec2(x + col, y + row), self.bg);
            }
        }
        self.update_cursor();
    }

    pub fn scroll(&mut self) {
        let rows = self.buffer.len();
        let cols = self.buffer[0].len();
        for y in 1..rows {
            for x in 0..cols {
                self.buffer[y - 1][x] = self.buffer[y][x];
            }
        }
        for x in 0..cols {
            self.buffer[rows - 1][x] = (' ', self.fg, self.bg);
        }
        
        // On ne redessine pas tout l'écran à chaque scroll si on veut de la performance,
        // mais ici c'est plus simple pour garder la cohérence avec le buffer.
        self.framebuffer.clear_screen(self.bg);
        for (y, row) in self.buffer.iter().enumerate() {
            for (x, &(c, fg, _bg)) in row.iter().enumerate() {
                if c == ' ' {
                    continue;
                }
                if let Some(glyph) = self.font.get_glyph(c, Vec2::new(0, 0)) {
                    for row_idx in 0..glyph.height {
                        for col_idx in 0..glyph.width {
                            let byte_index = (row_idx * ((glyph.width + 7) / 8)) + (col_idx / 8);
                            let bit_index = 7 - (col_idx % 8);
                            if byte_index < glyph.data.len() {
                                let byte = glyph.data[byte_index];
                                let pixel_on = (byte >> bit_index) & 1 == 1;
                                if pixel_on {
                                    let px = x * self.font.width() + col_idx;
                                    let py = y * self.font.height() + row_idx;
                                    self.framebuffer.set_pixel(vec2(px, py), fg);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}