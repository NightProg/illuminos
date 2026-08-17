use crate::context::GLOBAL_CONTEXT;
use crate::graphic::Color;
use crate::graphic::font::PsfFont;
use crate::graphic::framebuffer::RawFrameBuffer;
use crate::sync::mutex::TimeoutMutex;
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;

#[derive(Clone, Copy)]
pub struct Cursor {
    pub position: (usize, usize),
    pub size: (usize, usize),
    pub visible: bool,
}

#[derive(Clone)]
pub struct TextBuffer<P: PsfFont> {
    pub framebuffer: Arc<TimeoutMutex<RawFrameBuffer>>,
    pub font: P,
    pub fg: Color,
    pub bg: Color,
    pub buffer: Vec<Vec<(char, Color, Color)>>,
    pub tab_width: usize,
    pub cursor: Cursor,
}

impl<P: PsfFont> TextBuffer<P> {
    pub fn new(font: &P, fg: Color, bg: Color) -> Self {
        let fb = GLOBAL_CONTEXT.framebuffer.lock();
        let cols = fb.width / font.width();
        let rows = fb.height / font.height();
        let buffer = vec![vec![(' ', fg, bg); cols]; rows];
        drop(fb);
        Self {
            framebuffer: GLOBAL_CONTEXT.framebuffer.clone(),
            font: font.clone(),
            fg,
            bg,
            buffer,
            tab_width: 4,
            cursor: Cursor {
                position: (0, 0),
                size: (font.width(), font.height()),
                visible: true,
            },
        }
    }

    pub fn clear(&mut self) {
        self.framebuffer.lock().clear_screen(self.bg);
        for row in &mut self.buffer {
            for cell in row.iter_mut() {
                *cell = (' ', self.fg, self.bg);
            }
        }
        self.cursor.position = (0, 0);
    }

    pub fn update_cursor(&mut self) {
        if self.cursor.visible {
            let x = self.cursor.position.0 * self.cursor.size.0;
            let y = self.cursor.position.1 * self.cursor.size.1;
            for row in 0..self.font.height() {
                for col in 0..self.font.width() {
                    self.framebuffer.lock().set_pixel(x + col, y + row, self.fg);
                }
            }
        }
    }
    pub fn remove_cursor(&mut self) {
        let x = self.cursor.position.0 * self.font.width();
        let y = self.cursor.position.1 * self.font.height();
        for row in 0..self.font.height() {
            for col in 0..self.font.width() {
                self.framebuffer.lock().set_pixel(x + col, y + row, self.bg);
            }
        }
    }
    pub fn put_char(&mut self, c: char) {
        match c {
            '\n' => {
                self.remove_cursor();
                self.cursor.position.0 = 0;
                self.cursor.position.1 += 1;
                if self.cursor.position.1 >= self.buffer.len() {
                    self.scroll();
                    self.cursor.position.1 = self.buffer.len() - 1;
                }
            }
            '\r' => {
                self.cursor.position.0 = 0;
            }
            '\t' => {
                self.cursor.position.0 +=
                    self.tab_width - (self.cursor.position.0 % self.tab_width);
                if self.cursor.position.0 >= self.buffer[0].len() {
                    self.cursor.position.0 = 0;
                    self.cursor.position.1 += 1;
                    if self.cursor.position.1 >= self.buffer.len() {
                        self.scroll();
                        self.cursor.position.1 = self.buffer.len() - 1;
                    }
                }
            }
            _ => {
                if self.cursor.position.1 >= self.buffer.len() {
                    self.scroll();
                    self.cursor.position.1 = self.buffer.len() - 1;
                }
                if self.cursor.position.0 >= self.buffer[0].len() {
                    self.cursor.position.0 = 0;
                    self.cursor.position.1 += 1;
                    if self.cursor.position.1 >= self.buffer.len() {
                        self.scroll();
                        self.cursor.position.1 = self.buffer.len() - 1;
                    }
                }
                self.buffer[self.cursor.position.1][self.cursor.position.0] = (c, self.fg, self.bg);
                if let Some(glyph) = self.font.get_glyph(c) {
                    for row_idx in 0..glyph.height {
                        for col_idx in 0..glyph.width {
                            let byte_index = (row_idx * ((glyph.width + 7) / 8)) + (col_idx / 8);
                            let bit_index = 7 - (col_idx % 8);
                            if byte_index < glyph.data.len() {
                                let byte = glyph.data[byte_index];
                                let pixel_on = (byte >> bit_index) & 1 == 1;
                                let color = if pixel_on { self.fg } else { self.bg };
                                let x = self.cursor.position.0 * self.font.width() + col_idx;
                                let y = self.cursor.position.1 * self.font.height() + row_idx;
                                self.framebuffer.lock().set_pixel(x, y, color);
                            }
                        }
                    }
                }
                self.cursor.position.0 += 1;
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
        if self.cursor.position.0 > 0 {
            self.cursor.position.0 -= 1;
        } else if self.cursor.position.1 > 0 {
            self.cursor.position.1 -= 1;
            self.cursor.position.0 = self.buffer[0].len() - 1;
        }
        self.buffer[self.cursor.position.1][self.cursor.position.0] = (' ', self.fg, self.bg);
        let x = self.cursor.position.0 * self.font.width();
        let y = self.cursor.position.1 * self.font.height();
        for row in 0..self.font.height() {
            for col in 0..self.font.width() {
                self.framebuffer.lock().set_pixel(x + col, y + row, self.bg);
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

        self.framebuffer.lock().clear_screen(self.bg);
        for (y, row) in self.buffer.iter().enumerate() {
            for (x, &(c, fg, _bg)) in row.iter().enumerate() {
                if c == ' ' {
                    continue;
                }
                if let Some(glyph) = self.font.get_glyph(c) {
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
                                    self.framebuffer.lock().set_pixel(px, py, fg);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
