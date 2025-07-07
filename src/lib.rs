/*!
 * Luna Visual AI Library
 * 
 * Core functionality for the Luna Visual AI assistant
 */

pub mod ai;
pub mod core;
pub mod input;
pub mod overlay;
pub mod utils;
pub mod vision;

// Re-export main types for easier access
pub use ai::{AIVisionPipeline, AICoordinator};
pub use core::{LunaCore, LunaAction, LunaError, ScreenAnalysis, ScreenElement};
pub use input::WindowsInputSystem;
pub use overlay::VisualOverlay;
pub use utils::{MetricsCollector, setup_logging};
pub use vision::ScreenCapture;

/// Luna version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NAME: &str = env!("CARGO_PKG_NAME");
pub const DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");

/// Initialize Luna library with default configuration
pub async fn init() -> anyhow::Result<LunaCore> {
    // Setup logging
    setup_logging()?;
    
    // Initialize metrics
    utils::init_global_metrics();
    
    // Create and initialize Luna core
    LunaCore::new().await
}

/// Initialize Luna library with custom configuration
pub async fn init_with_config(config: core::LunaConfig) -> anyhow::Result<LunaCore> {
    // Setup logging
    setup_logging()?;
    
    // Initialize metrics
    utils::init_global_metrics();
    
    // Create Luna core (would pass config in real implementation)
    LunaCore::new().await
}

/// Get Luna version information
pub fn version_info() -> VersionInfo {
    VersionInfo {
        name: NAME.to_string(),
        version: VERSION.to_string(),
        description: DESCRIPTION.to_string(),
    }
}

/// Version information structure
#[derive(Debug, Clone)]
pub struct VersionInfo {
    pub name: String,
    pub version: String,
    pub description: String,
}

/// Check system compatibility
pub fn check_compatibility() -> CompatibilityReport {
    let permissions = utils::WindowsApiHelper::check_permissions();
    let system_info = utils::WindowsApiHelper::get_system_info();
    
    CompatibilityReport {
        compatible: permissions.all_granted() && system_info.memory_usage_mb < 1000,
        windows_version: utils::WinApiUtils::get_windows_version(),
        permissions,
        issues: get_compatibility_issues(&permissions, &system_info),
        recommendations: get_recommendations(&permissions, &system_info),
    }
}

/// Compatibility report
#[derive(Debug, Clone)]
pub struct CompatibilityReport {
    pub compatible: bool,
    pub windows_version: String,
    pub permissions: utils::PermissionStatus,
    pub issues: Vec<String>,
    pub recommendations: Vec<String>,
}

fn get_compatibility_issues(
    permissions: &utils::PermissionStatus,
    system_info: &utils::SystemInfo,
) -> Vec<String> {
    let mut issues = Vec::new();
    
    if !permissions.screen_capture {
        issues.push("Screen capture not available".to_string());
    }
    
    if !permissions.input_simulation {
        issues.push("Input simulation not available".to_string());
    }
    
    if system_info.memory_usage_mb > 800 {
        issues.push("High memory usage detected".to_string());
    }
    
    if system_info.cpu_usage_percent > 80.0 {
        issues.push("High CPU usage detected".to_string());
    }
    
    issues
}

fn get_recommendations(
    permissions: &utils::PermissionStatus,
    system_info: &utils::SystemInfo,
) -> Vec<String> {
    let mut recommendations = Vec::new();
    
    if !permissions.all_granted() {
        recommendations.push("Run Luna as administrator for full functionality".to_string());
    }
    
    if system_info.memory_usage_mb > 500 {
        recommendations.push("Close other applications to free memory".to_string());
    }
    
    if !utils::WinApiUtils::is_admin() {
        recommendations.push("Consider running as administrator".to_string());
    }
    
    recommendations.push("Ensure Windows Defender allows Luna to run".to_string());
    
    recommendations
}

/// Global error handler for Luna
pub fn set_error_handler<F>(handler: F) 
where 
    F: Fn(&LunaError) + Send + Sync + 'static 
{
    // In real implementation, would set a global error handler
    // For now, just a placeholder
    std::panic::set_hook(Box::new(move |panic_info| {
        eprintln!("Luna panic: {:?}", panic_info);
    }));
}

/// Cleanup Luna resources
pub async fn cleanup() {
    // Flush logs
    utils::flush_logs();
    
    // Export final metrics
    if let Ok(metrics_json) = utils::global_metrics().export_json() {
        // In real implementation, would save metrics to file
        tracing::debug!("Final metrics: {}", metrics_json);
    }
    
    tracing::info!("Luna cleanup completed");
}