use super::{Key, KeyEvent, KeyState, KeyboardLayout, SpecialKey};



const AZERTY_SCANCODE_TO_CHAR: [Option<char>; 256] = {
    let mut map = [None; 256];
    map[0x10] = Some('a');
    map[0x11] = Some('z');
    map[0x12] = Some('e');
    map[0x13] = Some('r');
    map[0x14] = Some('t');
    map[0x15] = Some('y');
    map[0x16] = Some('u');
    map[0x17] = Some('i');
    map[0x18] = Some('o');
    map[0x19] = Some('p');
    map[0x1E] = Some('q');
    map[0x1F] = Some('s');
    map[0x20] = Some('d');
    map[0x21] = Some('f');
    map[0x22] = Some('g');
    map[0x23] = Some('h');
    map[0x24] = Some('j');
    map[0x25] = Some('k');
    map[0x26] = Some('l');
    map[0x2C] = Some('w');
    map[0x2D] = Some('x');
    map[0x2E] = Some('c');
    map[0x2F] = Some('v');
    map[0x30] = Some('b');
    map[0x31] = Some('n');
    map[0x32] = Some('m');
    map
};

pub struct Azerty;

impl KeyboardLayout for Azerty {
    fn from_scancode(scancode: u8) -> Option<KeyEvent> {
        if scancode == 0 {
            return None;
        }

        if let Some(c) = AZERTY_SCANCODE_TO_CHAR[scancode as usize] {
            return Some(KeyEvent::pressed(Key::Char(c)));
        }

        let key = match scancode {
            0x1C => Some(Key::Special(SpecialKey::Enter)),
            0x39 => Some(Key::Special(SpecialKey::Space)),
            0x0E => Some(Key::Special(SpecialKey::Backspace)),
            0x0F => Some(Key::Special(SpecialKey::Tab)),
            0x01 => Some(Key::Special(SpecialKey::Escape)),
            0x4B => Some(Key::Special(SpecialKey::Left)),
            0x4D => Some(Key::Special(SpecialKey::Right)),
            0x48 => Some(Key::Special(SpecialKey::Up)),
            0x50 => Some(Key::Special(SpecialKey::Down)),
            0x47 => Some(Key::Special(SpecialKey::Home)),
            0x4F => Some(Key::Special(SpecialKey::End)),
            0x49 => Some(Key::Special(SpecialKey::PageUp)),
            0x51 => Some(Key::Special(SpecialKey::PageDown)),
            0x52 => Some(Key::Special(SpecialKey::Insert)),
            0x53 => Some(Key::Special(SpecialKey::Delete)),
            0x3B => Some(Key::Special(SpecialKey::F1)),
            0x3C => Some(Key::Special(SpecialKey::F2)),
            0x3D => Some(Key::Special(SpecialKey::F3)),
            0x3E => Some(Key::Special(SpecialKey::F4)),
            0x3F => Some(Key::Special(SpecialKey::F5)),
            0x40 => Some(Key::Special(SpecialKey::F6)),
            0x41 => Some(Key::Special(SpecialKey::F7)),
            0x42 => Some(Key::Special(SpecialKey::F8)),
            0x43 => Some(Key::Special(SpecialKey::F9)),
            0x44 => Some(Key::Special(SpecialKey::F10)),
            0x57 => Some(Key::Special(SpecialKey::F11)),
            0x58 => Some(Key::Special(SpecialKey::F12)),
            _ => None,
        };

        let state = KeyState::from_scancode(scancode);

        if let Some(k) = key {
            Some(KeyEvent {
                key: k,
                state,
            })
        } else {
            None
        }
    }
}
