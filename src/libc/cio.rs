use crate::io::port::{Fd, PORTS};

// pub extern "C" fn write(fd: i32, buf: *const u8, count: usize) -> isize {
//     let fd = fd as usize;
//     let buf = unsafe { std::slice::from_raw_parts(buf, count) };
//     PORTS.lock().get_port(fd)
// }
