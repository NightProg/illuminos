use core::{fmt::Write, ops::Deref};

use alloc::vec::Vec;
use lazy_static::lazy_static;
use spin::Mutex;

use crate::{context::GLOBAL_CONTEXT, drivers::keyboard::{Key, KeyEvent, KeyState, SpecialKey}};

use super::{font::FONT_DEFAULT, framebuffer::{self, FrameBuffer}, GraphicMode, WHITE};

lazy_static! {
    pub static ref GLOBAL_TEXT_BUFFER: Mutex<TextBuffer> = Mutex::new(TextBuffer::new());
    pub static ref GLOBAL_TEXT_EDIT: Mutex<TextEdit> = Mutex::new(TextEdit::from_text_buffer(GLOBAL_TEXT_BUFFER.lock().clone()));
}

pub fn set_global_text_buffer_color(color: u32) {
    GLOBAL_TEXT_BUFFER.lock().set_color(color);
}

pub fn get_global_text_buffer_color() -> u32 {
    GLOBAL_TEXT_BUFFER.lock().color
}

#[macro_export]
macro_rules! print_textbuffer {
    ($($arg:tt)*) => {
        {
            use core::fmt::Write;
            write!(&mut *$crate::graphic::console::GLOBAL_TEXT_BUFFER.lock(), $($arg)*).unwrap();
        }
    }
}

#[macro_export]
macro_rules! println_textbuffer {
    ($($arg:tt)*) => {
        {
            use core::fmt::Write;
            writeln!(&mut *$crate::graphic::console::GLOBAL_TEXT_BUFFER.lock(), $($arg)*).unwrap();
        }
    }
}


#[derive(Debug, Clone)]
pub struct TextBuffer {
    pub width: usize,
    pub height: usize,
    pub cursor_x: usize,
    pub cursor_y: usize,
    pub color: u32,
    pub cursor_x_old: Vec<usize>,
    pub cursor_old_pos: (usize, usize)
}

impl TextBuffer {
    pub fn new() -> Self {
        let width = GLOBAL_CONTEXT.lock().framebuffer.as_ref().unwrap().width() / 8; // Largeur de la zone de texte
        let height = GLOBAL_CONTEXT.lock().framebuffer.as_ref().unwrap().height() / 16; // Hauteur de la zone de texte
        TextBuffer {
            width,
            height,
            cursor_x: 0,
            cursor_y: 0,
            cursor_x_old: Vec::new(),
            color: 0xFFFFFF,
            cursor_old_pos: (0, 0)
        }
    }

    pub fn set_color(&mut self, color: u32) {
        self.color = color;
    }

    pub fn draw_char(&mut self, c: char, x: usize, y: usize) {
        FONT_DEFAULT.draw_char(c, x * 8, y * 16, GLOBAL_CONTEXT.lock().framebuffer.as_mut().unwrap(), self.color);
    }

    pub fn clear_char(&mut self, x: usize, y: usize) {
        FONT_DEFAULT.clear_char(x * 8, y * 16, GLOBAL_CONTEXT.lock().framebuffer.as_mut().unwrap());
    }

    pub fn draw_string(&mut self, s: &str, x: usize, y: usize) {
        FONT_DEFAULT.draw_string(s, x * 8, y * 16, GLOBAL_CONTEXT.lock().framebuffer.as_mut().unwrap(), self.color);
    }

    pub fn clear(&mut self) {
        for y in 0..self.height {
            for x in 0..self.width {
                self.clear_char(x, y);
            }
        }
    }



    pub fn set_cursor(&mut self, x: usize, y: usize) {
        self.cursor_x = x;
        self.cursor_y = y;
    }

    pub fn get_cursor(&self) -> (usize, usize) {
        (self.cursor_x, self.cursor_y)
    }

    pub fn write(&mut self, c: char) {
        if c == '\n' {
            self.cursor_x_old.push(self.cursor_x);
            self.cursor_x = 0;
            self.cursor_y += 1; 

        } else {
            self.draw_char(c, self.cursor_x, self.cursor_y);
            self.cursor_x += 1;


        }

        if self.cursor_x >= self.width {
            self.cursor_x_old.push(self.cursor_x);
            self.cursor_x = 0;
            self.cursor_y += 1;


        }

        if self.cursor_y >= self.height {
            self.clear();
            self.cursor_y = 0;

        }
    }

    pub fn remove_char(&mut self) {
        if self.cursor_x > 0 {
            self.cursor_x -= 1;
            self.clear_char(self.cursor_x, self.cursor_y);
        } else {
            if self.cursor_y > 0 {
                self.cursor_y -= 1;
                self.cursor_x = self.cursor_x_old.pop().unwrap_or(0);
                self.clear_char(self.cursor_x, self.cursor_y);
            }
        }

    }

    pub fn write_string(&mut self, s: &str) {
        for c in s.chars() {
            self.write(c);
        }
    }

}


impl Write for TextBuffer {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

pub struct TextEdit {
    pub text_buffer: TextBuffer,
}

impl TextEdit {
    pub fn from_text_buffer(text_buffer: TextBuffer) -> Self {
        TextEdit { text_buffer }
    }
    pub fn new() -> Self {
        let text_buffer = TextBuffer::new();
        TextEdit { text_buffer }
    }

    pub fn write(&mut self, c: char) {
        self.text_buffer.write(c);
    }

    pub fn remove_char(&mut self) {
        self.text_buffer.remove_char();
    }


    pub fn clear(&mut self) {
        self.text_buffer.clear();
    }
}



impl GraphicMode for TextEdit {
    fn handle_keyboard_event(&mut self, event: KeyEvent) {
        match event.key {
            Key::Char(c)  => {
                self.write(c);
            },
            Key::Special(spe) => {
                match spe {
                    SpecialKey::Enter => {
                        self.write('\n');
                    }
                    SpecialKey::Backspace => {
                        if event.state == KeyState::Released {
                            return;
                        }
                        self.remove_char();
                    }
                    _ => {}
                }
            }
        }
    }
}

