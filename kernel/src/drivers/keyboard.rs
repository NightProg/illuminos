use alloc::collections::VecDeque;
use alloc::sync::Arc;
use alloc::vec::Vec;
use lazy_static::lazy_static;
use pc_keyboard::{HandleControl, KeyEvent, Keyboard, ScancodeSet1, ScancodeSet2, layouts};
use spin::Mutex;

use crate::sync::mutex::TimeoutMutex;

pub static mut KEYBOARD_STREAM: Mutex<KeyboardStream> = Mutex::new(KeyboardStream::new());

pub static KEYBOARD: TimeoutMutex<Keyboard<layouts::Azerty, ScancodeSet1>> = TimeoutMutex::new(
    Keyboard::new(ScancodeSet1::new(), layouts::Azerty, HandleControl::Ignore),
);

#[derive(Clone)]
pub struct KeyboardStream {
    keys: VecDeque<KeyEvent>,
    callback: Vec<Arc<dyn Fn(&KeyEvent)>>,
    have_changed: bool,
}

unsafe impl Send for KeyboardStream {}
unsafe impl Sync for KeyboardStream {}

impl KeyboardStream {
    pub const fn new() -> Self {
        Self {
            keys: VecDeque::new(),
            callback: Vec::new(),
            have_changed: false,
        }
    }

    pub fn init_capacity(&mut self) {
        self.keys.reserve(128);
        self.callback.reserve(8);
    }

    pub fn add_callback<F>(&mut self, callback: F)
    where
        F: Fn(&KeyEvent) + 'static,
    {
        self.callback.push(Arc::new(callback));
    }

    pub fn push_key(&mut self, key: KeyEvent) {
        if self.keys.len() >= 64 {
            self.keys.pop_front();
        }
        self.keys.push_back(key.clone());
        self.have_changed = true;
    }

    pub fn handled_keys(&mut self) {
        if self.have_changed {
            self.have_changed = false;
        }
    }

    pub fn has_changed(&self) -> bool {
        self.have_changed
    }

    pub fn get_changed_key_event(&self) -> Option<KeyEvent> {
        if self.have_changed && !self.keys.is_empty() {
            Some(self.keys.back().cloned().unwrap())
        } else {
            None
        }
    }

    pub fn get_keys(&self) -> &VecDeque<KeyEvent> {
        &self.keys
    }

    pub fn clear_keys(&mut self) {
        self.keys.clear();
    }

    pub fn pop(&mut self) -> Option<KeyEvent> {
        self.keys.pop_front()
    }
}

impl Iterator for KeyboardStream {
    type Item = KeyEvent;

    fn next(&mut self) -> Option<Self::Item> {
        self.keys.pop_front()
    }
}
