use core::ffi::*;
use crate::io::port::Fd;
use alloc::ffi::CString;

pub const OS_WRITE_TO_FAILED: c_int = 1;

pub const OS_OPEN_READ_FLAG: c_int = 1;
pub const OS_OPEN_WRITE_FLAG: c_int = 2;
pub const OS_OPEN_BINARY_FLAG: c_int = 3;


pub unsafe extern "C" fn os_write_to(fd: c_int, msg: *const c_char) -> c_int {
    let mut fd = Fd(fd as usize);
    let string = CStr::from_ptr(msg);
    if let Ok(s) = string.to_str() {
        use core::fmt::Write;
        let res = fd.write_str(&s);
        return if res.is_ok() { 0 } else { OS_WRITE_TO_FAILED };
    }

    return OS_WRITE_TO_FAILED;
}


#[repr(C)]
#[derive(Copy, Clone)]
pub struct OsHandle {
    pub os_write_to: unsafe extern "C" fn(c_int, *const c_char) -> c_int,
    pub os_write_to_failed: c_int,
//    pub os_open_read_flag: c_int,
//    pub os_open_write_flag: c_int,
//    pub os_open_binary_flag: c_int
}


impl OsHandle {
    pub fn new() -> Self {
        OsHandle {
            os_write_to,
            os_write_to_failed: OS_WRITE_TO_FAILED,
  //          os_open_read_flag: OS_OPEN_READ_FLAG,
  //          os_open_write_flag: OS_OPEN_WRITE_FLAG,
  //          os_open_binary_flag: OS_OPEN_BINARY_FLAG
        }
    }
}
