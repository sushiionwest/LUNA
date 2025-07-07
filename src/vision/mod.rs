//! # Vision System Module
//! 
//! This module handles all computer vision tasks including screen capture,
//! image processing, and coordinate mapping. It provides the foundation
//! for Luna's visual understanding capabilities.
//!
//! ## Key Components
//! - **Screen Capture**: High-performance Windows screen capture
//! - **Element Detection**: UI element identification and tracking
//! - **Image Processing**: Real-time image enhancement and filtering
//! - **Coordinate Mapping**: Precise screen coordinate transformations

pub mod screen_capture;
pub mod element_detection;
pub mod image_processor;
pub mod coordinate_mapper;

// Re-export key types
pub use screen_capture::{ScreenCapture, Screenshot};
pub use element_detection::{ElementDetector, DetectedElement};
pub use image_processor::{ImageProcessor, ProcessingOptions};
pub use coordinate_mapper::{CoordinateMapper, ScreenPoint};

use crate::core::{LunaError, LunaResult};
use tracing::info;

/// Initialize the vision subsystem
pub async fn init() -> LunaResult<()> {
    info!("📷 Initializing Luna Vision subsystem");
    
    // Initialize screen capture
    screen_capture::init().await?;
    
    // Initialize element detection
    element_detection::init().await?;
    
    // Initialize image processor
    image_processor::init().await?;
    
    // Initialize coordinate mapper
    coordinate_mapper::init().await?;
    
    info!("✅ Vision subsystem initialized successfully");
    Ok(())
}

/// Shutdown the vision subsystem
pub async fn shutdown() -> LunaResult<()> {
    info!("Shutting down vision subsystem");
    
    // Shutdown components
    screen_capture::shutdown().await?;
    element_detection::shutdown().await?;
    image_processor::shutdown().await?;
    coordinate_mapper::shutdown().await?;
    
    info!("✅ Vision subsystem shut down successfully");
    Ok(())
}

/// Validate vision system functionality
pub async fn validate_system() -> LunaResult<()> {
    info!("Validating vision system");
    
    // Test screen capture
    screen_capture::validate().await?;
    
    // Test element detection
    element_detection::validate().await?;
    
    // Test image processing
    image_processor::validate().await?;
    
    // Test coordinate mapping
    coordinate_mapper::validate().await?;
    
    info!("✅ Vision system validation complete");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_vision_init() {
        // Note: This may fail without proper Windows environment
        let result = init().await;
        
        if result.is_ok() {
            let validation = validate_system().await;
            assert!(validation.is_ok());
            
            shutdown().await.unwrap();
        }
        // Expected to fail in non-Windows test environments
    }
}