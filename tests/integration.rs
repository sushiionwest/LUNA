/*!
 * Luna Integration Tests
 */

use luna::{init, LunaAction, LunaCore};
use tokio_test;

#[tokio::test]
async fn test_luna_initialization() {
    let result = init().await;
    assert!(result.is_ok(), "Luna initialization should succeed");
}

#[tokio::test]
async fn test_screen_capture() {
    let luna = init().await.expect("Luna should initialize");
    // Test would capture screen and verify it works
    // For now, just test that the system doesn't crash
}

#[tokio::test]
async fn test_basic_command() {
    let luna = init().await.expect("Luna should initialize");
    
    // Test a safe command that shouldn't actually execute anything dangerous
    let result = luna.execute_command("Wait 100ms").await;
    
    // The command might fail due to parsing, but shouldn't crash
    // In a real test environment, we'd mock the input system
}

#[test]
fn test_version_info() {
    let version = luna::version_info();
    assert_eq!(version.name, "luna");
    assert_eq!(version.version, "1.0.0");
}

#[test]
fn test_compatibility_check() {
    let compat = luna::check_compatibility();
    // Should at least detect Windows version
    assert!(!compat.windows_version.is_empty());
}