use alloc::string::{String, ToString};

use super::{
    GraphicMode,
    app::Application,
    windows::{WINDOW_MANAGER, Window},
};
use crate::graphic::framebuffer::FrameBuffer;
use crate::graphic::text::{TextBuffer, TextEdit};
use crate::graphic::windows::WindowManager;
use crate::{drivers::keyboard::Key, error};
use core::fmt::Write;

pub enum ConsoleCommand {
    Clear,
    Print(Expr),
    Println(Expr),
}

impl ConsoleCommand {
    pub fn from_str(cmd: &str) -> Option<Self> {
        let mut parts = cmd.split_whitespace();
        match parts.next() {
            Some("clear") => Some(ConsoleCommand::Clear),
            Some("print") => {
                if let Some(arg) = parts.next() {
                    Some(ConsoleCommand::Print(Expr::from_str(arg)))
                } else {
                    None
                }
            }
            Some("println") => {
                if let Some(arg) = parts.next() {
                    Some(ConsoleCommand::Println(Expr::String(arg.to_string())))
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

pub enum Expr {
    String(String),
    Number(i32),
    Bool(bool),
}

impl Expr {
    pub fn from_str(s: &str) -> Self {
        if let Ok(num) = s.parse::<i32>() {
            Expr::Number(num)
        } else if s == "true" {
            Expr::Bool(true)
        } else if s == "false" {
            Expr::Bool(false)
        } else {
            Expr::String(s.to_string())
        }
    }
}

pub struct Console {
    winid: usize,
    current_cmd: String,
    text_edit: TextEdit,
}

impl Console {
    pub fn new() -> Self {
        Console {
            winid: 0,
            current_cmd: String::new(),
            text_edit: TextEdit::new(0),
        }
    }

    pub fn text_buffer_mut(&mut self) -> &mut TextBuffer {
        &mut self.text_edit.text_buffer
    }

    pub fn get_window_id(&self) -> usize {
        self.winid
    }

    pub fn set_window_id(&mut self, console: usize, id: usize) {
        self.winid = console;
        self.text_edit.text_buffer.winid = id;
        self.text_edit.text_buffer.init();
    }

    pub fn exec_cmd(&mut self, cmd: String) {
        if let Some(command) = ConsoleCommand::from_str(&cmd) {
            match command {
                ConsoleCommand::Clear => self.clear(),
                ConsoleCommand::Print(expr) => self.print(expr),
                ConsoleCommand::Println(expr) => self.println(expr),
            }
        } else {
            error!("Unknown command: {}", cmd);
        }
    }

    pub fn clear(&mut self) {
        let mut window = WINDOW_MANAGER.lock();
        let win = window.get_window_mut(self.winid);
        win.clear();
    }

    pub fn print(&mut self, expr: Expr) {
        let mut window = WINDOW_MANAGER.lock();
        let win = window.get_window_mut(self.winid);
        match expr {
            Expr::String(s) => {
                write!(self.text_buffer_mut(), "{}: {}", win.get_window_id(), s).unwrap()
            }
            Expr::Number(n) => {
                write!(self.text_buffer_mut(), "{}: {}", win.get_window_id(), n).unwrap()
            }
            Expr::Bool(b) => {
                write!(self.text_buffer_mut(), "{}: {}", win.get_window_id(), b).unwrap()
            }
        }
    }

    pub fn println(&mut self, expr: Expr) {
        self.print(expr);
        self.text_buffer_mut().write('\n');
    }
}

impl Application for Console {
    fn window(&mut self, window_manager: &mut WindowManager) -> usize {
        let console = window_manager.new_window(600, 800, 0, 800);
        let buffer = window_manager.new_window(600, 800, 800, 0);
        self.set_window_id(console, buffer);
        self.winid
    }

    fn handle_keyboard_event(&mut self, event: crate::drivers::keyboard::KeyEvent) {
        self.text_edit.handle_keyboard_event(event);
    }
}
