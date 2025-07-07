/*!
 * Luna Logging System - Structured logging for the portable app
 */

use anyhow::Result;
use std::path::PathBuf;
use tracing::{Level, info};
use tracing_subscriber::{
    fmt::{self, time::ChronoUtc},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter, Layer,
};

/// Setup logging system for Luna
pub fn setup_logging() -> Result<()> {
    setup_logging_with_config(&LoggingConfig::default())
}

/// Setup logging with custom configuration
pub fn setup_logging_with_config(config: &LoggingConfig) -> Result<()> {
    // Create log directory if it doesn't exist
    if let Some(log_dir) = config.log_file.parent() {
        std::fs::create_dir_all(log_dir)?;
    }
    
    // Environment filter for log levels
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| {
            match config.level {
                LogLevel::Trace => "luna=trace",
                LogLevel::Debug => "luna=debug", 
                LogLevel::Info => "luna=info",
                LogLevel::Warn => "luna=warn",
                LogLevel::Error => "luna=error",
            }.into()
        });
    
    // Console layer
    let console_layer = fmt::layer()
        .with_ansi(config.use_colors)
        .with_timer(ChronoUtc::rfc3339())
        .with_target(config.show_targets)
        .with_thread_ids(config.show_thread_ids)
        .with_thread_names(config.show_thread_names)
        .with_filter(env_filter.clone());
    
    // File layer (if log file is specified)
    let file_layer = if config.log_to_file {
        let file_appender = tracing_appender::rolling::daily(
            config.log_file.parent().unwrap_or(&PathBuf::from(".")),
            config.log_file.file_name().unwrap_or(std::ffi::OsStr::new("luna.log"))
        );
        
        Some(fmt::layer()
            .with_writer(file_appender)
            .with_ansi(false) // No colors in file
            .with_timer(ChronoUtc::rfc3339())
            .with_target(true)
            .with_thread_ids(true)
            .with_filter(env_filter))
    } else {
        None
    };
    
    // Initialize subscriber
    let subscriber = tracing_subscriber::registry()
        .with(console_layer);
    
    if let Some(file_layer) = file_layer {
        subscriber.with(file_layer).init();
    } else {
        subscriber.init();
    }
    
    info!("Luna logging system initialized");
    info!("Log level: {:?}", config.level);
    if config.log_to_file {
        info!("Logging to file: {:?}", config.log_file);
    }
    
    Ok(())
}

/// Logging configuration
#[derive(Debug, Clone)]
pub struct LoggingConfig {
    pub level: LogLevel,
    pub log_to_file: bool,
    pub log_file: PathBuf,
    pub use_colors: bool,
    pub show_targets: bool,
    pub show_thread_ids: bool,
    pub show_thread_names: bool,
    pub max_file_size_mb: u64,
    pub max_files: usize,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: LogLevel::Info,
            log_to_file: true,
            log_file: get_default_log_path(),
            use_colors: true,
            show_targets: false,
            show_thread_ids: false,
            show_thread_names: false,
            max_file_size_mb: 10,
            max_files: 5,
        }
    }
}

/// Log levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl From<LogLevel> for Level {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Trace => Level::TRACE,
            LogLevel::Debug => Level::DEBUG,
            LogLevel::Info => Level::INFO,
            LogLevel::Warn => Level::WARN,
            LogLevel::Error => Level::ERROR,
        }
    }
}

/// Get default log file path
fn get_default_log_path() -> PathBuf {
    // For portable app, place logs next to executable
    let exe_dir = std::env::current_exe()
        .map(|p| p.parent().unwrap_or(&std::path::Path::new(".")).to_path_buf())
        .unwrap_or_else(|_| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    
    exe_dir.join("logs").join("luna.log")
}

/// Create a structured log entry
#[macro_export]
macro_rules! log_action {
    ($level:expr, $action:expr, $($field:ident = $value:expr),*) => {
        tracing::event!(
            $level,
            action = $action,
            $($field = $value,)*
        );
    };
}

/// Log command execution
pub fn log_command_start(command: &str) {
    info!(
        command = command,
        event = "command_start",
        "Starting command execution"
    );
}

pub fn log_command_success(command: &str, duration_ms: u64) {
    info!(
        command = command,
        duration_ms = duration_ms,
        event = "command_success",
        "Command completed successfully"
    );
}

pub fn log_command_error(command: &str, error: &str, duration_ms: u64) {
    tracing::error!(
        command = command,
        error = error,
        duration_ms = duration_ms,
        event = "command_error",
        "Command failed"
    );
}

/// Log AI processing
pub fn log_ai_processing(model: &str, operation: &str, processing_time_ms: u64, success: bool) {
    if success {
        info!(
            model = model,
            operation = operation,
            processing_time_ms = processing_time_ms,
            event = "ai_processing_success",
            "AI processing completed"
        );
    } else {
        tracing::warn!(
            model = model,
            operation = operation,
            processing_time_ms = processing_time_ms,
            event = "ai_processing_failed",
            "AI processing failed"
        );
    }
}

/// Log safety events
pub fn log_safety_block(command: &str, reason: &str) {
    tracing::warn!(
        command = command,
        reason = reason,
        event = "safety_block",
        "Command blocked by safety system"
    );
}

/// Log performance metrics
pub fn log_performance_warning(metric: &str, value: f64, threshold: f64) {
    tracing::warn!(
        metric = metric,
        value = value,
        threshold = threshold,
        event = "performance_warning",
        "Performance metric exceeded threshold"
    );
}

/// Log system events
pub fn log_system_event(event_type: &str, details: &str) {
    info!(
        event_type = event_type,
        details = details,
        event = "system_event",
        "System event occurred"
    );
}

/// Create a log context for operations
pub struct LogContext {
    operation: String,
    start_time: std::time::Instant,
    fields: std::collections::HashMap<String, String>,
}

impl LogContext {
    pub fn new(operation: &str) -> Self {
        Self {
            operation: operation.to_string(),
            start_time: std::time::Instant::now(),
            fields: std::collections::HashMap::new(),
        }
    }
    
    pub fn add_field<V: ToString>(&mut self, key: &str, value: V) {
        self.fields.insert(key.to_string(), value.to_string());
    }
    
    pub fn success(self) {
        let duration_ms = self.start_time.elapsed().as_millis() as u64;
        
        let mut event = tracing::info_span!("operation_complete");
        event.record("operation", &self.operation);
        event.record("duration_ms", &duration_ms);
        event.record("success", &true);
        
        for (key, value) in &self.fields {
            event.record(key, value);
        }
        
        tracing::info!(parent: &event, "Operation completed successfully");
    }
    
    pub fn error(self, error: &str) {
        let duration_ms = self.start_time.elapsed().as_millis() as u64;
        
        let mut event = tracing::error_span!("operation_failed");
        event.record("operation", &self.operation);
        event.record("duration_ms", &duration_ms);
        event.record("success", &false);
        event.record("error", &error);
        
        for (key, value) in &self.fields {
            event.record(key, value);
        }
        
        tracing::error!(parent: &event, "Operation failed: {}", error);
    }
}

/// Get current log level
pub fn get_log_level() -> LogLevel {
    // Check environment variable or use default
    match std::env::var("LUNA_LOG_LEVEL").as_deref() {
        Ok("trace") => LogLevel::Trace,
        Ok("debug") => LogLevel::Debug,
        Ok("info") => LogLevel::Info,
        Ok("warn") => LogLevel::Warn,
        Ok("error") => LogLevel::Error,
        _ => LogLevel::Info,
    }
}

/// Flush logs (ensure all logs are written)
pub fn flush_logs() {
    // In real implementation, would flush all appenders
    std::io::Write::flush(&mut std::io::stdout()).ok();
    std::io::Write::flush(&mut std::io::stderr()).ok();
}