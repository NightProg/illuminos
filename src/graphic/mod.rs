use core::ops::Deref;

use text::TextEdit;
use lazy_static::lazy_static;
use spin::Mutex;

use crate::{context::GLOBAL_CONTEXT, drivers::keyboard::KeyEvent};

pub mod vram;
pub mod framebuffer;
pub mod font;
pub mod text;
pub mod windows;
pub mod app;
pub mod console;
pub mod widget;

pub const RED: u32 = 0xFF0000;
pub const GREEN: u32 = 0x00FF00;
pub const BLUE: u32 = 0x0000FF;
pub const WHITE: u32 = 0xFFFFFF;
pub const BLACK: u32 = 0x000000;
pub const YELLOW: u32 = 0xFFFF00;
pub const CYAN: u32 = 0x00FFFF;
pub const MAGENTA: u32 = 0xFF00FF;
pub const GRAY: u32 = 0x808080;
pub const LIGHT_GRAY: u32 = 0xD3D3D3;
pub const DARK_GRAY: u32 = 0xA9A9A9;
pub const ORANGE: u32 = 0xFFA500;
pub const PURPLE: u32 = 0x800080;
pub const PINK: u32 = 0xFFC0CB;
pub const BROWN: u32 = 0xA52A2A;
pub const LIGHT_BLUE: u32 = 0xADD8E6;
pub const LIGHT_GREEN: u32 = 0x90EE90;



pub trait GraphicMode {
    fn handle_keyboard_event(&mut self, event: KeyEvent);
}
