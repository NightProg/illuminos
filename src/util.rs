

pub struct IterPtr<T>   {
    ptr: *mut T,
    size: usize,
    current: usize
}

impl<T> IterPtr<T> {
    pub fn new(ptr: *mut T, size: usize) -> Self {
        Self {
            ptr,
            size,
            current: 0
        }
    }

}

impl<T> Iterator for IterPtr<T> {
    type Item = *mut T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current >= self.size {
            None
        } else {
            self.current += 1;
            Some(unsafe {
                self.ptr.add(self.current)
            })
        
        }
    }
}




#[macro_export]
macro_rules! print {
    (serial, $($arg:tt)*) => ({
        $crate::print_serial!($($arg)*);
    });
    (textbuffer, $($arg:tt)*) => ({
        $crate::print_textbuffer!($($arg)*);
    })
}

#[macro_export]
macro_rules! println {
    (serial, $($arg:tt)*) => ({
        $crate::println_serial!($($arg)*);
    });
    (textbuffer, $($arg:tt)*) => ({
        $crate::println_textbuffer!($($arg)*);
    })
}

