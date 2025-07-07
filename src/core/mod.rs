/*!
 * Luna Visual AI Core Module
 * 
 * Core infrastructure components for Luna Visual AI:
 * - Configuration management with validation
 * - Comprehensive error handling and recovery
 * - Advanced memory management with monitoring
 * - Event system for inter-component communication
 * - Safety validation and user protection
 * 
 * This module provides the foundation upon which all other
 * Luna components are built.
 */

pub mod config;
pub mod error;
pub mod memory;
pub mod events;
pub mod safety;

// Re-export commonly used types
pub use config::Config;
pub use error::{LunaError, Result};
pub use memory::{MemoryStats, get_current_usage, get_available_memory};

use anyhow::Context;
use tracing::info;

/// Initialize all core subsystems
/// 
/// This function sets up the foundational components that all other
/// Luna systems depend on. It should be called once during application
/// startup before any other Luna operations.
/// 
/// # Arguments
/// 
/// * `config` - Configuration for all Luna components
/// 
/// # Returns
/// 
/// Returns `Ok(())` if all core systems initialize successfully,
/// or an error if any critical system fails to start.
pub async fn init(config: &Config) -> Result<()> {
    info!("Initializing Luna Visual AI core systems");

    // Validate configuration
    config.validate()?;

    // Initialize memory management
    memory::init(config.memory.clone())
        .context("Failed to initialize memory management")
        .map_err(|e| LunaError::internal(e.to_string(), "core::memory"))?;

    // Initialize event system
    events::init()
        .context("Failed to initialize event system")
        .map_err(|e| LunaError::internal(e.to_string(), "core::events"))?;

    // Initialize safety system
    safety::init(config.safety.clone())
        .context("Failed to initialize safety system")
        .map_err(|e| LunaError::internal(e.to_string(), "core::safety"))?;

    info!("✅ Core systems initialized successfully");
    Ok(())
}

/// Shutdown all core subsystems
/// 
/// Gracefully shuts down all core components, ensuring proper
/// cleanup of resources and persistent state.
pub async fn shutdown() -> Result<()> {
    info!("Shutting down Luna Visual AI core systems");

    // Shutdown in reverse order of initialization
    safety::shutdown().await?;
    events::shutdown().await?;
    memory::cleanup_all()?;

    info!("✅ Core systems shut down successfully");
    Ok(())
}

/// Validate that all core systems are healthy
/// 
/// Performs comprehensive health checks on all core components
/// to ensure they are functioning correctly.
pub async fn health_check() -> Result<()> {
    info!("Performing core systems health check");

    // Check memory system
    memory::validate_memory_requirements()?;

    // Check event system
    events::validate_system().await?;

    // Check safety system
    safety::validate_system().await?;

    info!("✅ Core systems health check passed");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_core_init_shutdown() {
        let config = Config::default();
        
        let init_result = init(&config).await;
        assert!(init_result.is_ok(), "Failed to initialize core: {:?}", init_result);

        let health_result = health_check().await;
        assert!(health_result.is_ok(), "Health check failed: {:?}", health_result);

        let shutdown_result = shutdown().await;
        assert!(shutdown_result.is_ok(), "Failed to shutdown core: {:?}", shutdown_result);
    }
}