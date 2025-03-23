pub mod azerty;
pub mod qwerty;

use spin::Mutex;
use x86_64::instructions::port::Port;
use lazy_static::lazy_static;

use crate::WRITER;


#[derive(Debug, Clone, Copy)]
pub enum Key {
    Char(char),
    Special(SpecialKey),
}

impl Key {
    pub fn from_scancode<T: KeyboardLayout>(scancode: u8) -> Option<Key> {
        T::from_scamcode(scancode)

    }

    pub fn to_string(&self) -> Option<char> {
        match self {
            Key::Char(c) => {
                Some(*c)
            },
            Key::Special(s) => {
                Some(match s {
                    SpecialKey::Backspace => { WRITER.lock().remove_last_char(); return None }
                    SpecialKey::Tab => { '\t' }
                    SpecialKey::Enter => { '\n' }
                    SpecialKey::Escape => { return None }
                    SpecialKey::Left => { return None }
                    SpecialKey::Right => { return None }
                    SpecialKey::Up => { return None }
                    SpecialKey::Down => { return None }
                    SpecialKey::Home => { return None }
                    SpecialKey::End => { return None }
                    SpecialKey::PageUp => { return None }
                    SpecialKey::Space => { ' ' }
                    _ => { '\0'}
                })
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



pub struct Keyboard {
    pub handle_key: fn(Key),
}

impl Keyboard {

    pub fn read_scancode(&self) -> u8 {
        let mut port = Port::new(0x60);
        unsafe {
            port.read()
        }
    }

    pub fn read_key(&self) -> Option<Key> {
        let scancode = self.read_scancode();
        Key::from_scancode::<qwerty::Qwerty>(scancode)
    }

    pub fn new(key: fn(Key)) -> Keyboard {
        Keyboard {
            handle_key: key,
        }
    }

    pub fn handle_key(&self, key: Key) {
        (self.handle_key)(key);
    }
}


lazy_static! {
    pub static ref KEYBOARD: Mutex<Keyboard> = Mutex::new(Keyboard::new(|_| {}));
}

pub fn set_keyboard_handler(handler: fn(Key)) {
    KEYBOARD.lock().handle_key = handler;
}


pub trait KeyboardLayout {
    fn from_scamcode(scancode: u8) -> Option<Key>;
}



