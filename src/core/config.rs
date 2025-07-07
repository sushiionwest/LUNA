/*!
 * Luna Visual AI Configuration System
 * 
 * Comprehensive configuration management with:
 * - Default values for all settings
 * - File-based configuration loading
 * - Environment variable overrides
 * - Real-time configuration updates
 * - Validation and error handling
 */

use crate::core::error::{LunaError, Result};
use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{info, warn};

/// Main configuration structure for Luna Visual AI
/// 
/// Contains all settings needed to configure Luna's behavior,
/// performance characteristics, and feature enabling/disabling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// AI model configuration
    pub ai: AiConfig,
    
    /// Vision system configuration
    pub vision: VisionConfig,
    
    /// Input processing configuration
    pub input: InputConfig,
    
    /// Visual overlay configuration
    pub overlay: OverlayConfig,
    
    /// Memory management configuration
    pub memory: MemoryConfig,
    
    /// Safety and security configuration
    pub safety: SafetyConfig,
    
    /// Logging configuration
    pub logging: LoggingConfig,
    
    /// Performance tuning configuration
    pub performance: PerformanceConfig,
}

/// AI model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    /// Directory to store AI models
    pub model_cache_dir: PathBuf,
    
    /// Maximum memory usage for AI models (in bytes)
    pub max_model_memory: u64,
    
    /// GPU device ID to use (-1 for CPU)
    pub gpu_device_id: i32,
    
    /// Florence-2 model configuration
    pub florence: ModelConfig,
    
    /// CLIP model configuration
    pub clip: ModelConfig,
    
    /// TrOCR model configuration
    pub trocr: ModelConfig,
    
    /// SAM model configuration
    pub sam: ModelConfig,
    
    /// Parallel processing configuration
    pub parallel_processing: bool,
    
    /// Model inference timeout (in milliseconds)
    pub inference_timeout_ms: u64,
}

/// Individual AI model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    /// Model name/identifier
    pub name: String,
    
    /// Whether this model is enabled
    pub enabled: bool,
    
    /// Model file path (relative to model_cache_dir)
    pub model_path: String,
    
    /// Maximum memory usage for this model
    pub max_memory: u64,
    
    /// Model-specific parameters
    pub parameters: serde_json::Value,
}

/// Vision system configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionConfig {
    /// Screen capture method ("gdi", "directx", "auto")
    pub capture_method: String,
    
    /// Screen capture frame rate (FPS)
    pub capture_fps: u32,
    
    /// Screenshot compression quality (0-100)
    pub compression_quality: u8,
    
    /// Maximum screenshot resolution
    pub max_resolution: (u32, u32),
    
    /// Multi-monitor support
    pub multi_monitor: bool,
    
    /// Element detection confidence threshold
    pub detection_threshold: f32,
    
    /// Image preprocessing options
    pub preprocessing: PreprocessingConfig,
}

/// Image preprocessing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreprocessingConfig {
    /// Enable noise reduction
    pub noise_reduction: bool,
    
    /// Enable contrast enhancement
    pub contrast_enhancement: bool,
    
    /// Enable sharpening
    pub sharpening: bool,
    
    /// Resize factor for AI processing
    pub resize_factor: f32,
}

/// Input processing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputConfig {
    /// Voice command configuration
    pub voice: VoiceConfig,
    
    /// Text command configuration
    pub text: TextConfig,
    
    /// Hotkey configuration
    pub hotkeys: Vec<HotkeyConfig>,
    
    /// Input validation settings
    pub validation: ValidationConfig,
}

/// Voice command configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceConfig {
    /// Enable voice commands
    pub enabled: bool,
    
    /// Wake word for activation
    pub wake_word: String,
    
    /// Audio device index (-1 for default)
    pub audio_device: i32,
    
    /// Whisper model size
    pub whisper_model: String,
    
    /// Voice activation threshold
    pub activation_threshold: f32,
    
    /// Noise gate threshold
    pub noise_gate: f32,
    
    /// Voice command timeout (in seconds)
    pub command_timeout: u32,
}

/// Text command configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextConfig {
    /// Enable text commands
    pub enabled: bool,
    
    /// Command prefix character
    pub command_prefix: String,
    
    /// Maximum command length
    pub max_length: usize,
    
    /// Enable natural language processing
    pub nlp_enabled: bool,
}

/// Hotkey configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotkeyConfig {
    /// Hotkey name/identifier
    pub name: String,
    
    /// Key combination (e.g., "Ctrl+Shift+L")
    pub keys: String,
    
    /// Action to perform
    pub action: String,
    
    /// Whether hotkey is enabled
    pub enabled: bool,
}

/// Input validation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConfig {
    /// Enable command validation
    pub enabled: bool,
    
    /// Blacklisted commands (regex patterns)
    pub blacklist: Vec<String>,
    
    /// Require confirmation for dangerous actions
    pub confirm_dangerous: bool,
    
    /// Maximum actions per minute
    pub rate_limit: u32,
}

/// Visual overlay configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverlayConfig {
    /// Enable visual overlay
    pub enabled: bool,
    
    /// Overlay transparency (0.0 - 1.0)
    pub transparency: f32,
    
    /// Countdown duration (in seconds)
    pub countdown_duration: u32,
    
    /// Show AI decision process
    pub show_debug_info: bool,
    
    /// Highlight color (RGBA)
    pub highlight_color: [f32; 4],
    
    /// UI theme configuration
    pub theme: ThemeConfig,
    
    /// Animation settings
    pub animations: AnimationConfig,
}

/// UI theme configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfig {
    /// Theme name
    pub name: String,
    
    /// Primary color
    pub primary_color: [f32; 4],
    
    /// Secondary color
    pub secondary_color: [f32; 4],
    
    /// Background color
    pub background_color: [f32; 4],
    
    /// Text color
    pub text_color: [f32; 4],
    
    /// Font size
    pub font_size: f32,
}

/// Animation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimationConfig {
    /// Enable animations
    pub enabled: bool,
    
    /// Animation duration (in milliseconds)
    pub duration: u64,
    
    /// Animation easing type
    pub easing: String,
    
    /// Reduce motion for accessibility
    pub reduce_motion: bool,
}

/// Memory management configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    /// Maximum total memory usage (in bytes)
    pub max_total_memory: u64,
    
    /// Memory cleanup threshold (0.0 - 1.0)
    pub cleanup_threshold: f32,
    
    /// Enable memory profiling
    pub profiling_enabled: bool,
    
    /// Memory check interval (in seconds)
    pub check_interval: u32,
    
    /// Emergency cleanup settings
    pub emergency_cleanup: EmergencyCleanupConfig,
}

/// Emergency cleanup configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmergencyCleanupConfig {
    /// Enable emergency cleanup
    pub enabled: bool,
    
    /// Memory threshold for emergency cleanup
    pub threshold: f32,
    
    /// Actions to take during emergency cleanup
    pub actions: Vec<String>,
}

/// Safety and security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyConfig {
    /// Enable safety checks
    pub enabled: bool,
    
    /// Dangerous action patterns
    pub dangerous_patterns: Vec<String>,
    
    /// Require confirmation timeout (in seconds)
    pub confirmation_timeout: u32,
    
    /// Enable sandbox mode
    pub sandbox_mode: bool,
    
    /// Allowed applications for interaction
    pub allowed_apps: Vec<String>,
    
    /// Blocked applications
    pub blocked_apps: Vec<String>,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level (error, warn, info, debug, trace)
    pub level: String,
    
    /// Enable file logging
    pub file_logging: bool,
    
    /// Log file path
    pub log_file: PathBuf,
    
    /// Maximum log file size (in bytes)
    pub max_file_size: u64,
    
    /// Number of log files to keep
    pub max_files: u32,
    
    /// Enable structured logging (JSON)
    pub structured: bool,
}

/// Performance tuning configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Number of worker threads
    pub worker_threads: usize,
    
    /// Enable CPU affinity
    pub cpu_affinity: bool,
    
    /// CPU affinity mask
    pub affinity_mask: u64,
    
    /// Process priority (-20 to 19 on Unix, 0-4 on Windows)
    pub process_priority: i32,
    
    /// Enable performance monitoring
    pub monitoring: bool,
    
    /// Performance metrics interval (in seconds)
    pub metrics_interval: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            ai: AiConfig::default(),
            vision: VisionConfig::default(),
            input: InputConfig::default(),
            overlay: OverlayConfig::default(),
            memory: MemoryConfig::default(),
            safety: SafetyConfig::default(),
            logging: LoggingConfig::default(),
            performance: PerformanceConfig::default(),
        }
    }
}

impl Default for AiConfig {
    fn default() -> Self {
        let model_cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("luna-visual-ai")
            .join("models");

        Self {
            model_cache_dir,
            max_model_memory: 4_000_000_000, // 4GB
            gpu_device_id: 0,
            florence: ModelConfig {
                name: "Florence-2".to_string(),
                enabled: true,
                model_path: "florence-2-base".to_string(),
                max_memory: 1_000_000_000, // 1GB
                parameters: serde_json::json!({
                    "temperature": 0.1,
                    "max_tokens": 512
                }),
            },
            clip: ModelConfig {
                name: "CLIP".to_string(),
                enabled: true,
                model_path: "clip-vit-base-patch32".to_string(),
                max_memory: 500_000_000, // 500MB
                parameters: serde_json::json!({
                    "image_size": 224
                }),
            },
            trocr: ModelConfig {
                name: "TrOCR".to_string(),
                enabled: true,
                model_path: "trocr-base-printed".to_string(),
                max_memory: 800_000_000, // 800MB
                parameters: serde_json::json!({
                    "max_length": 256
                }),
            },
            sam: ModelConfig {
                name: "SAM".to_string(),
                enabled: true,
                model_path: "sam-vit-base".to_string(),
                max_memory: 1_200_000_000, // 1.2GB
                parameters: serde_json::json!({
                    "points_per_side": 32
                }),
            },
            parallel_processing: true,
            inference_timeout_ms: 5000,
        }
    }
}

impl Default for VisionConfig {
    fn default() -> Self {
        Self {
            capture_method: "auto".to_string(),
            capture_fps: 30,
            compression_quality: 85,
            max_resolution: (1920, 1080),
            multi_monitor: true,
            detection_threshold: 0.5,
            preprocessing: PreprocessingConfig::default(),
        }
    }
}

impl Default for PreprocessingConfig {
    fn default() -> Self {
        Self {
            noise_reduction: true,
            contrast_enhancement: true,
            sharpening: false,
            resize_factor: 1.0,
        }
    }
}

impl Default for InputConfig {
    fn default() -> Self {
        Self {
            voice: VoiceConfig::default(),
            text: TextConfig::default(),
            hotkeys: vec![
                HotkeyConfig {
                    name: "activate_luna".to_string(),
                    keys: "Ctrl+Shift+L".to_string(),
                    action: "activate".to_string(),
                    enabled: true,
                },
                HotkeyConfig {
                    name: "emergency_stop".to_string(),
                    keys: "Ctrl+Shift+Esc".to_string(),
                    action: "emergency_stop".to_string(),
                    enabled: true,
                },
            ],
            validation: ValidationConfig::default(),
        }
    }
}

impl Default for VoiceConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            wake_word: "Luna".to_string(),
            audio_device: -1,
            whisper_model: "base".to_string(),
            activation_threshold: 0.5,
            noise_gate: 0.1,
            command_timeout: 10,
        }
    }
}

impl Default for TextConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            command_prefix: "/".to_string(),
            max_length: 500,
            nlp_enabled: true,
        }
    }
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            blacklist: vec![
                r"delete.*system32".to_string(),
                r"format.*drive".to_string(),
                r"rm\s+-rf\s+/".to_string(),
            ],
            confirm_dangerous: true,
            rate_limit: 60, // 60 actions per minute max
        }
    }
}

impl Default for OverlayConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            transparency: 0.9,
            countdown_duration: 3,
            show_debug_info: false,
            highlight_color: [1.0, 0.5, 0.0, 0.8], // Orange
            theme: ThemeConfig::default(),
            animations: AnimationConfig::default(),
        }
    }
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            name: "Luna Dark".to_string(),
            primary_color: [0.4, 0.7, 1.0, 1.0], // Luna blue
            secondary_color: [1.0, 0.5, 0.0, 1.0], // Orange
            background_color: [0.0, 0.0, 0.0, 0.8], // Semi-transparent black
            text_color: [1.0, 1.0, 1.0, 1.0], // White
            font_size: 14.0,
        }
    }
}

impl Default for AnimationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            duration: 300,
            easing: "ease-out".to_string(),
            reduce_motion: false,
        }
    }
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            max_total_memory: 8_000_000_000, // 8GB
            cleanup_threshold: 0.8,
            profiling_enabled: false,
            check_interval: 30,
            emergency_cleanup: EmergencyCleanupConfig::default(),
        }
    }
}

impl Default for EmergencyCleanupConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            threshold: 0.95,
            actions: vec![
                "clear_model_cache".to_string(),
                "clear_image_buffers".to_string(),
                "force_gc".to_string(),
            ],
        }
    }
}

impl Default for SafetyConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            dangerous_patterns: vec![
                r"delete.*".to_string(),
                r"format.*".to_string(),
                r"shutdown.*".to_string(),
                r"restart.*".to_string(),
            ],
            confirmation_timeout: 10,
            sandbox_mode: false,
            allowed_apps: vec![],
            blocked_apps: vec![
                "cmd.exe".to_string(),
                "powershell.exe".to_string(),
                "regedit.exe".to_string(),
            ],
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        let log_file = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("luna-visual-ai")
            .join("logs")
            .join("luna.log");

        Self {
            level: "info".to_string(),
            file_logging: true,
            log_file,
            max_file_size: 50_000_000, // 50MB
            max_files: 5,
            structured: false,
        }
    }
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            worker_threads: num_cpus::get(),
            cpu_affinity: false,
            affinity_mask: 0,
            process_priority: 0,
            monitoring: true,
            metrics_interval: 60,
        }
    }
}

impl Config {
    /// Load configuration from a file
    /// 
    /// Supports TOML, JSON, and YAML formats based on file extension.
    /// Falls back to default configuration if file doesn't exist.
    pub async fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        
        if !path.exists() {
            warn!("Configuration file not found: {:?}, using defaults", path);
            return Ok(Self::default());
        }

        let content = fs::read_to_string(path).await
            .context("Failed to read configuration file")
            .map_err(|e| LunaError::config(e.to_string(), Some(path.to_string_lossy())))?;

        let config = match path.extension().and_then(|ext| ext.to_str()) {
            Some("toml") => {
                toml::from_str(&content)
                    .context("Failed to parse TOML configuration")
                    .map_err(|e| LunaError::config(e.to_string(), Some("TOML parsing")))?
            }
            Some("json") => {
                serde_json::from_str(&content)
                    .context("Failed to parse JSON configuration")
                    .map_err(|e| LunaError::config(e.to_string(), Some("JSON parsing")))?
            }
            Some("yaml") | Some("yml") => {
                serde_yaml::from_str(&content)
                    .context("Failed to parse YAML configuration")
                    .map_err(|e| LunaError::config(e.to_string(), Some("YAML parsing")))?
            }
            _ => {
                return Err(LunaError::config(
                    "Unsupported configuration file format",
                    Some("file extension"),
                ));
            }
        };

        info!("Loaded configuration from: {:?}", path);
        Ok(config)
    }

    /// Save configuration to a file
    pub async fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let path = path.as_ref();
        
        // Create directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await
                .context("Failed to create configuration directory")
                .map_err(|e| LunaError::io(e.to_string(), Some(parent.to_string_lossy())))?;
        }

        let content = match path.extension().and_then(|ext| ext.to_str()) {
            Some("toml") => {
                toml::to_string_pretty(self)
                    .context("Failed to serialize configuration to TOML")
                    .map_err(|e| LunaError::config(e.to_string(), Some("TOML serialization")))?
            }
            Some("json") => {
                serde_json::to_string_pretty(self)
                    .context("Failed to serialize configuration to JSON")
                    .map_err(|e| LunaError::config(e.to_string(), Some("JSON serialization")))?
            }
            Some("yaml") | Some("yml") => {
                serde_yaml::to_string(self)
                    .context("Failed to serialize configuration to YAML")
                    .map_err(|e| LunaError::config(e.to_string(), Some("YAML serialization")))?
            }
            _ => {
                return Err(LunaError::config(
                    "Unsupported configuration file format for saving",
                    Some("file extension"),
                ));
            }
        };

        fs::write(path, content).await
            .context("Failed to write configuration file")
            .map_err(|e| LunaError::io(e.to_string(), Some(path.to_string_lossy())))?;

        info!("Saved configuration to: {:?}", path);
        Ok(())
    }

    /// Validate configuration values
    pub fn validate(&self) -> Result<()> {
        // Validate memory limits
        if self.memory.max_total_memory < 1_000_000_000 {
            return Err(LunaError::config(
                "Maximum total memory must be at least 1GB",
                Some("memory.max_total_memory"),
            ));
        }

        // Validate AI model memory doesn't exceed total
        if self.ai.max_model_memory > self.memory.max_total_memory {
            return Err(LunaError::config(
                "AI model memory limit exceeds total memory limit",
                Some("ai.max_model_memory"),
            ));
        }

        // Validate countdown duration
        if self.overlay.countdown_duration > 60 {
            return Err(LunaError::config(
                "Countdown duration cannot exceed 60 seconds",
                Some("overlay.countdown_duration"),
            ));
        }

        // Validate transparency
        if self.overlay.transparency < 0.0 || self.overlay.transparency > 1.0 {
            return Err(LunaError::config(
                "Transparency must be between 0.0 and 1.0",
                Some("overlay.transparency"),
            ));
        }

        info!("Configuration validation passed");
        Ok(())
    }

    /// Get the default configuration file path
    pub fn default_config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("luna-visual-ai")
            .join("config.toml")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert!(config.validate().is_ok());
    }

    #[tokio::test]
    async fn test_config_save_load() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test_config.toml");
        
        let config = Config::default();
        config.save_to_file(&config_path).await.unwrap();
        
        let loaded_config = Config::from_file(&config_path).await.unwrap();
        assert_eq!(config.ai.max_model_memory, loaded_config.ai.max_model_memory);
    }

    #[test]
    fn test_config_validation() {
        let mut config = Config::default();
        
        // Test invalid memory configuration
        config.memory.max_total_memory = 500_000_000; // 500MB - too low
        assert!(config.validate().is_err());
        
        // Test invalid transparency
        config = Config::default();
        config.overlay.transparency = 1.5; // Invalid transparency
        assert!(config.validate().is_err());
    }
}