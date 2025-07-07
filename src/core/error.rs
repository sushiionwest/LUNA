/*!
 * Luna Visual AI Error Types
 * 
 * Comprehensive error handling system for all Luna components.
 * Provides detailed error context, recovery suggestions, and
 * integration with Rust's Result type system.
 */

use std::fmt;
use thiserror::Error;

/// Main error type for Luna Visual AI operations
/// 
/// This enum covers all possible error conditions that can occur
/// within Luna, providing detailed context and recovery suggestions.
#[derive(Error, Debug)]
pub enum LunaError {
    /// AI model loading or inference errors
    #[error("AI model error: {message}")]
    AiModel {
        message: String,
        model_name: String,
        recovery_suggestion: String,
    },

    /// Screen capture and vision system errors
    #[error("Vision system error: {message}")]
    Vision {
        message: String,
        component: String,
        is_recoverable: bool,
    },

    /// Voice input and command processing errors
    #[error("Input processing error: {message}")]
    Input {
        message: String,
        input_type: InputType,
    },

    /// Visual overlay and UI errors
    #[error("Overlay system error: {message}")]
    Overlay {
        message: String,
        context: String,
    },

    /// Memory management and resource errors
    #[error("Memory error: {message}")]
    Memory {
        message: String,
        current_usage: u64,
        limit: u64,
    },

    /// Configuration and setup errors
    #[error("Configuration error: {message}")]
    Config {
        message: String,
        config_key: Option<String>,
    },

    /// Windows API integration errors
    #[error("Windows API error: {message} (Code: {error_code})")]
    WindowsApi {
        message: String,
        error_code: u32,
        api_name: String,
    },

    /// File system and I/O errors
    #[error("I/O error: {message}")]
    Io {
        message: String,
        path: Option<String>,
    },

    /// Network and external service errors
    #[error("Network error: {message}")]
    Network {
        message: String,
        endpoint: Option<String>,
    },

    /// Safety validation errors
    #[error("Safety check failed: {message}")]
    Safety {
        message: String,
        risk_level: RiskLevel,
        user_override_allowed: bool,
    },

    /// Generic internal errors
    #[error("Internal error: {message}")]
    Internal {
        message: String,
        component: String,
    },
}

/// Input type categorization for error reporting
#[derive(Debug, Clone, Copy)]
pub enum InputType {
    Voice,
    Text,
    Hotkey,
    Mouse,
    Keyboard,
}

/// Risk level for safety errors
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl fmt::Display for InputType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InputType::Voice => write!(f, "voice"),
            InputType::Text => write!(f, "text"),
            InputType::Hotkey => write!(f, "hotkey"),
            InputType::Mouse => write!(f, "mouse"),
            InputType::Keyboard => write!(f, "keyboard"),
        }
    }
}

impl fmt::Display for RiskLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RiskLevel::Low => write!(f, "low"),
            RiskLevel::Medium => write!(f, "medium"),
            RiskLevel::High => write!(f, "high"),
            RiskLevel::Critical => write!(f, "critical"),
        }
    }
}

impl LunaError {
    /// Create a new AI model error
    pub fn ai_model<S: Into<String>>(
        message: S,
        model_name: S,
        recovery_suggestion: S,
    ) -> Self {
        Self::AiModel {
            message: message.into(),
            model_name: model_name.into(),
            recovery_suggestion: recovery_suggestion.into(),
        }
    }

    /// Create a new vision system error
    pub fn vision<S: Into<String>>(
        message: S,
        component: S,
        is_recoverable: bool,
    ) -> Self {
        Self::Vision {
            message: message.into(),
            component: component.into(),
            is_recoverable,
        }
    }

    /// Create a new input processing error
    pub fn input<S: Into<String>>(message: S, input_type: InputType) -> Self {
        Self::Input {
            message: message.into(),
            input_type,
        }
    }

    /// Create a new overlay system error
    pub fn overlay<S: Into<String>>(message: S, context: S) -> Self {
        Self::Overlay {
            message: message.into(),
            context: context.into(),
        }
    }

    /// Create a new memory error
    pub fn memory<S: Into<String>>(
        message: S,
        current_usage: u64,
        limit: u64,
    ) -> Self {
        Self::Memory {
            message: message.into(),
            current_usage,
            limit,
        }
    }

    /// Create a new configuration error
    pub fn config<S: Into<String>>(
        message: S,
        config_key: Option<S>,
    ) -> Self {
        Self::Config {
            message: message.into(),
            config_key: config_key.map(|k| k.into()),
        }
    }

    /// Create a new Windows API error
    pub fn windows_api<S: Into<String>>(
        message: S,
        error_code: u32,
        api_name: S,
    ) -> Self {
        Self::WindowsApi {
            message: message.into(),
            error_code,
            api_name: api_name.into(),
        }
    }

    /// Create a new I/O error
    pub fn io<S: Into<String>>(message: S, path: Option<S>) -> Self {
        Self::Io {
            message: message.into(),
            path: path.map(|p| p.into()),
        }
    }

    /// Create a new safety error
    pub fn safety<S: Into<String>>(
        message: S,
        risk_level: RiskLevel,
        user_override_allowed: bool,
    ) -> Self {
        Self::Safety {
            message: message.into(),
            risk_level,
            user_override_allowed,
        }
    }

    /// Create a new internal error
    pub fn internal<S: Into<String>>(message: S, component: S) -> Self {
        Self::Internal {
            message: message.into(),
            component: component.into(),
        }
    }

    /// Check if this error is recoverable
    pub fn is_recoverable(&self) -> bool {
        match self {
            LunaError::Vision { is_recoverable, .. } => *is_recoverable,
            LunaError::Memory { .. } => false, // Memory errors typically require restart
            LunaError::Safety { risk_level, .. } => *risk_level < RiskLevel::Critical,
            LunaError::WindowsApi { .. } => false, // API errors usually require restart
            LunaError::AiModel { .. } => true, // AI models can often be reloaded
            LunaError::Input { .. } => true,
            LunaError::Overlay { .. } => true,
            LunaError::Config { .. } => true,
            LunaError::Io { .. } => true,
            LunaError::Network { .. } => true,
            LunaError::Internal { .. } => false,
        }
    }

    /// Get suggested recovery action
    pub fn recovery_suggestion(&self) -> String {
        match self {
            LunaError::AiModel { recovery_suggestion, .. } => {
                recovery_suggestion.clone()
            }
            LunaError::Vision { component, is_recoverable, .. } => {
                if *is_recoverable {
                    format!("Try restarting the {} component", component)
                } else {
                    "Restart Luna application".to_string()
                }
            }
            LunaError::Input { input_type, .. } => {
                match input_type {
                    InputType::Voice => "Check microphone connection and permissions".to_string(),
                    InputType::Hotkey => "Check for conflicting hotkey assignments".to_string(),
                    _ => "Retry the operation".to_string(),
                }
            }
            LunaError::Memory { .. } => {
                "Close other applications to free memory, then restart Luna".to_string()
            }
            LunaError::Safety { user_override_allowed, .. } => {
                if *user_override_allowed {
                    "Review the action and manually confirm if safe".to_string()
                } else {
                    "This action cannot be performed for safety reasons".to_string()
                }
            }
            _ => "Try restarting Luna or contact support if the problem persists".to_string(),
        }
    }

    /// Get the component that caused this error
    pub fn component(&self) -> &str {
        match self {
            LunaError::AiModel { model_name, .. } => model_name,
            LunaError::Vision { component, .. } => component,
            LunaError::Input { .. } => "input",
            LunaError::Overlay { .. } => "overlay",
            LunaError::Memory { .. } => "memory",
            LunaError::Config { .. } => "config",
            LunaError::WindowsApi { api_name, .. } => api_name,
            LunaError::Io { .. } => "io",
            LunaError::Network { .. } => "network",
            LunaError::Safety { .. } => "safety",
            LunaError::Internal { component, .. } => component,
        }
    }
}

/// Convert from standard I/O errors
impl From<std::io::Error> for LunaError {
    fn from(err: std::io::Error) -> Self {
        LunaError::io(err.to_string(), None)
    }
}

/// Convert from Windows API errors
impl From<windows::core::Error> for LunaError {
    fn from(err: windows::core::Error) -> Self {
        LunaError::windows_api(
            err.to_string(),
            err.code().0 as u32,
            "Windows API",
        )
    }
}

/// Convert from serde JSON errors
impl From<serde_json::Error> for LunaError {
    fn from(err: serde_json::Error) -> Self {
        LunaError::config(
            format!("JSON parsing error: {}", err),
            None,
        )
    }
}

/// Result type alias for Luna operations
pub type Result<T> = std::result::Result<T, LunaError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let error = LunaError::ai_model(
            "Model failed to load",
            "Florence-2",
            "Download the model again",
        );
        
        assert!(error.to_string().contains("Model failed to load"));
        assert!(error.is_recoverable());
    }

    #[test]
    fn test_risk_level_ordering() {
        assert!(RiskLevel::Low < RiskLevel::High);
        assert!(RiskLevel::Critical > RiskLevel::Medium);
    }

    #[test]
    fn test_recovery_suggestions() {
        let memory_error = LunaError::memory("Out of memory", 8_000_000_000, 4_000_000_000);
        assert!(memory_error.recovery_suggestion().contains("free memory"));
        assert!(!memory_error.is_recoverable());
    }
}