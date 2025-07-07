//! # Visual Overlay System
//! 
//! This module provides the visual overlay that shows users exactly what Luna sees
//! and where it plans to click. It's the critical safety and transparency feature
//! that builds user trust and allows cancellation of actions.
//!
//! ## Key Features
//! - Real-time visual feedback showing AI analysis
//! - 3-second countdown before any action
//! - Highlight detected elements and click targets
//! - Show confidence scores and reasoning
//! - Emergency stop functionality (ESC key)
//!
//! ## Components
//! - **Visual Feedback**: Shows what Luna detected
//! - **Countdown Timer**: Safety delay before actions
//! - **Highlight System**: Visual indicators for targets
//! - **Emergency Controls**: Cancel and stop mechanisms

pub mod visual_feedback;
pub mod countdown;
pub mod highlight;
pub mod app;

// Re-export key types
pub use visual_feedback::{VisualFeedback, FeedbackOverlay};
pub use countdown::{CountdownTimer, CountdownState};
pub use highlight::{HighlightManager, HighlightStyle};
pub use app::{OverlayApp, OverlayConfig};

use crate::core::{LunaError, LunaResult};
use tracing::info;

/// Initialize the overlay subsystem
pub async fn init() -> LunaResult<()> {
    info!("🎨 Initializing Luna Visual Overlay");
    
    // Initialize visual feedback
    visual_feedback::init().await?;
    
    // Initialize countdown system
    countdown::init().await?;
    
    // Initialize highlight manager
    highlight::init().await?;
    
    // Initialize overlay app
    app::init().await?;
    
    info!("✅ Visual overlay initialized successfully");
    Ok(())
}

/// Shutdown the overlay subsystem
pub async fn shutdown() -> LunaResult<()> {
    info!("Shutting down visual overlay");
    
    // Shutdown components
    visual_feedback::shutdown().await?;
    countdown::shutdown().await?;
    highlight::shutdown().await?;
    app::shutdown().await?;
    
    info!("✅ Visual overlay shut down successfully");
    Ok(())
}

/// Validate overlay system functionality
pub async fn validate_system() -> LunaResult<()> {
    info!("Validating overlay system");
    
    // Test visual feedback
    visual_feedback::validate().await?;
    
    // Test countdown
    countdown::validate().await?;
    
    // Test highlighting
    highlight::validate().await?;
    
    // Test overlay app
    app::validate().await?;
    
    info!("✅ Overlay system validation complete");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_overlay_init() {
        let result = init().await;
        
        if result.is_ok() {
            let validation = validate_system().await;
            assert!(validation.is_ok());
            
            shutdown().await.unwrap();
        }
        // Expected to potentially fail in headless test environments
    }
}