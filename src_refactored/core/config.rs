/*!
 * Luna Configuration - Simplified configuration with minimal dependencies
 */

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Luna configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LunaConfig {
    /// Safety system settings
    pub safety: SafetyConfig,
    /// Vision processing settings
    pub vision: VisionConfig,
    /// Input system settings
    pub input: InputConfig,
    /// Logging settings
    pub logging: LoggingConfig,
}

/// Safety system configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyConfig {
    /// Enable safety system
    pub enabled: bool,
    /// Confidence threshold for dangerous patterns
    pub threat_threshold: f32,
    /// Maximum actions per command
    pub max_actions_per_command: usize,
    /// Action delay in milliseconds
    pub action_delay_ms: u64,
    /// Blocked applications
    pub blocked_apps: Vec<String>,
}

/// Vision processing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionConfig {
    /// Confidence threshold for element detection
    pub confidence_threshold: f32,
    /// Maximum elements to detect
    pub max_elements: usize,
    /// Edge detection sensitivity
    pub edge_threshold: f32,
    /// Minimum element size
    pub min_element_size: u32,
    /// Screenshot quality (0-100)
    pub screenshot_quality: u8,
}

/// Input system configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputConfig {
    /// Click delay in milliseconds
    pub click_delay_ms: u64,
    /// Type delay between characters in milliseconds
    pub type_delay_ms: u64,
    /// Scroll amount per action
    pub scroll_amount: i32,
    /// Enable input validation
    pub validate_coordinates: bool,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level (error, warn, info, debug, trace)
    pub level: String,
    /// Enable file logging
    pub log_to_file: bool,
    /// Log directory
    pub log_dir: Option<PathBuf>,
    /// Maximum log file size in MB
    pub max_file_size_mb: u64,
    /// Maximum number of log files to keep
    pub max_files: u32,
}

impl Default for LunaConfig {
    fn default() -> Self {
        Self {
            safety: SafetyConfig::default(),
            vision: VisionConfig::default(),
            input: InputConfig::default(),
            logging: LoggingConfig::default(),
        }
    }
}

impl Default for SafetyConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            threat_threshold: 0.7,
            max_actions_per_command: 10,
            action_delay_ms: 50,
            blocked_apps: vec![
                "cmd.exe".to_string(),
                "powershell.exe".to_string(),
                "regedit.exe".to_string(),
            ],
        }
    }
}

impl Default for VisionConfig {
    fn default() -> Self {
        Self {
            confidence_threshold: 0.6,
            max_elements: 50,
            edge_threshold: 30.0,
            min_element_size: 20,
            screenshot_quality: 85,
        }
    }
}

impl Default for InputConfig {
    fn default() -> Self {
        Self {
            click_delay_ms: 50,
            type_delay_ms: 10,
            scroll_amount: 3,
            validate_coordinates: true,
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            log_to_file: false,
            log_dir: None,
            max_file_size_mb: 10,
            max_files: 5,
        }
    }
}

impl LunaConfig {
    /// Load configuration from file
    pub fn from_file(path: &std::path::Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: LunaConfig = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// Save configuration to file
    pub fn save_to_file(&self, path: &std::path::Path) -> anyhow::Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Get default configuration file path
    pub fn default_config_path() -> anyhow::Result<PathBuf> {
        let mut path = if let Some(config_dir) = dirs::config_dir() {
            config_dir
        } else {
            std::env::current_dir()?
        };
        
        path.push("luna");
        std::fs::create_dir_all(&path)?;
        path.push("config.json");
        
        Ok(path)
    }

    /// Load configuration from default location or create default
    pub fn load_or_default() -> Self {
        if let Ok(config_path) = Self::default_config_path() {
            if config_path.exists() {
                if let Ok(config) = Self::from_file(&config_path) {
                    return config;
                }
            }
        }
        
        // Return default configuration if loading fails
        Self::default()
    }

    /// Save current configuration to default location
    pub fn save_to_default_location(&self) -> anyhow::Result<()> {
        let config_path = Self::default_config_path()?;
        self.save_to_file(&config_path)
    }

    /// Validate configuration values
    pub fn validate(&self) -> anyhow::Result<()> {
        // Validate safety config
        if self.safety.threat_threshold < 0.0 || self.safety.threat_threshold > 1.0 {
            return Err(anyhow::anyhow!("Safety threat threshold must be between 0.0 and 1.0"));
        }

        if self.safety.max_actions_per_command == 0 {
            return Err(anyhow::anyhow!("Max actions per command must be greater than 0"));
        }

        // Validate vision config
        if self.vision.confidence_threshold < 0.0 || self.vision.confidence_threshold > 1.0 {
            return Err(anyhow::anyhow!("Vision confidence threshold must be between 0.0 and 1.0"));
        }

        if self.vision.max_elements == 0 {
            return Err(anyhow::anyhow!("Max elements must be greater than 0"));
        }

        if self.vision.screenshot_quality > 100 {
            return Err(anyhow::anyhow!("Screenshot quality must be between 0 and 100"));
        }

        // Validate logging config
        let valid_levels = ["error", "warn", "info", "debug", "trace"];
        if !valid_levels.contains(&self.logging.level.as_str()) {
            return Err(anyhow::anyhow!("Invalid log level: {}", self.logging.level));
        }

        Ok(())
    }

    /// Apply configuration to logger
    pub fn apply_logging(&self) -> anyhow::Result<()> {
        // Set up env_logger if logging feature is enabled
        #[cfg(feature = "logging")]
        {
            use std::env;
            env::set_var("RUST_LOG", &self.logging.level);
            env_logger::init();
        }
        
        Ok(())
    }
}