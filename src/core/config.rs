/*!
 * Luna Configuration - Simple embedded configuration
 */

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Luna configuration - embedded defaults
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LunaConfig {
    /// AI model settings
    pub ai: AiConfig,
    /// Safety settings
    pub safety: SafetyConfig,
    /// UI settings
    pub ui: UiConfig,
    /// Performance settings
    pub performance: PerformanceConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    /// Model precision (f16, f32)
    pub precision: String,
    /// Use GPU acceleration if available
    pub use_gpu: bool,
    /// Maximum inference time in milliseconds
    pub max_inference_time_ms: u64,
    /// Confidence threshold for actions
    pub confidence_threshold: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyConfig {
    /// Enable safety checks
    pub enabled: bool,
    /// Confirmation timeout in seconds
    pub confirmation_timeout_seconds: u64,
    /// Blocked keywords that prevent execution
    pub blocked_keywords: Vec<String>,
    /// Maximum actions per command
    pub max_actions_per_command: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    /// Show visual overlay
    pub show_overlay: bool,
    /// Overlay transparency (0.0 to 1.0)
    pub overlay_opacity: f32,
    /// Highlight color (RGB)
    pub highlight_color: [u8; 3],
    /// Enable sound effects
    pub enable_sounds: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Maximum memory usage in MB
    pub max_memory_mb: u64,
    /// Number of worker threads
    pub worker_threads: usize,
    /// Screenshot cache size
    pub screenshot_cache_size: usize,
}

impl Default for LunaConfig {
    fn default() -> Self {
        Self {
            ai: AiConfig {
                precision: "f32".to_string(),
                use_gpu: false, // CPU-only for maximum compatibility
                max_inference_time_ms: 5000,
                confidence_threshold: 0.7,
            },
            safety: SafetyConfig {
                enabled: true,
                confirmation_timeout_seconds: 3,
                blocked_keywords: vec![
                    "delete".to_string(),
                    "format".to_string(),
                    "shutdown".to_string(),
                    "restart".to_string(),
                    "registry".to_string(),
                    "system32".to_string(),
                ],
                max_actions_per_command: 10,
            },
            ui: UiConfig {
                show_overlay: true,
                overlay_opacity: 0.8,
                highlight_color: [100, 149, 237], // Cornflower blue
                enable_sounds: true,
            },
            performance: PerformanceConfig {
                max_memory_mb: 512,
                worker_threads: 2,
                screenshot_cache_size: 5,
            },
        }
    }
}

impl LunaConfig {
    /// Load configuration (embedded defaults for portable executable)
    pub fn load() -> Result<Self> {
        // For portable executable, we use embedded defaults
        // No external config files needed
        Ok(Self::default())
    }
    
    /// Get data directory (portable mode uses current directory)
    pub fn data_dir() -> PathBuf {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    }
    
    /// Get temporary directory for Luna
    pub fn temp_dir() -> PathBuf {
        std::env::temp_dir().join("luna")
    }
    
    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        if self.performance.max_memory_mb < 128 {
            return Err(anyhow::anyhow!("Minimum memory requirement is 128MB"));
        }
        
        if self.performance.worker_threads == 0 {
            return Err(anyhow::anyhow!("Must have at least 1 worker thread"));
        }
        
        if self.ai.confidence_threshold < 0.1 || self.ai.confidence_threshold > 1.0 {
            return Err(anyhow::anyhow!("Confidence threshold must be between 0.1 and 1.0"));
        }
        
        Ok(())
    }
}