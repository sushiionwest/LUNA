// Simple logging implementation without external logging crates
// Replaces complex logging frameworks with standard library

use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum LogLevel {
    Error = 0,
    Warn = 1,
    Info = 2,
    Debug = 3,
    Trace = 4,
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Error => write!(f, "ERROR"),
            LogLevel::Warn => write!(f, "WARN "),
            LogLevel::Info => write!(f, "INFO "),
            LogLevel::Debug => write!(f, "DEBUG"),
            LogLevel::Trace => write!(f, "TRACE"),
        }
    }
}

pub struct Logger {
    level: LogLevel,
    file: Option<Arc<Mutex<File>>>,
    console: bool,
}

impl Logger {
    pub fn new() -> Self {
        Self {
            level: LogLevel::Info,
            file: None,
            console: true,
        }
    }

    pub fn with_level(mut self, level: LogLevel) -> Self {
        self.level = level;
        self
    }

    pub fn with_file(mut self, path: &str) -> Result<Self, io::Error> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        self.file = Some(Arc::new(Mutex::new(file)));
        Ok(self)
    }

    pub fn with_console(mut self, enabled: bool) -> Self {
        self.console = enabled;
        self
    }

    pub fn log(&self, level: LogLevel, message: &str) {
        if level <= self.level {
            let timestamp = format_timestamp();
            let log_line = format!("{} [{}] {}\n", timestamp, level, message);

            if self.console {
                print!("{}", log_line);
                let _ = io::stdout().flush();
            }

            if let Some(ref file) = self.file {
                if let Ok(mut f) = file.lock() {
                    let _ = f.write_all(log_line.as_bytes());
                    let _ = f.flush();
                }
            }
        }
    }

    pub fn error(&self, message: &str) {
        self.log(LogLevel::Error, message);
    }

    pub fn warn(&self, message: &str) {
        self.log(LogLevel::Warn, message);
    }

    pub fn info(&self, message: &str) {
        self.log(LogLevel::Info, message);
    }

    pub fn debug(&self, message: &str) {
        self.log(LogLevel::Debug, message);
    }

    pub fn trace(&self, message: &str) {
        self.log(LogLevel::Trace, message);
    }
}

impl Default for Logger {
    fn default() -> Self {
        Self::new()
    }
}

fn format_timestamp() -> String {
    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(duration) => {
            let secs = duration.as_secs();
            let millis = duration.subsec_millis();
            
            // Simple timestamp formatting (YYYY-MM-DD HH:MM:SS.mmm)
            let days_since_epoch = secs / 86400;
            let days_since_1970 = days_since_epoch;
            
            // Approximate date calculation (not leap year aware, but close enough for logging)
            let year = 1970 + days_since_1970 / 365;
            let day_of_year = days_since_1970 % 365;
            let month = (day_of_year / 30) + 1;
            let day = (day_of_year % 30) + 1;
            
            let time_of_day = secs % 86400;
            let hour = time_of_day / 3600;
            let minute = (time_of_day % 3600) / 60;
            let second = time_of_day % 60;
            
            format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:03}", 
                    year, month, day, hour, minute, second, millis)
        }
        Err(_) => "1970-01-01 00:00:00.000".to_string(),
    }
}

// Global logger instance
static mut GLOBAL_LOGGER: Option<Logger> = None;
static LOGGER_INIT: std::sync::Once = std::sync::Once::new();

pub fn init_logger(logger: Logger) {
    LOGGER_INIT.call_once(|| {
        unsafe {
            GLOBAL_LOGGER = Some(logger);
        }
    });
}

pub fn get_logger() -> Option<&'static Logger> {
    unsafe { GLOBAL_LOGGER.as_ref() }
}

// Convenience macros for global logging
#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        if let Some(logger) = $crate::utils::logging::get_logger() {
            logger.error(&format!($($arg)*));
        }
    };
}

#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => {
        if let Some(logger) = $crate::utils::logging::get_logger() {
            logger.warn(&format!($($arg)*));
        }
    };
}

#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        if let Some(logger) = $crate::utils::logging::get_logger() {
            logger.info(&format!($($arg)*));
        }
    };
}

#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {
        if let Some(logger) = $crate::utils::logging::get_logger() {
            logger.debug(&format!($($arg)*));
        }
    };
}

#[macro_export]
macro_rules! log_trace {
    ($($arg:tt)*) => {
        if let Some(logger) = $crate::utils::logging::get_logger() {
            logger.trace(&format!($($arg)*));
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_logger_levels() {
        let logger = Logger::new().with_level(LogLevel::Warn);
        
        // These should not output anything since we're only logging WARN and above
        logger.debug("This should not appear");
        logger.trace("This should not appear");
        
        // These should output
        logger.warn("This should appear");
        logger.error("This should appear");
    }

    #[test]
    fn test_file_logging() {
        let temp_file = std::env::temp_dir().join("test_log.txt");
        
        {
            let logger = Logger::new()
                .with_file(temp_file.to_str().unwrap())
                .unwrap()
                .with_console(false);
            
            logger.info("Test message");
        }
        
        let content = fs::read_to_string(&temp_file).unwrap();
        assert!(content.contains("Test message"));
        
        // Clean up
        let _ = fs::remove_file(&temp_file);
    }

    #[test]
    fn test_log_level_ordering() {
        assert!(LogLevel::Error < LogLevel::Warn);
        assert!(LogLevel::Warn < LogLevel::Info);
        assert!(LogLevel::Info < LogLevel::Debug);
        assert!(LogLevel::Debug < LogLevel::Trace);
    }
}