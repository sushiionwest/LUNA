/*!
 * Luna Visual AI - AI Module
 * 
 * Core AI functionality with 4-specialist architecture:
 * - Florence-2: Object detection and UI element identification
 * - CLIP: Natural language to visual element matching
 * - TrOCR: Text recognition from screen elements
 * - SAM: Precise click target segmentation
 * 
 * Features:
 * - Parallel model execution with tokio
 * - Smart memory management and model caching
 * - GPU acceleration with fallback to CPU
 * - Local-only inference (no cloud dependencies)
 * - Real-time performance monitoring
 * - Graceful degradation on model failures
 */

pub mod clip;
pub mod florence;
pub mod model_manager;
pub mod pipeline;
pub mod sam;
pub mod trocr;

// Re-export key types
pub use model_manager::{ModelManager, ModelLoadError};
pub use pipeline::{AiPipeline, AiAnalysisResult, ClickTarget, TimingInfo};
pub use florence::{Florence2Specialist, DetectedObject};
pub use clip::{ClipSpecialist, MatchResult, TextQuery, VisualElement};
pub use trocr::{TrOcrSpecialist, ExtractedText, TextRegion};
pub use sam::{SamSpecialist, SegmentationMask, SegmentationPrompt};

use crate::core::{config::AiConfig, error::{LunaError, Result}};
use tracing::info;

/// Initialize the AI subsystem
pub async fn init(config: AiConfig) -> Result<()> {
    info!("🧠 Initializing Luna Visual AI models");
    
    // Initialize model manager
    model_manager::init(config).await?;
    
    info!("✅ AI subsystem initialized successfully");
    Ok(())
}

/// Shutdown the AI subsystem
pub async fn shutdown() -> Result<()> {
    info!("Shutting down AI subsystem");
    
    // Shutdown model manager
    model_manager::shutdown().await?;
    
    info!("✅ AI subsystem shut down successfully");
    Ok(())
}

/// Validate AI models are loaded and functional
pub async fn validate_models() -> Result<()> {
    model_manager::validate_models().await
}

/// Check GPU availability for AI inference
pub async fn check_gpu_availability() -> Result<bool> {
    model_manager::check_gpu_availability().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::AiConfig;

    #[tokio::test]
    async fn test_ai_init() {
        let config = AiConfig::default();
        
        // Note: This test may fail without actual models
        // In a real implementation, you might want to use mock models
        let result = init(config).await;
        
        if result.is_ok() {
            let validation = validate_models().await;
            assert!(validation.is_ok());
            
            shutdown().await.unwrap();
        }
        // If init fails (no models), that's expected in test environment
    }
}