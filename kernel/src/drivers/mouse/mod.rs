use lazy_static::lazy_static;
use spin::Mutex;
use crate::geo::{rect, Rect};
use crate::info;
use crate::io::ps2::Ps2Controller;
use crate::math::{vec2, Vec2};

pub const MOUSE_DATA_REPORTING: u8 = 0xF4;

lazy_static! {
    pub static ref MOUSE_POS: Mutex<Vec2<i64>> = Mutex::new(vec2(100, 100));
}
lazy_static! {
    pub static ref MOUSE: Mutex<Mouse> = Mutex::new(Mouse::new());
}

#[derive(Debug, Copy, Clone)]
pub struct Mouse {
    pub ps2controller: Ps2Controller,
    pub packet: [u8; 3],
    pub index: usize,
}
impl Mouse {
    pub fn new() -> Self {
        let mut ps2controller = Ps2Controller::new();
        Self { ps2controller, packet: [0; 3], index: 0 }
    }

    pub fn init(&mut self) {
        info!("Enable auxiliary device");
        self.ps2controller.cmd_write(crate::io::ps2::CMD_ENABLE_AUX);
        info!("Send mouse command");
        self.ps2controller.send_mouse_cmd(MOUSE_DATA_REPORTING); // Enable mouse
        info!("Mouse command sent");
        let data = self.ps2controller.data(); // Read data
        if data != 0xFA {
            panic!("Mouse not detected: {:X}", data);
        }
    }
}