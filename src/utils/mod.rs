/*!
 * Luna Visual AI Utilities Module
 * 
 * Common utilities and helper functions used throughout Luna:
 * - Advanced logging system with security filtering
 * - Performance monitoring and metrics collection
 * - Windows API helpers and wrappers
 * - Memory management utilities
 * - Configuration helpers
 * - Development and debugging tools
 */

pub mod logging;
pub mod metrics;
pub mod windows_api;

// Re-export commonly used utilities
pub use logging::{init as init_logging, is_initialized as logging_initialized};

use crate::core::error::{LunaError, Result};
use tracing::info;

/// Initialize all utility subsystems
pub async fn init() -> Result<()> {
    info!("Initializing Luna Visual AI utilities");

    // Logging is typically initialized first, before this call
    // Additional utility initialization can be added here

    info!("✅ Utilities initialized successfully");
    Ok(())
}

/// Shutdown all utility subsystems
pub async fn shutdown() -> Result<()> {
    info!("Shutting down Luna Visual AI utilities");

    // Cleanup any utility resources
    // Logging cleanup is handled automatically by tracing

    info!("✅ Utilities shut down successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_utils_init_shutdown() {
        let init_result = init().await;
        assert!(init_result.is_ok());

        let shutdown_result = shutdown().await;
        assert!(shutdown_result.is_ok());
    }
}