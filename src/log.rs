use core::fmt::Display;

use crate::{graphic::{BLUE, RED, YELLOW}, print, println};


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogOutput {
    Serial,
    TextBuffer,
}

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

pub static LOGGER: spin::Mutex<Logger> = spin::Mutex::new(Logger::new(LogLevel::Info, LogOutput::Serial));

pub fn set_log_level(level: LogLevel) {
    LOGGER.lock().level = level;
}

pub fn set_log_output(output: LogOutput) {
    LOGGER.lock().output = output;
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
    output: LogOutput,
}

impl Logger {
    pub const fn new(level: LogLevel, output: LogOutput) -> Self {
        Logger {
            level,
            output,
        }
    }
}

impl core::fmt::Write for Logger {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        if self.output == LogOutput::Serial {
            print!(serial, "{}", s);
        } else if self.output == LogOutput::TextBuffer {
            let old = crate::graphic::console::get_global_text_buffer_color();
            crate::graphic::console::set_global_text_buffer_color(match self.level {
                LogLevel::Info => BLUE,
                LogLevel::Warning => YELLOW,
                LogLevel::Error => RED,
            });

            print!(textbuffer, "{}", s);
            crate::graphic::console::set_global_text_buffer_color(old);
        }
        Ok(())
    }
}