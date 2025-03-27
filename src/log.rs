use core::fmt::Display;

use crate::{drivers::vga, print, println};


pub enum LogLevel {
    Info,
    Warning,
    Error,
}

impl Display for LogLevel {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Warning => write!(f, "WARNING"),
            LogLevel::Error => write!(f, "ERROR"),
        }
    }
}

#[macro_export]
macro_rules! log {
    ($level:expr, $($arg:tt)*) => {
        {
            use core::fmt::Write;
            $crate::log::set_log_level($level);
            let _ = write!(crate::log::LOGGER.lock(), "{}:{}:{}: ", file!(), line!(), column!());
            let _ = write!(crate::log::LOGGER.lock(), "[{}]: ", $level);
            let _ = write!(crate::log::LOGGER.lock(), $($arg)*);
            let _ = writeln!(crate::log::LOGGER.lock());
        }
    };
}

pub static LOGGER: spin::Mutex<Logger> = spin::Mutex::new(Logger::new(LogLevel::Info));

pub fn set_log_level(level: LogLevel) {
    LOGGER.lock().level = level;
}


#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {
        $crate::log!($crate::log::LogLevel::Info, $($arg)*);
    };
}


#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {
        $crate::log!($crate::log::LogLevel::Warning, $($arg)*);
    };
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {
        $crate::log!($crate::log::LogLevel::Error, $($arg)*);
    };
}


pub struct Logger {
    level: LogLevel,
}

impl Logger {
    pub const fn new(level: LogLevel) -> Self {
        Logger {
            level,
        }
    }
}

impl core::fmt::Write for Logger {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let old = vga::get_color();
        match self.level {
            LogLevel::Info => {
                vga::set_color(vga::Color::LightBlue, vga::Color::Black);
            }
            LogLevel::Warning => {
                vga::set_color(vga::Color::Yellow, vga::Color::Black);
            }
            LogLevel::Error => {
                vga::set_color(vga::Color::Red, vga::Color::Black);
            }
        }
        print!("{}", s);
        vga::set_color(old.foreground(), old.background());
        Ok(())
    }
}