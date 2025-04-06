use alloc::string::String;
use core::{fmt::Write, ops::Deref};

use alloc::vec::Vec;
use lazy_static::lazy_static;
use spin::Mutex;

use crate::{context::GLOBAL_CONTEXT, drivers::keyboard::{Key, KeyEvent, KeyState, SpecialKey}, info, println};

use super::{font::FONT_DEFAULT, framebuffer::{self, FrameBuffer}, windows::WINDOW_MANAGER, GraphicMode, WHITE};



#[derive(Debug, Clone)]
pub struct TextBuffer {
    pub width: usize,
    pub height: usize,
    pub cursor_x: usize,
    pub cursor_y: usize,
    pub color: u32,
    pub cursor_x_old: Vec<usize>,
    pub cursor_old_pos: (usize, usize),
    pub winid: usize,
    pub lines: Vec<String>,
    pub cols: String
}

impl TextBuffer {

    pub fn create(width: usize, height: usize, x: usize, y: usize) -> Self {
        let winid = WINDOW_MANAGER.lock().new_window(width , height , x, y);

        let mut new = Self::new(winid);
        new.init();

        new
    }
    pub fn new(winid: usize) -> Self {
        TextBuffer {
            width: 0,
            height: 0,
            cursor_x: 0,
            cursor_y: 0,
            cursor_x_old: Vec::new(),
            color: 0xFFFFFF,
            cursor_old_pos: (0, 0),
            lines: Vec::new(),
            cols: String::new(),
            winid
        }
    }

    pub fn init(&mut self) {
        let width = (WINDOW_MANAGER
            .lock()
            .get_window(self.winid)
            .get_virt()
            .frame_buffer()
            .width()) / 8;
        let height = (WINDOW_MANAGER
            .lock()
            .get_window(self.winid)
            .get_virt()
            .frame_buffer()
            .height())/ 16;

        self.width = width;
        self.height = height;
    }
    pub fn get_color(&self) -> u32 {
        self.color
    }

    pub fn set_color(&mut self, color: u32) {
        self.color = color;
    }


    pub fn draw_char(&mut self, c: char, x: usize, y: usize) {
        WINDOW_MANAGER
            .lock()
            .get_window_mut(self.winid)
            .get_virt_mut()
            .frame_buffer_mut()
            .draw_char(c, x * 8, y * 16, self.color);
    }

    pub fn clear_char(&mut self, x: usize, y: usize) {
        WINDOW_MANAGER
            .lock()
            .get_window_mut(self.winid)
            .get_virt_mut()
            .frame_buffer_mut()
            .clear_char(x * 8 , y * 16);
    }

    pub fn draw_string(&mut self, s: &str, x: usize, y: usize) {
        WINDOW_MANAGER
            .lock()
            .get_window_mut(self.winid)
            .get_virt_mut()
            .frame_buffer_mut()
            .draw_string(s, x * 8, y * 16, self.color);
    }
    pub fn clear(&mut self) {
        self.cursor_y = 0;
        self.cursor_x = 0;

        WINDOW_MANAGER
            .lock()
            .get_window_mut(self.winid)
            .get_virt_mut()
            .frame_buffer_mut()
            .clear_screen(0x000000);
        WINDOW_MANAGER
            .lock()
            .get_window_mut(self.winid)
            .get_virt_mut()
            .frame_buffer_mut()
            .draw_rect_no_fill(0, 0, self.width * 8, self.height * 16, 0xFFFFFF);
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
            self.lines.push(
                self.cols.clone()
            );

            self.cols = String::new();

        } else {
            self.cols.push(c);
            self.draw_char(c, self.cursor_x, self.cursor_y);
            self.cursor_x += 1;


        }

        if self.cursor_x >= self.width {
            self.cursor_x_old.push(self.cursor_x);
            self.cursor_x = 0;
            self.cursor_y += 1;


        }

        if self.cursor_y >= self.height+1 {
            self.scrolling();
        }
    }

    pub fn scrolling(&mut self) {
        if !self.lines.is_empty() {
            self.lines.remove(0);
            // Adjust the current cursor position
            if self.cursor_y > 0 {
                self.cursor_y -= 1;
            }
        }
        // Clear the text area
        self.clear();
        // Redraw the remaining lines
        for (y, line) in self.lines.clone().into_iter().enumerate() {
            self.draw_string(&line, 0, y);
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
    pub fn new(winid: usize) -> Self {
        let text_buffer = TextBuffer::new(winid);
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

