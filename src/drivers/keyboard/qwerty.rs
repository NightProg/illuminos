use super::{Key, KeyboardLayout, SpecialKey};



pub const QWERTY_SCANCODE_TO_CHAR: [Option<char>; 256] = {
    let mut map = [None; 256];
    map[0x02] = Some('1');
    map[0x03] = Some('2');
    map[0x04] = Some('3');
    map[0x05] = Some('4');
    map[0x06] = Some('5');
    map[0x07] = Some('6');
    map[0x08] = Some('7');
    map[0x09] = Some('8');
    map[0x0A] = Some('9');
    map[0x0B] = Some('0');
    map[0x0C] = Some('-');
    map[0x0D] = Some('=');
    map[0x0E] = Some('\x08');
    map[0x0F] = Some('\t');
    map[0x10] = Some('q');
    map[0x11] = Some('w');
    map[0x12] = Some('e');
    map[0x13] = Some('r');
    map[0x14] = Some('t');
    map[0x15] = Some('y');
    map[0x16] = Some('u');
    map[0x17] = Some('i');
    map[0x18] = Some('o');
    map[0x19] = Some('p');
    map[0x1A] = Some('[');
    map[0x1B] = Some(']');
    map[0x1C] = Some('\n');
    map[0x1E] = Some('a');
    map[0x1F] = Some('s');
    map[0x20] = Some('d');
    map[0x21] = Some('f');
    map[0x22] = Some('g');
    map[0x23] = Some('h');
    map[0x24] = Some('j');
    map[0x25] = Some('k');
    map[0x26] = Some('l');
    map[0x27] = Some(';');
    map[0x28] = Some('\'');
    map[0x29] = Some('`');
    map[0x2B] = Some('\\');
    map[0x2C] = Some('z');
    map[0x2D] = Some('x');
    map[0x2E] = Some('c');
    map[0x2F] = Some('v');
    map[0x30] = Some('b');
    map[0x31] = Some('n');
    map[0x32] = Some('m');
    map[0x33] = Some(',');
    map[0x34] = Some('.');
    map[0x35] = Some('/');
    map[0x39] = Some(' ');
    map
};

pub struct Qwerty;

impl KeyboardLayout for Qwerty {
    fn from_scamcode(scancode: u8) -> Option<Key> {
        if scancode == 0 {
            return None;
        }

        if let Some(c) = QWERTY_SCANCODE_TO_CHAR[scancode as usize] {
            return Some(Key::Char(c));
        }

        match scancode {
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
            _ => None,
        }
    }
}