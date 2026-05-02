use crate::io::{inb, outb};

pub const DATA_PORT: u16 = 0x60;
pub const STATUS_PORT: u16 = 0x64;
pub const CMD_ENABLE_AUX: u8 = 0xA8; // Enable auxiliary device - souris
pub const CMD_WRITE_MOUSE: u8 = 0xD4;


#[derive(Debug, Clone, Copy)]
pub struct Ps2Controller {
    pub status: u8,
}

impl Ps2Controller {
    pub fn new() -> Self {
        Self { status: 0 }
    }

    pub fn status(&mut self) -> u8 {
        self.status = inb(STATUS_PORT);
        self.status
    }

    pub fn wait_output(&mut self) {
        while self.status() & 0x01 == 0 {}
    }

    pub fn wait_input(&mut self) {
        while self.status() & 0x02 != 0 {}

    }

    pub fn cmd_write(&mut self, cmd: u8) {
        self.wait_input();
        outb(STATUS_PORT, cmd);
    }

    pub fn send_mouse_cmd(&mut self, cmd: u8) {
        self.cmd_write(CMD_WRITE_MOUSE);
        self.wait_input();
        outb(DATA_PORT, cmd);
    }

    pub fn data(&mut self) -> u8 {
        self.wait_output();
        inb(DATA_PORT)
    }
}