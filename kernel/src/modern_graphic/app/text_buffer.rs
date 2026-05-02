use alloc::string::{String, ToString};
use alloc::vec::Vec;
use lazy_static::lazy_static;
use crate::modern_graphic::Color;
use crate::modern_graphic::command::CommandBuffer;
use crate::modern_graphic::font::Psf2Font;
use crate::modern_graphic::image::Image;
use crate::modern_graphic::queue::Queue;
use crate::{print, println};

#[derive(Clone)]
pub struct TextBuffer {
    pub queue: Queue,
    pub global_command_buffer: CommandBuffer,
    pub font: Psf2Font,

    pub current_pos: (f32, f32),
    pub current_color: Color,
    pub dimensions: (f32, f32),
    pub target: Image,
    pub buffer: String
}

impl TextBuffer {
    pub fn new(queue: Queue, target: Image, font: Psf2Font, dimensions: (f32, f32)) -> Self {
        let mut command_buffer = CommandBuffer::new();
        command_buffer.set_main_image_target(target.clone());
        Self {
            queue,
            global_command_buffer: command_buffer,
            current_pos: (0.0, 0.0),
            current_color: Color::white(),
            font,
            dimensions,
            target,
            buffer: String::new(),
        }
    }

    pub fn set_current_pos(&mut self, x: f32, y: f32) {
        self.current_pos = (x, y);
    }

    pub fn set_current_color(&mut self, color: Color) {
        self.current_color = color;
    }

    pub fn flush(&mut self, new_target: Image) -> Image {
        let image = self.global_command_buffer.execute_with(&self.queue).unwrap();
        self.global_command_buffer.clear();
        self.global_command_buffer.set_main_image_target(new_target);
        self.current_pos = (0.0, 0.0);
        self.buffer.clear();
        image
    }

    pub fn draw_char(&mut self, c: char) {
        let glyph = self.font.get_glyph(c, [self.current_pos.0, self.current_pos.1]).unwrap();
        glyph.draw(&mut self.global_command_buffer, self.current_color);
        self.buffer.push(c);
        self.current_pos.0 += glyph.width as f32;
        if self.current_pos.0 >= self.dimensions.0 {
            self.current_pos.0 = 0.0;
            self.current_pos.1 += glyph.height as f32;
        }
        if self.current_pos.1 >= self.dimensions.1 {
            // scroll the text buffer up
            self.current_pos.1 = self.dimensions.1 - (glyph.height) as f32;
            self.global_command_buffer.clear();
            self.global_command_buffer.set_main_image_target(self.target.clone());
            let nl = self.buffer.find('\n').map(|n| n+1).unwrap_or(self.buffer.len());
            self.buffer.drain(0..nl);

            self.current_pos.0 = 0.0;

            self.draw_string(&self.buffer.clone());

        }


    }

    pub fn max_width(&self) -> f32 {
        self.dimensions.0 / self.font.width as f32
    }

    pub fn max_height(&self) -> f32 {
        self.dimensions.1 / self.font.height as f32
    }




    pub fn draw_string(&mut self, s: &str) {
        let mut s = s.to_string();
        let mut lines: Vec<String> = s.lines().map(|l| l.to_string()).collect();
        let mut new_lines: Vec<String> = s.lines().map(|l| l.to_string()).collect();
        lines.append(&mut new_lines);

        let mut has_remove = false;
        while lines.len() > (self.max_height() - 1.0) as usize {
            lines.remove(0);
            if !has_remove {
                has_remove = true;
            }
        }

        if has_remove {
            s = lines.join("\n");
        }


        for c in s.chars() {
            if c == '\n' {
                self.current_pos.0 = 0.0;
                self.current_pos.1 += self.font.get_glyph('A', [0.0, 0.0]).unwrap().height as f32; // Use height of a glyph to move down
                self.buffer.push('\n');
                continue;
            }
            if c == '\r' {
                self.current_pos.0 = 0.0; // Reset to the start of the line
                continue;
            }
            if c == '\t' {
                self.current_pos.0 += 4.0 * self.font.get_glyph('A', [0.0, 0.0]).unwrap().width as f32; // Tab size of 4 characters
                continue;
            }
            self.draw_char(c);
        }
    }




}