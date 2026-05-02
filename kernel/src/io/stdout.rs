use alloc::collections::VecDeque;
use spin::Mutex;

pub static STDOUT_BUFFER: Mutex<VecDeque<u8>> = Mutex::new(VecDeque::new());

pub fn write(buf: &[u8]) {
    STDOUT_BUFFER.lock().extend(buf.iter());
}

pub fn flush(out: &mut impl core::fmt::Write) {
    let mut temp_buf = VecDeque::new();
    if let Some(mut lock) = STDOUT_BUFFER.try_lock() {
        core::mem::swap(&mut *lock, &mut temp_buf);
    }

    while let Some(b) = temp_buf.pop_front() {
        let c = b as char;
        let _ = out.write_char(c);
    }
}
