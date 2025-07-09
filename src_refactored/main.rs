// Main entry point for the refactored LUNA application
// Simplified architecture with reduced dependencies

use std::time::Duration;
use std::thread;

mod ai;
mod core;
mod input;
mod utils;
mod vision;
mod overlay;

use core::Luna;
use utils::logging::{Logger, LogLevel};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    let logger = Logger::new()
        .with_level(LogLevel::Info)
        .with_file("luna.log")?
        .with_console(true);
    
    utils::logging::init_logger(logger);

    println!("🌙 LUNA - Visual AI Assistant (Refactored)");
    println!("==========================================");
    
    // Create and configure LUNA
    let config = core::LunaConfig::default();
    let mut luna = Luna::new(config)?;
    
    // Initialize the system
    luna.initialize()?;
    
    println!("✅ LUNA initialized successfully");
    println!("🔍 Starting vision system...");
    
    // Main application loop
    loop {
        match run_luna_cycle(&mut luna) {
            Ok(should_continue) => {
                if !should_continue {
                    println!("👋 LUNA shutting down gracefully");
                    break;
                }
            }
            Err(e) => {
                eprintln!("❌ Error in LUNA cycle: {}", e);
                // Continue running unless it's a critical error
                if is_critical_error(&e) {
                    break;
                }
            }
        }
        
        // Small delay to prevent excessive CPU usage
        thread::sleep(Duration::from_millis(100));
    }
    
    // Cleanup
    luna.shutdown()?;
    println!("🌙 LUNA shutdown complete");
    
    Ok(())
}

fn run_luna_cycle(luna: &mut Luna) -> Result<bool, Box<dyn std::error::Error>> {
    // Check for user commands (in a real implementation, this would check for keyboard input, etc.)
    if check_for_exit_signal() {
        return Ok(false);
    }
    
    // Capture screen
    let screen_image = match luna.capture_screen() {
        Ok(image) => image,
        Err(e) => {
            log_debug!("Screen capture failed: {}", e);
            return Ok(true); // Continue despite capture failure
        }
    };
    
    // Analyze screen for UI elements
    let ui_elements = match luna.analyze_screen(&screen_image) {
        Ok(elements) => elements,
        Err(e) => {
            log_debug!("Screen analysis failed: {}", e);
            return Ok(true); // Continue despite analysis failure
        }
    };
    
    // Update overlay if elements were found
    if !ui_elements.is_empty() {
        log_info!("Found {} UI elements", ui_elements.len());
        luna.update_overlay(&ui_elements)?;
    }
    
    // Process any pending actions
    luna.process_pending_actions()?;
    
    Ok(true)
}

fn check_for_exit_signal() -> bool {
    // In a real implementation, this would check for:
    // - Ctrl+C signal
    // - Escape key press
    // - GUI close button
    // - Command line input
    
    // For this simplified version, we'll run indefinitely
    false
}

fn is_critical_error(error: &dyn std::error::Error) -> bool {
    // Determine if an error should cause the application to exit
    let error_string = error.to_string().to_lowercase();
    
    error_string.contains("critical") ||
    error_string.contains("fatal") ||
    error_string.contains("out of memory") ||
    error_string.contains("permission denied")
}

// Alternative simplified main for testing specific functionality
#[cfg(feature = "test-mode")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 LUNA Test Mode");
    
    // Test individual components
    test_vision_system()?;
    test_ai_system()?;
    test_overlay_system()?;
    
    println!("✅ All tests completed");
    Ok(())
}

#[cfg(feature = "test-mode")]
fn test_vision_system() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing vision system...");
    
    use vision::screen_capture::quick_screenshot;
    use vision::quick_analyze;
    
    // Capture screen
    let image = quick_screenshot()?;
    println!("  📸 Screen captured: {}x{}", image.width, image.height);
    
    // Analyze for UI elements
    let elements = quick_analyze(&image)?;
    println!("  🔍 Found {} UI elements", elements.len());
    
    for (i, element) in elements.iter().take(5).enumerate() {
        println!("    {}. {} at ({:.0}, {:.0}) - {:.1}% confidence", 
                i + 1, 
                element.element_type, 
                element.bounds.x, 
                element.bounds.y, 
                element.confidence * 100.0);
    }
    
    Ok(())
}

#[cfg(feature = "test-mode")]
fn test_ai_system() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing AI system...");
    
    use ai::VisionAI;
    use utils::image_processing::Image;
    
    let ai = VisionAI::new();
    let test_image = Image::new(100, 100, 3);
    
    let elements = ai.detect_ui_elements(&test_image)?;
    println!("  🤖 AI detected {} elements", elements.len());
    
    let text_content = ai.extract_text_content(&test_image)?;
    println!("  📝 AI extracted text: '{}'", text_content);
    
    Ok(())
}

#[cfg(feature = "test-mode")]
fn test_overlay_system() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing overlay system...");
    
    use overlay::{OverlayManager, OverlayConfig, Color};
    use utils::geometry::Rectangle;
    
    let mut manager = OverlayManager::new(OverlayConfig::default());
    
    // Add test overlays
    let rect = Rectangle::new(10.0, 10.0, 100.0, 50.0);
    let color = Color::rgb(255, 0, 0);
    manager.add_highlight(rect, color, Some("Test Highlight".to_string()));
    
    println!("  🎨 Created overlay with {} elements", manager.get_visible_elements().len());
    
    // Test animations
    let duration = Duration::from_millis(500);
    manager.update_animations(duration);
    
    Ok(())
}

// Command-line argument parsing for different modes
fn parse_args() -> AppMode {
    let args: Vec<String> = std::env::args().collect();
    
    for arg in &args[1..] {
        match arg.as_str() {
            "--test" => return AppMode::Test,
            "--demo" => return AppMode::Demo,
            "--headless" => return AppMode::Headless,
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            _ => {} // Ignore unknown arguments
        }
    }
    
    AppMode::Normal
}

#[derive(Debug)]
enum AppMode {
    Normal,
    Test,
    Demo,
    Headless,
}

fn print_help() {
    println!("LUNA - Visual AI Assistant");
    println!("");
    println!("USAGE:");
    println!("    luna [OPTIONS]");
    println!("");
    println!("OPTIONS:");
    println!("    --test      Run in test mode");
    println!("    --demo      Run demonstration mode");
    println!("    --headless  Run without overlay display");
    println!("    --help, -h  Show this help message");
    println!("");
    println!("EXAMPLES:");
    println!("    luna                 # Run normally");
    println!("    luna --test          # Test all systems");
    println!("    luna --headless      # Run without visual overlay");
}

// Performance monitoring
struct PerformanceMetrics {
    frame_count: u64,
    total_processing_time: Duration,
    last_fps_update: std::time::Instant,
}

impl PerformanceMetrics {
    fn new() -> Self {
        Self {
            frame_count: 0,
            total_processing_time: Duration::from_millis(0),
            last_fps_update: std::time::Instant::now(),
        }
    }
    
    fn record_frame(&mut self, processing_time: Duration) {
        self.frame_count += 1;
        self.total_processing_time += processing_time;
        
        // Report FPS every 5 seconds
        if self.last_fps_update.elapsed() >= Duration::from_secs(5) {
            let fps = self.frame_count as f64 / self.last_fps_update.elapsed().as_secs_f64();
            let avg_time = self.total_processing_time.as_millis() as f64 / self.frame_count as f64;
            
            log_info!("Performance: {:.1} FPS, {:.1}ms avg processing time", fps, avg_time);
            
            // Reset metrics
            self.frame_count = 0;
            self.total_processing_time = Duration::from_millis(0);
            self.last_fps_update = std::time::Instant::now();
        }
    }
}