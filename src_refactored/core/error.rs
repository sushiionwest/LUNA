/*!
 * Luna Error Types - Simplified error handling without heavy dependencies
 */

use std::fmt;

/// Luna-specific error types
#[derive(Debug)]
pub enum LunaError {
    /// Configuration error
    Config(String),
    /// Safety system blocked operation
    UnsafeCommand(String),
    /// Safety system blocked action
    UnsafeAction(String),
    /// Vision processing error
    Vision(String),
    /// Input system error
    Input(String),
    /// Screen capture error
    ScreenCapture(String),
    /// AI processing error
    AI(String),
    /// System error
    System(String),
    /// Invalid argument
    InvalidArgument(String),
    /// Operation timeout
    Timeout(String),
    /// Resource not found
    NotFound(String),
    /// Permission denied
    PermissionDenied(String),
}

impl fmt::Display for LunaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LunaError::Config(msg) => write!(f, "Configuration error: {}", msg),
            LunaError::UnsafeCommand(cmd) => write!(f, "Unsafe command blocked: {}", cmd),
            LunaError::UnsafeAction(action) => write!(f, "Unsafe action blocked: {}", action),
            LunaError::Vision(msg) => write!(f, "Vision processing error: {}", msg),
            LunaError::Input(msg) => write!(f, "Input system error: {}", msg),
            LunaError::ScreenCapture(msg) => write!(f, "Screen capture error: {}", msg),
            LunaError::AI(msg) => write!(f, "AI processing error: {}", msg),
            LunaError::System(msg) => write!(f, "System error: {}", msg),
            LunaError::InvalidArgument(msg) => write!(f, "Invalid argument: {}", msg),
            LunaError::Timeout(msg) => write!(f, "Operation timeout: {}", msg),
            LunaError::NotFound(msg) => write!(f, "Resource not found: {}", msg),
            LunaError::PermissionDenied(msg) => write!(f, "Permission denied: {}", msg),
        }
    }
}

impl std::error::Error for LunaError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        // Most Luna errors don't wrap other errors
        None
    }
}

/// Result type alias for Luna operations
pub type LunaResult<T> = Result<T, LunaError>;

/// Convert from common error types
impl From<std::io::Error> for LunaError {
    fn from(error: std::io::Error) -> Self {
        match error.kind() {
            std::io::ErrorKind::NotFound => LunaError::NotFound(error.to_string()),
            std::io::ErrorKind::PermissionDenied => LunaError::PermissionDenied(error.to_string()),
            std::io::ErrorKind::TimedOut => LunaError::Timeout(error.to_string()),
            _ => LunaError::System(error.to_string()),
        }
    }
}

impl From<serde_json::Error> for LunaError {
    fn from(error: serde_json::Error) -> Self {
        LunaError::Config(format!("JSON error: {}", error))
    }
}

impl From<image::ImageError> for LunaError {
    fn from(error: image::ImageError) -> Self {
        LunaError::Vision(format!("Image processing error: {}", error))
    }
}

/// Error context for better error reporting
pub struct ErrorContext {
    pub operation: String,
    pub details: Option<String>,
    pub timestamp: std::time::SystemTime,
}

impl ErrorContext {
    pub fn new(operation: &str) -> Self {
        Self {
            operation: operation.to_string(),
            details: None,
            timestamp: std::time::SystemTime::now(),
        }
    }

    pub fn with_details(mut self, details: &str) -> Self {
        self.details = Some(details.to_string());
        self
    }
}

/// Helper trait for adding context to errors
pub trait ErrorExt<T> {
    fn with_context(self, context: ErrorContext) -> Result<T, LunaError>;
    fn with_operation(self, operation: &str) -> Result<T, LunaError>;
}

impl<T, E> ErrorExt<T> for Result<T, E> 
where 
    E: Into<LunaError>,
{
    fn with_context(self, context: ErrorContext) -> Result<T, LunaError> {
        self.map_err(|e| {
            let base_error = e.into();
            let error_msg = if let Some(details) = context.details {
                format!("{}: {} ({})", context.operation, base_error, details)
            } else {
                format!("{}: {}", context.operation, base_error)
            };
            
            // Preserve error type when possible
            match base_error {
                LunaError::Config(_) => LunaError::Config(error_msg),
                LunaError::Vision(_) => LunaError::Vision(error_msg),
                LunaError::Input(_) => LunaError::Input(error_msg),
                LunaError::ScreenCapture(_) => LunaError::ScreenCapture(error_msg),
                LunaError::AI(_) => LunaError::AI(error_msg),
                _ => LunaError::System(error_msg),
            }
        })
    }

    fn with_operation(self, operation: &str) -> Result<T, LunaError> {
        self.with_context(ErrorContext::new(operation))
    }
}

/// Macro for creating errors with context
#[macro_export]
macro_rules! luna_error {
    ($kind:ident, $msg:expr) => {
        LunaError::$kind($msg.to_string())
    };
    ($kind:ident, $fmt:expr, $($args:tt)*) => {
        LunaError::$kind(format!($fmt, $($args)*))
    };
}

/// Macro for early return with error context
#[macro_export]
macro_rules! bail {
    ($kind:ident, $msg:expr) => {
        return Err(luna_error!($kind, $msg))
    };
    ($kind:ident, $fmt:expr, $($args:tt)*) => {
        return Err(luna_error!($kind, $fmt, $($args)*))
    };
}

/// Ensure condition or return error
#[macro_export]
macro_rules! ensure {
    ($cond:expr, $kind:ident, $msg:expr) => {
        if !$cond {
            bail!($kind, $msg);
        }
    };
    ($cond:expr, $kind:ident, $fmt:expr, $($args:tt)*) => {
        if !$cond {
            bail!($kind, $fmt, $($args)*);
        }
    };
}

// Re-export macros at crate level
pub use {bail, ensure, luna_error};