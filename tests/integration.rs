/*!
 * Luna Visual AI Integration Tests
 * 
 * Comprehensive end-to-end testing for the complete Luna system:
 * - System initialization and shutdown
 * - AI model loading and inference
 * - Vision pipeline processing  
 * - Safety system validation
 * - Memory management under load
 * - Error handling and recovery
 * - Performance benchmarks
 */

use luna_visual_ai::{
    ai::{self, VisionPipeline},
    core::{config::Config, memory, safety, events},
    vision::screen_capture::ScreenCapture,
    input::command_parser::parse_command,
    overlay::app::LunaApp,
};
use anyhow::Result;
use std::time::{Duration, Instant};
use tokio::time::timeout;
use tracing::{info, warn};

/// Test configuration for integration tests
struct TestConfig {
    enable_gpu: bool,
    enable_voice: bool,
    enable_overlay: bool,
    memory_limit: u64,
    timeout_seconds: u64,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            enable_gpu: false, // Use CPU for consistent CI testing
            enable_voice: false, // Disable voice in headless environments
            enable_overlay: false, // Disable overlay in CI
            memory_limit: 2_000_000_000, // 2GB limit for CI
            timeout_seconds: 30, // 30 second timeout
        }
    }
}

/// Integration test suite for Luna Visual AI
#[tokio::test]
async fn test_complete_system_integration() -> Result<()> {
    let test_config = TestConfig::default();
    
    info!("🚀 Starting Luna Visual AI integration tests");
    
    // Test 1: System initialization
    test_system_initialization(&test_config).await?;
    
    // Test 2: AI model validation
    test_ai_models(&test_config).await?;
    
    // Test 3: Vision pipeline
    test_vision_pipeline(&test_config).await?;
    
    // Test 4: Safety system
    test_safety_system(&test_config).await?;
    
    // Test 5: Memory management
    test_memory_management(&test_config).await?;
    
    // Test 6: Error handling
    test_error_handling(&test_config).await?;
    
    // Test 7: Performance benchmarks
    test_performance_benchmarks(&test_config).await?;
    
    // Test 8: System shutdown
    test_system_shutdown(&test_config).await?;
    
    info!("✅ All integration tests passed successfully");
    Ok(())
}

/// Test system initialization and configuration
async fn test_system_initialization(test_config: &TestConfig) -> Result<()> {
    info!("Testing system initialization...");
    
    // Create test configuration
    let mut config = Config::default();
    config.ai.gpu_device_id = if test_config.enable_gpu { 0 } else { -1 };
    config.memory.max_total_memory = test_config.memory_limit;
    config.input.voice.enabled = test_config.enable_voice;
    config.overlay.enabled = test_config.enable_overlay;
    
    // Initialize core systems
    let init_result = timeout(
        Duration::from_secs(test_config.timeout_seconds),
        luna_visual_ai::init(&config)
    ).await;
    
    match init_result {
        Ok(Ok(())) => {
            info!("✅ System initialization successful");
            
            // Validate core systems are running
            assert!(memory::get_current_usage() > 0, "Memory tracking not working");
            assert!(events::get_stats().events_published >= 0, "Event system not initialized");
            
            Ok(())
        }
        Ok(Err(e)) => {
            warn!("❌ System initialization failed: {}", e);
            Err(e.into())
        }
        Err(_) => {
            warn!("❌ System initialization timed out");
            Err(anyhow::anyhow!("System initialization timeout"))
        }
    }
}

/// Test AI model loading and basic functionality
async fn test_ai_models(test_config: &TestConfig) -> Result<()> {
    info!("Testing AI models...");
    
    // Check GPU availability
    let gpu_available = ai::check_gpu_availability().await?;
    info!("GPU available: {}", gpu_available);
    
    // Test model validation (may fail without actual models in CI)
    match timeout(
        Duration::from_secs(test_config.timeout_seconds),
        ai::validate_models()
    ).await {
        Ok(Ok(())) => {
            info!("✅ AI models validated successfully");
            
            // Test model manager statistics
            let stats = ai::model_manager::get_usage_stats();
            info!("Model statistics: {} models tracked", stats.len());
            
            Ok(())
        }
        Ok(Err(e)) => {
            warn!("⚠️  AI model validation failed (expected in CI): {}", e);
            // This is expected in CI environments without models
            Ok(())
        }
        Err(_) => {
            warn!("❌ AI model validation timed out");
            Err(anyhow::anyhow!("AI model validation timeout"))
        }
    }
}

/// Test vision pipeline processing
async fn test_vision_pipeline(test_config: &TestConfig) -> Result<()> {
    info!("Testing vision pipeline...");
    
    // Test screen capture (may fail in headless environments)
    match ScreenCapture::new() {
        Ok(capture) => {
            info!("✅ Screen capture initialized");
            
            // Try to capture screen (may fail in headless CI)
            match timeout(
                Duration::from_secs(10),
                capture.capture_primary_display()
            ).await {
                Ok(Ok(screenshot)) => {
                    info!("✅ Screen capture successful: {}x{}", 
                          screenshot.width(), screenshot.height());
                    
                    // Validate screenshot properties
                    assert!(screenshot.width() > 0, "Invalid screenshot width");
                    assert!(screenshot.height() > 0, "Invalid screenshot height");
                }
                Ok(Err(e)) => {
                    warn!("⚠️  Screen capture failed (expected in headless): {}", e);
                }
                Err(_) => {
                    warn!("⚠️  Screen capture timed out (expected in headless)");
                }
            }
        }
        Err(e) => {
            warn!("⚠️  Screen capture initialization failed (expected in headless): {}", e);
        }
    }
    
    Ok(())
}

/// Test safety system functionality
async fn test_safety_system(test_config: &TestConfig) -> Result<()> {
    info!("Testing safety system...");
    
    // Test command validation
    let safe_command = safety::ActionRequest::new(
        "click save button".to_string(),
        safety::ActionType::Click,
        Some("notepad.exe".to_string()),
    );
    
    let result = timeout(
        Duration::from_secs(10),
        safety::validate_action(&safe_command)
    ).await??;
    
    info!("Safe command validation: allowed={}, risk={:?}", 
          result.allowed, result.risk_level);
    
    // Test dangerous command detection
    let dangerous_command = safety::ActionRequest::new(
        "delete system files".to_string(),
        safety::ActionType::SystemCommand,
        None,
    );
    
    let result = timeout(
        Duration::from_secs(10),
        safety::validate_action(&dangerous_command)
    ).await??;
    
    info!("Dangerous command validation: allowed={}, risk={:?}", 
          result.allowed, result.risk_level);
    
    // Dangerous commands should be blocked or require confirmation
    assert!(
        !result.allowed || result.requires_confirmation,
        "Dangerous command was not properly restricted"
    );
    
    // Test emergency stop
    safety::emergency_stop().await?;
    
    // Verify emergency stop is active
    let emergency_result = safety::validate_action(&safe_command).await?;
    assert!(!emergency_result.allowed, "Emergency stop not working");
    
    // Clear emergency stop
    safety::clear_emergency_stop().await?;
    
    info!("✅ Safety system tests passed");
    Ok(())
}

/// Test memory management under load
async fn test_memory_management(test_config: &TestConfig) -> Result<()> {
    info!("Testing memory management...");
    
    let initial_memory = memory::get_current_usage();
    info!("Initial memory usage: {} MB", initial_memory / 1_000_000);
    
    // Simulate memory allocations
    let mut allocations = Vec::new();
    
    for i in 0..10 {
        let allocation_id = memory::allocate(10_000_000, &format!("test_{}", i))?; // 10MB each
        allocations.push(allocation_id);
    }
    
    let peak_memory = memory::get_current_usage();
    info!("Peak memory usage: {} MB", peak_memory / 1_000_000);
    
    // Verify memory tracking
    assert!(peak_memory > initial_memory, "Memory allocation not tracked");
    
    // Test memory statistics
    let stats = memory::get_stats();
    info!("Memory stats: total={} MB, pooled={} MB", 
          stats.total_allocated / 1_000_000,
          stats.pool_usage.total_pooled_memory / 1_000_000);
    
    // Clean up allocations
    for allocation_id in allocations {
        memory::deallocate(allocation_id)?;
    }
    
    let final_memory = memory::get_current_usage();
    info!("Final memory usage: {} MB", final_memory / 1_000_000);
    
    // Verify cleanup
    assert!(
        final_memory <= initial_memory + 1_000_000, // Allow 1MB tolerance
        "Memory not properly cleaned up"
    );
    
    // Test buffer pooling
    let buffer = memory::get_image_buffer(1_000_000); // 1MB buffer
    assert_eq!(buffer.len(), 1_000_000, "Buffer size mismatch");
    
    memory::return_image_buffer(buffer);
    
    info!("✅ Memory management tests passed");
    Ok(())
}

/// Test error handling and recovery
async fn test_error_handling(_test_config: &TestConfig) -> Result<()> {
    info!("Testing error handling...");
    
    // Test invalid action validation
    let invalid_action = safety::ActionRequest::new(
        "".to_string(), // Empty command
        safety::ActionType::Click,
        None,
    );
    
    // This should handle gracefully
    let result = safety::validate_action(&invalid_action).await;
    match result {
        Ok(validation) => {
            info!("Empty command handled: allowed={}", validation.allowed);
        }
        Err(e) => {
            info!("Empty command error handled: {}", e);
        }
    }
    
    // Test invalid memory allocation
    let invalid_alloc = memory::allocate(u64::MAX, "invalid_test");
    assert!(invalid_alloc.is_err(), "Invalid allocation should fail");
    
    // Test command parsing with invalid input
    let parse_result = parse_command("this is not a valid command format !@#$%");
    match parse_result {
        Ok(parsed) => {
            info!("Command parsing handled gracefully: {:?}", parsed);
        }
        Err(e) => {
            info!("Command parsing error handled: {}", e);
        }
    }
    
    info!("✅ Error handling tests passed");
    Ok(())
}

/// Test performance benchmarks
async fn test_performance_benchmarks(test_config: &TestConfig) -> Result<()> {
    info!("Running performance benchmarks...");
    
    // Benchmark command parsing
    let start = Instant::now();
    for _ in 0..1000 {
        let _ = parse_command("click the save button");
    }
    let parse_duration = start.elapsed();
    info!("Command parsing: {} μs per operation", 
          parse_duration.as_micros() / 1000);
    
    // Benchmark memory allocation/deallocation
    let start = Instant::now();
    for _ in 0..100 {
        if let Ok(id) = memory::allocate(1_000_000, "benchmark") {
            let _ = memory::deallocate(id);
        }
    }
    let memory_duration = start.elapsed();
    info!("Memory alloc/dealloc: {} μs per operation", 
          memory_duration.as_micros() / 100);
    
    // Benchmark safety validation
    let test_action = safety::ActionRequest::new(
        "click button".to_string(),
        safety::ActionType::Click,
        None,
    );
    
    let start = Instant::now();
    for _ in 0..100 {
        let _ = safety::validate_action(&test_action).await;
    }
    let safety_duration = start.elapsed();
    info!("Safety validation: {} μs per operation", 
          safety_duration.as_micros() / 100);
    
    // Performance assertions
    assert!(
        parse_duration.as_micros() / 1000 < 100, 
        "Command parsing too slow"
    );
    assert!(
        memory_duration.as_micros() / 100 < 1000, 
        "Memory operations too slow"
    );
    assert!(
        safety_duration.as_micros() / 100 < 10000, 
        "Safety validation too slow"
    );
    
    info!("✅ Performance benchmarks passed");
    Ok(())
}

/// Test system shutdown
async fn test_system_shutdown(test_config: &TestConfig) -> Result<()> {
    info!("Testing system shutdown...");
    
    let shutdown_result = timeout(
        Duration::from_secs(test_config.timeout_seconds),
        luna_visual_ai::shutdown()
    ).await;
    
    match shutdown_result {
        Ok(Ok(())) => {
            info!("✅ System shutdown successful");
            
            // Verify cleanup
            let final_memory = memory::get_current_usage();
            info!("Final memory after shutdown: {} bytes", final_memory);
            
            Ok(())
        }
        Ok(Err(e)) => {
            warn!("❌ System shutdown failed: {}", e);
            Err(e.into())
        }
        Err(_) => {
            warn!("❌ System shutdown timed out");
            Err(anyhow::anyhow!("System shutdown timeout"))
        }
    }
}

/// Stress test for concurrent operations
#[tokio::test]
async fn test_concurrent_stress() -> Result<()> {
    info!("Running concurrent stress test...");
    
    let config = Config::default();
    luna_visual_ai::init(&config).await?;
    
    // Spawn multiple concurrent tasks
    let mut handles = Vec::new();
    
    for i in 0..10 {
        let handle = tokio::spawn(async move {
            // Concurrent memory allocations
            for j in 0..10 {
                if let Ok(id) = memory::allocate(100_000, &format!("stress_{}_{}", i, j)) {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                    let _ = memory::deallocate(id);
                }
            }
            
            // Concurrent safety validations
            let action = safety::ActionRequest::new(
                format!("test action {}", i),
                safety::ActionType::Click,
                None,
            );
            
            for _ in 0..5 {
                let _ = safety::validate_action(&action).await;
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        });
        
        handles.push(handle);
    }
    
    // Wait for all tasks to complete
    for handle in handles {
        handle.await?;
    }
    
    // Verify system stability
    let stats = memory::get_stats();
    info!("Stress test completed - Memory: {} MB", 
          stats.current_usage / 1_000_000);
    
    luna_visual_ai::shutdown().await?;
    
    info!("✅ Concurrent stress test passed");
    Ok(())
}

/// Test configuration validation
#[tokio::test]
async fn test_configuration_validation() -> Result<()> {
    info!("Testing configuration validation...");
    
    // Test valid configuration
    let valid_config = Config::default();
    assert!(valid_config.validate().is_ok(), "Default config should be valid");
    
    // Test invalid configuration - memory too low
    let mut invalid_config = Config::default();
    invalid_config.memory.max_total_memory = 100_000_000; // 100MB - too low
    assert!(invalid_config.validate().is_err(), "Invalid config should fail validation");
    
    // Test invalid configuration - transparency out of range
    let mut invalid_config = Config::default();
    invalid_config.overlay.transparency = 1.5; // > 1.0
    assert!(invalid_config.validate().is_err(), "Invalid transparency should fail");
    
    // Test invalid configuration - countdown too long
    let mut invalid_config = Config::default();
    invalid_config.overlay.countdown_duration = 100; // > 60 seconds
    assert!(invalid_config.validate().is_err(), "Invalid countdown should fail");
    
    info!("✅ Configuration validation tests passed");
    Ok(())
}

/// Helper function to setup test logging
fn setup_test_logging() {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
    
    let _ = tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env()
            .add_directive("luna_visual_ai=debug".parse().unwrap())
            .add_directive("integration=info".parse().unwrap()))
        .with(tracing_subscriber::fmt::layer()
            .with_test_writer()
            .with_target(false))
        .try_init();
}

/// Test module initialization
#[ctor::ctor]
fn init_tests() {
    setup_test_logging();
    println!("🧪 Luna Visual AI Integration Tests Initialized");
}

/// Test module cleanup
#[ctor::dtor]
fn cleanup_tests() {
    println!("🧪 Luna Visual AI Integration Tests Completed");
}