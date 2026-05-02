use alloc::collections::VecDeque;
use spin::Mutex;

pub static STDIN_BUFFER: Mutex<VecDeque<u8>> = Mutex::new(VecDeque::new());

pub fn pop_back() -> Option<u8> {
    if let Some(mut lock) = STDIN_BUFFER.try_lock() {
        lock.pop_back()
    } else {
        None
    }
}

pub fn pop() -> Option<u8> {
    if let Some(mut lock) = STDIN_BUFFER.try_lock() {
        lock.pop_front()
    } else {
        None
    }
}

pub fn blocking_pop() -> u8 {
    loop {
        if let Some(b) = pop() {
            return b;
        }
        x86_64::instructions::hlt();
    }
}

pub fn read(buf: &mut [u8]) -> usize {
    let mut bytes_read = 0;
    if let Some(mut lock) = STDIN_BUFFER.try_lock() {
        while bytes_read < buf.len() && !lock.is_empty() {
            if let Some(b) = lock.pop_front() {
                buf[bytes_read] = b;
                bytes_read += 1;
            }
        }
    }
    bytes_read
}

pub fn blocking_read(buf: &mut [u8]) -> usize {
    let mut bytes_read = 0;
    loop {
        bytes_read += read(&mut buf[bytes_read..]);
        if bytes_read > 0 {
            break;
        }
        x86_64::instructions::hlt();
    }
    bytes_read
}

pub fn write(buf: &[u8]) {
    STDIN_BUFFER.lock().extend(buf.iter());
}

pub fn flush(out: &mut impl core::fmt::Write) {
    let mut temp_buf = VecDeque::new();
    if let Some(mut lock) = STDIN_BUFFER.try_lock() {
        core::mem::swap(&mut *lock, &mut temp_buf);
    }

    while let Some(b) = temp_buf.pop_front() {
        let c = b as char;
        let _ = out.write_char(c);
    }
}
