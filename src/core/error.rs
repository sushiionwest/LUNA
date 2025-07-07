/*!
 * Luna Error Types - Comprehensive error handling
 */

use thiserror::Error;

/// Luna-specific error types
#[derive(Error, Debug)]
pub enum LunaError {
    #[error("AI model not found: {0}")]
    ModelNotFound(String),
    
    #[error("AI inference failed: {0}")]
    InferenceFailed(String),
    
    #[error("Screen capture failed: {0}")]
    ScreenCaptureFailed(String),
    
    #[error("Unsafe command blocked: {0}")]
    UnsafeCommand(String),
    
    #[error("Input system error: {0}")]
    InputSystemError(String),
    
    #[error("Overlay error: {0}")]
    OverlayError(String),
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
    
    #[error("Memory error: {0}")]
    MemoryError(String),
    
    #[error("Timeout error: operation took too long")]
    TimeoutError,
    
    #[error("No elements found matching: {0}")]
    NoElementsFound(String),
    
    #[error("Multiple elements found, ambiguous: {0}")]
    AmbiguousElements(String),
    
    #[error("Command cancelled by user")]
    UserCancelled,
    
    #[error("System not ready: {0}")]
    SystemNotReady(String),
    
    #[error("Windows API error: {0}")]
    WindowsApiError(String),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    
    #[error("Image processing error: {0}")]
    ImageError(#[from] image::ImageError),
    
    #[error("Generic error: {0}")]
    Generic(String),
}

impl LunaError {
    /// Check if error is recoverable
    pub fn is_recoverable(&self) -> bool {
        match self {
            LunaError::TimeoutError => true,
            LunaError::UserCancelled => true,
            LunaError::NoElementsFound(_) => true,
            LunaError::AmbiguousElements(_) => true,
            LunaError::SystemNotReady(_) => true,
            _ => false,
        }
    }
    
    /// Get user-friendly error message
    pub fn user_message(&self) -> String {
        match self {
            LunaError::ModelNotFound(_) => "AI model not available. Please check your installation.".to_string(),
            LunaError::InferenceFailed(_) => "AI processing failed. Please try again.".to_string(),
            LunaError::ScreenCaptureFailed(_) => "Could not capture screen. Please check permissions.".to_string(),
            LunaError::UnsafeCommand(cmd) => format!("Command '{}' is blocked for safety reasons.", cmd),
            LunaError::InputSystemError(_) => "Input system error. Please try again.".to_string(),
            LunaError::OverlayError(_) => "Display overlay error. Please try again.".to_string(),
            LunaError::ConfigError(_) => "Configuration error. Please check your settings.".to_string(),
            LunaError::MemoryError(_) => "Memory error. Please restart Luna.".to_string(),
            LunaError::TimeoutError => "Operation timed out. Please try again.".to_string(),
            LunaError::NoElementsFound(what) => format!("Could not find '{}'on screen. Please make sure it's visible.", what),
            LunaError::AmbiguousElements(what) => format!("Found multiple '{}' elements. Please be more specific.", what),
            LunaError::UserCancelled => "Command cancelled by user.".to_string(),
            LunaError::SystemNotReady(_) => "Luna is still starting up. Please wait a moment.".to_string(),
            LunaError::WindowsApiError(_) => "Windows system error. Please try again.".to_string(),
            LunaError::IoError(_) => "File system error. Please check permissions.".to_string(),
            LunaError::SerializationError(_) => "Data processing error. Please try again.".to_string(),
            LunaError::ImageError(_) => "Image processing error. Please try again.".to_string(),
            LunaError::Generic(msg) => msg.clone(),
        }
    }
    
    /// Get error severity level
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            LunaError::ModelNotFound(_) => ErrorSeverity::Critical,
            LunaError::ConfigError(_) => ErrorSeverity::Critical,
            LunaError::MemoryError(_) => ErrorSeverity::Critical,
            LunaError::UnsafeCommand(_) => ErrorSeverity::High,
            LunaError::WindowsApiError(_) => ErrorSeverity::High,
            LunaError::InferenceFailed(_) => ErrorSeverity::Medium,
            LunaError::ScreenCaptureFailed(_) => ErrorSeverity::Medium,
            LunaError::InputSystemError(_) => ErrorSeverity::Medium,
            LunaError::OverlayError(_) => ErrorSeverity::Low,
            LunaError::TimeoutError => ErrorSeverity::Low,
            LunaError::NoElementsFound(_) => ErrorSeverity::Low,
            LunaError::AmbiguousElements(_) => ErrorSeverity::Low,
            LunaError::UserCancelled => ErrorSeverity::Info,
            LunaError::SystemNotReady(_) => ErrorSeverity::Info,
            LunaError::IoError(_) => ErrorSeverity::Medium,
            LunaError::SerializationError(_) => ErrorSeverity::Medium,
            LunaError::ImageError(_) => ErrorSeverity::Medium,
            LunaError::Generic(_) => ErrorSeverity::Medium,
        }
    }
}

/// Error severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    /// System cannot continue, requires restart
    Critical,
    /// Major functionality impaired
    High,
    /// Some functionality affected
    Medium,
    /// Minor issue, system can continue
    Low,
    /// Informational, not an error
    Info,
}

/// Result type alias for Luna operations
pub type LunaResult<T> = Result<T, LunaError>;

/// Error context for debugging
#[derive(Debug, Clone)]
pub struct ErrorContext {
    pub operation: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub user_command: Option<String>,
    pub system_state: String,
}

impl ErrorContext {
    pub fn new(operation: &str) -> Self {
        Self {
            operation: operation.to_string(),
            timestamp: chrono::Utc::now(),
            user_command: None,
            system_state: "unknown".to_string(),
        }
    }
    
    pub fn with_command(mut self, command: &str) -> Self {
        self.user_command = Some(command.to_string());
        self
    }
    
    pub fn with_state(mut self, state: &str) -> Self {
        self.system_state = state.to_string();
        self
    }
}