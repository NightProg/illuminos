pub mod session;

use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use alloc::{string::String, sync::Arc, vec::Vec};
use pc_keyboard::{DecodedKey, KeyCode, KeyEvent};

use crate::{
    drivers::keyboard::KEYBOARD,
    graphic::{
        Color,
        font::{FONT_DEFAULT, Psf2Font},
        framebuffer::RawFrameBuffer,
        text_buffer::TextBuffer,
    },
    println_serial,
    sync::{mutex::TimeoutMutex, ring_buffer::AtomicCircularRingBuffer, wait_queue::AtomicWaitQueue},
};

#[derive(Clone, Copy)]
pub enum Mode {
    Raw,
    Cooked,
}

pub struct Tty {
    mode: Mode,
    cursor: AtomicUsize,
    buffer: Arc<AtomicCircularRingBuffer<1024>>,
    wait_queue: AtomicWaitQueue,
    text_buffer: TimeoutMutex<TextBuffer<Psf2Font>>,
    ready: AtomicBool,
}

impl Tty {
    pub fn new(mode: Mode) -> Tty {
        Tty {
            mode,
            cursor: AtomicUsize::new(0),
            buffer: Arc::new(AtomicCircularRingBuffer::new()),
            wait_queue: AtomicWaitQueue::new(),
            text_buffer: TimeoutMutex::new(TextBuffer::new(
                &*FONT_DEFAULT,
                Color::white(),
                Color::black(),
            )),
            ready: AtomicBool::new(false),
        }
    }

    pub fn write_buf(&self, buf: &[u8]) {
        self.text_buffer
            .lock()
            .put_str(core::str::from_utf8(buf).unwrap_or("<?>"));
    }

    pub fn is_ready(&self) -> bool {
        self.ready.load(Ordering::Relaxed)
    }

    pub fn reset_ready(&self) {
        self.ready.store(false, Ordering::SeqCst);
    }

    pub fn pop_n(&self, n: usize) -> Vec<u8> {
        let res = self.buffer.pop_n(n);
        if self.buffer.is_empty() {
            self.ready.store(false, Ordering::SeqCst);
        }
        res
    }

    pub fn pop_all(&self) -> Vec<u8> {
        self.pop_n(self.buffer.len())
    }

    pub fn add_key(&self, key: KeyEvent) {
        match self.mode {
            Mode::Cooked => {
                let decoded_key = KEYBOARD.lock().process_keyevent(key.clone());
                if let Some(dkey) = decoded_key {
                    match dkey {
                        DecodedKey::Unicode(u) => {
                            if u == '\n' {
                                self.handle_return();
                            } else {
                                self.buffer.push(u as u8);
                                self.text_buffer.lock().put_char(u);
                            }
                        }
                        DecodedKey::RawKey(code) => match code {
                            KeyCode::Return => {
                                self.handle_return();
                            }
                            KeyCode::Backspace => {
                                if !self.buffer.is_empty() {
                                    self.buffer.pop_last();
                                    self.text_buffer.lock().backspace();
                                }
                            }
                            KeyCode::Tab => {
                                self.buffer.push(b'\t');
                                self.text_buffer.lock().put_char('\t')
                            }
                            code => println_serial!("skipping {:?}", code),
                        },
                    }
                }
            }
            Mode::Raw => {
                self.buffer.push(key.code as u8);
                self.wait_queue.wake_all();
                self.ready.store(true, Ordering::SeqCst);
            }
        }
    }

    fn handle_return(&self) {
        println_serial!("handle return");
        self.buffer.push(b'\n');
        self.wait_queue.wake_all();
        self.text_buffer.lock().put_char('\n');
        self.ready.store(true, Ordering::SeqCst);
    }
    pub fn wait(&self) {
        self.wait_queue.wait();
    }
}
