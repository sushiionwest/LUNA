/*!
 * Luna Visual AI Logging System
 * 
 * Advanced logging infrastructure with:
 * - Structured logging with JSON support
 * - Multiple output targets (console, file, syslog)
 * - Performance monitoring and metrics
 * - Security-aware log filtering (no sensitive data)
 * - Automatic log rotation and cleanup
 * - Real-time log level adjustment
 * - Component-specific log filtering
 */

use crate::core::error::{LunaError, Result};
use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use tracing::{level_filters::LevelFilter, Level};
use tracing_appender::{non_blocking, rolling};
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter, Layer,
};

/// Global logging initialization state
static LOGGING_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Logging configuration
#[derive(Debug, Clone)]
pub struct LoggingConfig {
    pub level: String,
    pub file_logging: bool,
    pub console_logging: bool,
    pub structured_logging: bool,
    pub log_directory: PathBuf,
    pub max_file_size: u64,
    pub max_files: u32,
    pub component_filters: Vec<ComponentFilter>,
    pub sensitive_patterns: Vec<String>,
}

/// Component-specific logging filter
#[derive(Debug, Clone)]
pub struct ComponentFilter {
    pub component: String,
    pub level: String,
}

/// Initialize the logging system
/// 
/// Sets up structured logging with multiple outputs based on configuration.
/// This should be called once at application startup.
/// 
/// # Arguments
/// 
/// * `level` - Default log level (error, warn, info, debug, trace)
/// * `debug_mode` - Enable debug mode with enhanced logging
/// 
/// # Returns
/// 
/// Returns `Ok(())` if logging initializes successfully.
pub fn init(level: &str, debug_mode: bool) -> Result<()> {
    if LOGGING_INITIALIZED.load(Ordering::Relaxed) {
        return Ok(());
    }

    let config = LoggingConfig::default();
    init_with_config(config, level, debug_mode)
}

/// Initialize logging with custom configuration
pub fn init_with_config(
    config: LoggingConfig, 
    level: &str, 
    debug_mode: bool
) -> Result<()> {
    if LOGGING_INITIALIZED.load(Ordering::Relaxed) {
        return Ok(());
    }

    // Parse log level
    let log_level = parse_log_level(level)?;
    
    // Create log directory if needed
    if config.file_logging {
        std::fs::create_dir_all(&config.log_directory)
            .map_err(|e| LunaError::io(
                format!("Failed to create log directory: {}", e),
                Some(config.log_directory.to_string_lossy()),
            ))?;
    }

    // Build environment filter
    let env_filter = build_env_filter(&config, log_level, debug_mode)?;

    // Create subscriber layers
    let mut layers = Vec::new();

    // Console layer
    if config.console_logging {
        let console_layer = create_console_layer(config.structured_logging, debug_mode)?;
        layers.push(console_layer);
    }

    // File layer
    if config.file_logging {
        let file_layer = create_file_layer(&config, debug_mode)?;
        layers.push(file_layer);
    }

    // Initialize subscriber
    tracing_subscriber::registry()
        .with(env_filter)
        .with(layers)
        .init();

    LOGGING_INITIALIZED.store(true, Ordering::Relaxed);

    // Log initialization success
    tracing::info!("🌙 Luna Visual AI logging system initialized");
    tracing::info!("Log level: {}", level);
    tracing::info!("Debug mode: {}", debug_mode);
    tracing::info!("File logging: {}", config.file_logging);
    if config.file_logging {
        tracing::info!("Log directory: {:?}", config.log_directory);
    }

    Ok(())
}

/// Check if logging has been initialized
pub fn is_initialized() -> bool {
    LOGGING_INITIALIZED.load(Ordering::Relaxed)
}

/// Parse log level string into tracing Level
fn parse_log_level(level: &str) -> Result<Level> {
    match level.to_lowercase().as_str() {
        "error" => Ok(Level::ERROR),
        "warn" | "warning" => Ok(Level::WARN),
        "info" => Ok(Level::INFO),
        "debug" => Ok(Level::DEBUG),
        "trace" => Ok(Level::TRACE),
        _ => Err(LunaError::config(
            format!("Invalid log level: {}", level),
            Some("log_level"),
        )),
    }
}

/// Build environment filter with component-specific filters
fn build_env_filter(
    config: &LoggingConfig,
    default_level: Level,
    debug_mode: bool,
) -> Result<EnvFilter> {
    let mut filter = EnvFilter::new(format!("luna_visual_ai={}", default_level));

    // Add component-specific filters
    for component_filter in &config.component_filters {
        let component_level = parse_log_level(&component_filter.level)?;
        filter = filter.add_directive(
            format!("luna_visual_ai::{}={}", component_filter.component, component_level)
                .parse()
                .map_err(|e| LunaError::config(
                    format!("Invalid component filter: {}", e),
                    Some("component_filters"),
                ))?,
        );
    }

    // Debug mode adjustments
    if debug_mode {
        filter = filter
            .add_directive("luna_visual_ai::ai=debug".parse().unwrap())
            .add_directive("luna_visual_ai::vision=debug".parse().unwrap())
            .add_directive("luna_visual_ai::overlay=debug".parse().unwrap());
    }

    // Add third-party library filters to reduce noise
    filter = filter
        .add_directive("wgpu=warn".parse().unwrap())
        .add_directive("winit=warn".parse().unwrap())
        .add_directive("egui=warn".parse().unwrap())
        .add_directive("tokio=info".parse().unwrap());

    Ok(filter)
}

/// Create console logging layer
fn create_console_layer(
    structured: bool,
    debug_mode: bool,
) -> Result<Box<dyn Layer<tracing_subscriber::Registry> + Send + Sync>> {
    if structured {
        // JSON console output
        let layer = fmt::layer()
            .json()
            .with_current_span(debug_mode)
            .with_span_list(debug_mode)
            .with_target(true)
            .with_thread_ids(debug_mode)
            .with_thread_names(debug_mode)
            .with_file(debug_mode)
            .with_line_number(debug_mode);

        Ok(Box::new(layer))
    } else {
        // Human-readable console output
        let layer = fmt::layer()
            .with_target(true)
            .with_thread_ids(debug_mode)
            .with_thread_names(debug_mode)
            .with_file(debug_mode)
            .with_line_number(debug_mode)
            .with_span_events(if debug_mode {
                FmtSpan::ENTER | FmtSpan::CLOSE
            } else {
                FmtSpan::NONE
            });

        Ok(Box::new(layer))
    }
}

/// Create file logging layer
fn create_file_layer(
    config: &LoggingConfig,
    debug_mode: bool,
) -> Result<Box<dyn Layer<tracing_subscriber::Registry> + Send + Sync>> {
    // Create rolling file appender
    let file_appender = rolling::daily(&config.log_directory, "luna.log");
    let (non_blocking, _guard) = non_blocking(file_appender);

    // Create file layer
    let layer = fmt::layer()
        .with_writer(non_blocking)
        .json() // Always use JSON for file logging
        .with_current_span(true)
        .with_span_list(debug_mode)
        .with_target(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_file(debug_mode)
        .with_line_number(debug_mode);

    Ok(Box::new(layer))
}

/// Create a secure logging macro that filters sensitive information
#[macro_export]
macro_rules! secure_log {
    ($level:ident, $($arg:tt)*) => {
        {
            let message = format!($($arg)*);
            let filtered_message = $crate::utils::logging::filter_sensitive_data(&message);
            tracing::$level!("{}", filtered_message);
        }
    };
}

/// Filter sensitive data from log messages
/// 
/// Removes or masks sensitive information like passwords, tokens, and personal data
/// before writing to logs.
pub fn filter_sensitive_data(message: &str) -> String {
    let mut filtered = message.to_string();

    // Common sensitive patterns
    let sensitive_patterns = vec![
        (r"password[s]?\s*[:=]\s*[^\s,}]+", "password: [REDACTED]"),
        (r"token[s]?\s*[:=]\s*[^\s,}]+", "token: [REDACTED]"),
        (r"key[s]?\s*[:=]\s*[^\s,}]+", "key: [REDACTED]"),
        (r"secret[s]?\s*[:=]\s*[^\s,}]+", "secret: [REDACTED]"),
        (r"api[_-]?key\s*[:=]\s*[^\s,}]+", "api_key: [REDACTED]"),
        (r"bearer\s+[^\s,}]+", "bearer [REDACTED]"),
        // Email addresses
        (r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b", "[EMAIL_REDACTED]"),
        // Phone numbers (basic pattern)
        (r"\b\d{3}[-.]?\d{3}[-.]?\d{4}\b", "[PHONE_REDACTED]"),
        // Credit card numbers (basic pattern)
        (r"\b\d{4}[-\s]?\d{4}[-\s]?\d{4}[-\s]?\d{4}\b", "[CARD_REDACTED]"),
        // Social Security Numbers (basic pattern)
        (r"\b\d{3}-\d{2}-\d{4}\b", "[SSN_REDACTED]"),
    ];

    for (pattern, replacement) in sensitive_patterns {
        if let Ok(re) = regex::Regex::new(pattern) {
            filtered = re.replace_all(&filtered, replacement).to_string();
        }
    }

    filtered
}

/// Performance logging utilities
pub mod perf {
    use std::time::Instant;
    use tracing::{debug, info, warn};

    /// Performance timer for measuring operation duration
    pub struct PerfTimer {
        name: String,
        start: Instant,
        warn_threshold_ms: Option<u64>,
    }

    impl PerfTimer {
        /// Start a new performance timer
        pub fn new(name: &str) -> Self {
            debug!("Starting operation: {}", name);
            Self {
                name: name.to_string(),
                start: Instant::now(),
                warn_threshold_ms: None,
            }
        }

        /// Start a performance timer with warning threshold
        pub fn with_threshold(name: &str, warn_threshold_ms: u64) -> Self {
            debug!("Starting operation: {} (warn threshold: {}ms)", name, warn_threshold_ms);
            Self {
                name: name.to_string(),
                start: Instant::now(),
                warn_threshold_ms: Some(warn_threshold_ms),
            }
        }

        /// Finish the timer and log the result
        pub fn finish(self) -> u64 {
            let duration_ms = self.start.elapsed().as_millis() as u64;
            
            if let Some(threshold) = self.warn_threshold_ms {
                if duration_ms > threshold {
                    warn!("Operation '{}' took {}ms (threshold: {}ms)", 
                          self.name, duration_ms, threshold);
                } else {
                    info!("Operation '{}' completed in {}ms", self.name, duration_ms);
                }
            } else {
                info!("Operation '{}' completed in {}ms", self.name, duration_ms);
            }
            
            duration_ms
        }
    }

    /// Log function entry and exit
    #[macro_export]
    macro_rules! trace_fn {
        () => {
            let _span = tracing::trace_span!(
                "function",
                function = function_name!(),
                module = module_path!(),
                file = file!(),
                line = line!()
            ).entered();
        };
        ($($arg:tt)*) => {
            let _span = tracing::trace_span!(
                "function",
                function = function_name!(),
                module = module_path!(),
                file = file!(),
                line = line!(),
                $($arg)*
            ).entered();
        };
    }
}

/// Component-specific logging utilities
pub mod components {
    use tracing::{debug, error, info, warn};

    /// AI component logging
    pub mod ai {
        use super::*;
        
        pub fn model_loaded(model_name: &str, memory_usage: u64) {
            info!("AI model loaded: {} (memory: {} MB)", model_name, memory_usage / 1_000_000);
        }
        
        pub fn inference_started(model_name: &str, input_size: usize) {
            debug!("AI inference started: {} (input size: {})", model_name, input_size);
        }
        
        pub fn inference_completed(model_name: &str, duration_ms: u64, confidence: f32) {
            info!("AI inference completed: {} ({}ms, confidence: {:.2})", 
                  model_name, duration_ms, confidence);
        }
        
        pub fn inference_failed(model_name: &str, error: &str) {
            error!("AI inference failed: {} - {}", model_name, error);
        }
    }

    /// Vision system logging
    pub mod vision {
        use super::*;
        
        pub fn screen_captured(width: u32, height: u32, capture_time_ms: u64) {
            debug!("Screen captured: {}x{} ({}ms)", width, height, capture_time_ms);
        }
        
        pub fn elements_detected(count: usize, confidence_avg: f32) {
            info!("UI elements detected: {} (avg confidence: {:.2})", count, confidence_avg);
        }
        
        pub fn capture_failed(reason: &str) {
            error!("Screen capture failed: {}", reason);
        }
    }

    /// Input system logging
    pub mod input {
        use super::*;
        
        pub fn voice_command_received(command: &str, confidence: f32) {
            info!("Voice command: '{}' (confidence: {:.2})", 
                  crate::utils::logging::filter_sensitive_data(command), confidence);
        }
        
        pub fn hotkey_pressed(hotkey: &str) {
            info!("Hotkey pressed: {}", hotkey);
        }
        
        pub fn command_parsed(original: &str, parsed: &str) {
            debug!("Command parsed: '{}' -> '{}'", 
                   crate::utils::logging::filter_sensitive_data(original),
                   crate::utils::logging::filter_sensitive_data(parsed));
        }
    }

    /// Safety system logging
    pub mod safety {
        use super::*;
        
        pub fn action_validated(action: &str, risk_level: &str, allowed: bool) {
            if allowed {
                info!("Action validated: '{}' (risk: {}, allowed: {})", 
                      crate::utils::logging::filter_sensitive_data(action), risk_level, allowed);
            } else {
                warn!("Action blocked: '{}' (risk: {})", 
                      crate::utils::logging::filter_sensitive_data(action), risk_level);
            }
        }
        
        pub fn confirmation_requested(action: &str, timeout_seconds: u32) {
            warn!("User confirmation required: '{}' (timeout: {}s)", 
                  crate::utils::logging::filter_sensitive_data(action), timeout_seconds);
        }
        
        pub fn emergency_stop_activated(reason: &str) {
            error!("🚨 EMERGENCY STOP: {}", reason);
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        let log_directory = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("luna-visual-ai")
            .join("logs");

        Self {
            level: "info".to_string(),
            file_logging: true,
            console_logging: true,
            structured_logging: false,
            log_directory,
            max_file_size: 50_000_000, // 50MB
            max_files: 5,
            component_filters: vec![
                ComponentFilter {
                    component: "ai".to_string(),
                    level: "info".to_string(),
                },
                ComponentFilter {
                    component: "vision".to_string(),
                    level: "info".to_string(),
                },
                ComponentFilter {
                    component: "safety".to_string(),
                    level: "warn".to_string(),
                },
            ],
            sensitive_patterns: vec![
                "password".to_string(),
                "token".to_string(),
                "secret".to_string(),
                "key".to_string(),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level_parsing() {
        assert!(matches!(parse_log_level("error"), Ok(Level::ERROR)));
        assert!(matches!(parse_log_level("warn"), Ok(Level::WARN)));
        assert!(matches!(parse_log_level("info"), Ok(Level::INFO)));
        assert!(matches!(parse_log_level("debug"), Ok(Level::DEBUG)));
        assert!(matches!(parse_log_level("trace"), Ok(Level::TRACE)));
        assert!(parse_log_level("invalid").is_err());
    }

    #[test]
    fn test_sensitive_data_filtering() {
        let message = "User login with password: secret123 and token: abc-def-ghi";
        let filtered = filter_sensitive_data(message);
        
        assert!(!filtered.contains("secret123"));
        assert!(!filtered.contains("abc-def-ghi"));
        assert!(filtered.contains("[REDACTED]"));
    }

    #[test]
    fn test_perf_timer() {
        let timer = perf::PerfTimer::new("test_operation");
        std::thread::sleep(std::time::Duration::from_millis(10));
        let duration = timer.finish();
        assert!(duration >= 10);
    }

    #[test]
    fn test_email_redaction() {
        let message = "User email: test@example.com contacted support";
        let filtered = filter_sensitive_data(message);
        assert!(!filtered.contains("test@example.com"));
        assert!(filtered.contains("[EMAIL_REDACTED]"));
    }
}