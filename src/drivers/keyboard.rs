pub mod azerty;
pub mod qwerty;

use spin::Mutex;
use x86_64::instructions::port::Port;
use lazy_static::lazy_static;

use crate::{context::{self, Context}};


#[derive(Debug, PartialEq, Eq)]
pub enum KeyState {
    Pressed,
    Released,
}

impl KeyState {
    pub fn from_scancode(scancode: u8) -> KeyState {
        if scancode & 0x80 != 0 {
            KeyState::Released
        } else {
            KeyState::Pressed
        }
    }
}

#[derive(Debug)]
pub struct KeyEvent {
    pub key: Key,
    pub state: KeyState,
}

impl KeyEvent {
    pub fn pressed(key: Key) -> KeyEvent {
        KeyEvent {
            key,
            state: KeyState::Pressed,
        }
    }

    pub fn released(key: Key) -> KeyEvent {
        KeyEvent {
            key,
            state: KeyState::Released,
        }
    }

    pub fn from_scancode<T: KeyboardLayout>(scancode: u8) -> Option<KeyEvent> {
        T::from_scancode(scancode)

    }

}


#[derive(Debug, Clone, Copy)]
pub enum Key {
    Char(char),
    Special(SpecialKey),
}

impl Key {

    pub fn to_string(&self) -> Option<char> {
        match self {
            Key::Char(c) => {
                Some(*c)
            },
            Key::Special(s) => {
                None
            },
        }
    }
}


#[derive(Debug, Clone, Copy)]
pub enum SpecialKey {
    Enter,
    Space,
    Backspace,
    Tab,
    Escape,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
    Insert,
    Delete,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
}



pub struct Keyboard{
    pub handle_key: fn(KeyEvent, &Mutex<Context>),
}

impl Keyboard {

    pub fn read_scancode(&self) -> u8 {
        let mut port = Port::new(0x60);
        unsafe {
            port.read()
        }
    }

    pub fn read_key(&self) -> Option<KeyEvent> {
        let scancode = self.read_scancode();
        KeyEvent::from_scancode::<qwerty::Qwerty>(scancode)
    }

    pub fn new(key: fn(KeyEvent, &Mutex<Context>)) -> Keyboard {
        Keyboard {
            handle_key: key,
        }
    }

    pub fn handle_key(&self, key: KeyEvent, context: &Mutex<Context>) {
        (self.handle_key)(key, context);
    }
}


lazy_static! {
    pub static ref KEYBOARD: Mutex<Keyboard> = Mutex::new(Keyboard::new(|_, _| {}));
}

pub fn set_keyboard_handler(handler: fn(KeyEvent, &Mutex<Context>)) {

    KEYBOARD.lock().handle_key = handler;
}


pub trait KeyboardLayout {
    fn from_scancode(scancode: u8) -> Option<KeyEvent>;
}



