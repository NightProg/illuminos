use alloc::string::String;
use crate::modern_graphic::command::CommandBuffer;
use crate::modern_graphic::image::{Image, ImageFormat};

pub struct Window {
    width: u32,
    height: u32,
    title: String,
    command_buffer: CommandBuffer,
}

impl Window {
    pub fn new(width: u32, height: u32, title: String) -> Self {
        Window {
            width,
            height,
            title,
            command_buffer: CommandBuffer::new(),
        }
    }

    pub fn render(&mut self, command_buffer: &mut CommandBuffer) {
        let output = command_buffer.call_sub_command_buffer(self.command_buffer.clone());
    }
}