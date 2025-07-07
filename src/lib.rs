/*!
 * Luna Visual AI Library
 * 
 * Core library providing AI-powered computer vision capabilities for desktop automation.
 * Built with Rust for maximum performance, memory safety, and reliability.
 * 
 * ## Architecture Overview
 * 
 * Luna uses a 6-step processing pipeline:
 * 1. **Input Processing** - Voice/text command interpretation
 * 2. **Screen Capture** - High-speed Win32 screen capture
 * 3. **AI Analysis** - 4-specialist AI model pipeline
 * 4. **Decision Engine** - Command-to-action mapping
 * 5. **Visual Preview** - 3-second countdown with cancellation
 * 6. **Action Execution** - Precise mouse/keyboard automation
 * 
 * ## Key Components
 * 
 * - **AI Pipeline**: Florence-2, CLIP, TrOCR, SAM working in parallel
 * - **Vision System**: DirectX screen capture with element detection
 * - **Overlay System**: Real-time egui overlay with visual feedback
 * - **Input System**: Voice commands and natural language processing
 * - **Safety System**: Comprehensive validation and user controls
 * 
 * ## Memory Management
 * 
 * Luna implements sophisticated memory management:
 * - Model caching with smart eviction
 * - Buffer pooling for image processing
 * - Automatic cleanup on memory pressure
 * - Real-time memory monitoring and alerts
 * 
 * ## Error Handling
 * 
 * All operations use Result types with detailed error context.
 * The system degrades gracefully when components fail.
 * 
 * ## Example Usage
 * 
 * ```rust
 * use luna_visual_ai::{LunaApp, core::config::Config};
 * 
 * #[tokio::main]
 * async fn main() -> anyhow::Result<()> {
 *     let config = Config::default();
 *     let app = LunaApp::new(config, true, false).await?;
 *     app.run().await
 * }
 * ```
 */

pub mod ai;
pub mod core;
pub mod input;
pub mod overlay;
pub mod utils;
pub mod vision;

// Re-export commonly used types
pub use crate::{
    core::{config::Config, error::LunaError},
    overlay::app::LunaApp,
};

use anyhow::Result;
use tracing::info;

/// Luna Visual AI library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Initialize Luna Visual AI library
/// 
/// This function should be called once at the start of your application.
/// It sets up logging, validates system requirements, and prepares
/// all subsystems for operation.
/// 
/// # Arguments
/// 
/// * `config` - Configuration for Luna components
/// 
/// # Returns
/// 
/// Returns `Ok(())` if initialization succeeds, or an error describing
/// what went wrong during setup.
/// 
/// # Example
/// 
/// ```rust
/// use luna_visual_ai::{init, Config};
/// 
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let config = Config::default();
///     init(&config).await?;
///     // Luna is now ready to use
///     Ok(())
/// }
/// ```
pub async fn init(config: &Config) -> Result<()> {
    info!("Initializing Luna Visual AI library v{}", VERSION);

    // Initialize logging if not already done
    if !utils::logging::is_initialized() {
        utils::logging::init("info", false)?;
    }

    // Initialize AI model manager
    ai::model_manager::init(config).await?;

    // Initialize screen capture system
    vision::screen_capture::init()?;

    // Initialize input system
    input::voice_processor::init(config).await?;

    // Validate all connections
    validate_all_connections().await?;

    info!("✅ Luna Visual AI library initialized successfully");
    Ok(())
}

/// Validate all system connections and components
/// 
/// This comprehensive validation ensures all Luna components are
/// functioning correctly and can communicate with each other.
/// 
/// # Validation Checks
/// 
/// - AI model loading and inference capability
/// - Windows API accessibility
/// - Screen capture functionality
/// - Audio input device availability
/// - Memory and resource availability
/// 
/// # Returns
/// 
/// Returns `Ok(())` if all validations pass, or detailed error
/// information about what component failed validation.
async fn validate_all_connections() -> Result<()> {
    info!("🔍 Validating all system connections...");

    // Validate AI models
    ai::model_manager::validate_models().await
        .map_err(|e| anyhow::anyhow!("AI model validation failed: {}", e))?;

    // Validate vision system
    vision::screen_capture::validate_apis().await
        .map_err(|e| anyhow::anyhow!("Vision system validation failed: {}", e))?;

    // Validate input system
    input::voice_processor::validate_audio_devices().await
        .map_err(|e| anyhow::anyhow!("Audio system validation failed: {}", e))?;

    // Validate memory availability
    core::memory::validate_memory_requirements()
        .map_err(|e| anyhow::anyhow!("Memory validation failed: {}", e))?;

    info!("✅ All system connections validated successfully");
    Ok(())
}

/// Graceful shutdown of Luna Visual AI
/// 
/// Properly shuts down all Luna components, ensuring:
/// - AI models are unloaded safely
/// - Memory is cleaned up
/// - System resources are released
/// - Temporary files are removed
/// 
/// This function should be called before your application exits.
pub async fn shutdown() -> Result<()> {
    info!("🌙 Shutting down Luna Visual AI...");

    // Shutdown AI model manager
    ai::model_manager::shutdown().await?;

    // Shutdown input system
    input::voice_processor::shutdown().await?;

    // Cleanup memory and resources
    core::memory::cleanup_all()?;

    info!("✅ Luna Visual AI shut down successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::Config;

    #[tokio::test]
    async fn test_init_and_shutdown() {
        let config = Config::default();
        
        // Test initialization
        let init_result = init(&config).await;
        assert!(init_result.is_ok(), "Failed to initialize Luna: {:?}", init_result);

        // Test shutdown
        let shutdown_result = shutdown().await;
        assert!(shutdown_result.is_ok(), "Failed to shutdown Luna: {:?}", shutdown_result);
    }

    #[tokio::test]
    async fn test_version_constant() {
        assert!(!VERSION.is_empty());
        assert!(VERSION.contains('.'));
    }
}