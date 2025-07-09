# LUNA API Documentation

## Overview

LUNA provides a comprehensive API for visual automation and screen analysis. The refactored architecture offers both high-level convenience functions and low-level control for advanced use cases.

## Quick Start

### Basic Usage

```rust
use luna;

// Initialize LUNA with defaults
let mut luna = luna::init()?;

// Analyze current screen
let elements = luna.analyze_current_screen()?;
println!("Found {} UI elements", elements.len());
```

### Configuration

```rust
use luna::{LunaConfig, core::SafetyLevel};

let config = LunaConfig {
    enable_overlay: true,
    safety_level: SafetyLevel::High,
    capture_fps: 30,
    ..Default::default()
};

let mut luna = luna::init_with_config(config)?;
```

## Core API

### Luna Struct

The main application controller that coordinates all subsystems.

```rust
pub struct Luna {
    // Internal fields...
}

impl Luna {
    /// Create a new LUNA instance with custom configuration
    pub fn new(config: LunaConfig) -> Result<Self, LunaError>;
    
    /// Initialize all subsystems
    pub fn initialize(&mut self) -> Result<(), LunaError>;
    
    /// Capture the current screen
    pub fn capture_screen(&self) -> Result<Image, LunaError>;
    
    /// Analyze an image for UI elements
    pub fn analyze_screen(&self, image: &Image) -> Result<Vec<UIElement>, LunaError>;
    
    /// Update the visual overlay with new elements
    pub fn update_overlay(&mut self, elements: &[UIElement]) -> Result<(), LunaError>;
    
    /// Process any pending automation actions
    pub fn process_pending_actions(&mut self) -> Result<(), LunaError>;
    
    /// Shutdown LUNA and cleanup resources
    pub fn shutdown(&mut self) -> Result<(), LunaError>;
}
```

### Configuration

```rust
pub struct LunaConfig {
    /// Enable visual overlay system
    pub enable_overlay: bool,
    
    /// Safety level for input automation
    pub safety_level: SafetyLevel,
    
    /// Screen capture frame rate
    pub capture_fps: u32,
    
    /// Logging level
    pub log_level: String,
    
    /// Vision system configuration
    pub vision_config: VisionConfig,
    
    /// Overlay system configuration
    pub overlay_config: OverlayConfig,
}

pub enum SafetyLevel {
    Low,    // Minimal safety checks
    Medium, // Standard safety validation
    High,   // Strict safety enforcement
}
```

## Vision API

### Screen Capture

```rust
use luna::vision::screen_capture::{ScreenCapture, CaptureConfig, quick_screenshot};

// Quick screenshot
let image = quick_screenshot()?;

// Advanced capture with configuration
let config = CaptureConfig {
    target_fps: 60,
    capture_cursor: false,
    capture_region: Some(CaptureRegion {
        x: 0, y: 0, width: 1920, height: 1080
    }),
    ..Default::default()
};

let mut capture = ScreenCapture::new(config);
let image = capture.capture_screen()?;
```

### UI Element Detection

```rust
use luna::vision::{quick_analyze, find_buttons, find_text_boxes};

// Quick analysis
let elements = quick_analyze(&image)?;

// Find specific element types
let buttons = find_buttons(&image)?;
let text_boxes = find_text_boxes(&image)?;

// Advanced detection
use luna::vision::ui_detection::UIDetector;

let detector = UIDetector::new();
let all_elements = detector.detect_all_elements(&image)?;
let buttons_only = detector.detect_buttons(&image)?;
```

### UI Element Structure

```rust
pub struct UIElement {
    /// Bounding rectangle of the element
    pub bounds: Rectangle,
    
    /// Type of UI element detected
    pub element_type: ElementType,
    
    /// Detection confidence (0.0 to 1.0)
    pub confidence: f64,
    
    /// Additional properties specific to element type
    pub properties: HashMap<String, String>,
}

pub enum ElementType {
    Button,
    TextBox,
    Label,
    Menu,
    Window,
    Icon,
    Image,
    Unknown,
}
```

### Text Recognition

```rust
use luna::vision::text_recognition::{TextRecognizer, extract_text_from_image};

// Quick text extraction
let text = extract_text_from_image(&image)?;

// Advanced text recognition
let recognizer = TextRecognizer::new();
let text_regions = recognizer.recognize_text(&image)?;

for region in text_regions {
    println!("Text: '{}' at ({:.0}, {:.0}) - {:.1}% confidence",
             region.text, region.bounds.x, region.bounds.y, 
             region.confidence * 100.0);
}
```

## Input API

### Safe Input Automation

```rust
use luna::input::{InputController, BasicSafetyChecker, InputAction, ActionType};

// Create controller with safety
let safety_checker = Box::new(BasicSafetyChecker::new());
let mut input = InputController::new(safety_checker);

// Create input actions
let click_action = InputAction {
    action_type: ActionType::Click { 
        button: luna::input::MouseButton::Left 
    },
    target: luna::input::Target {
        x: 100, y: 200,
        element_type: Some("button".to_string()),
    },
    timestamp: std::time::Instant::now(),
};

// Execute with automatic safety validation
input.execute_action(click_action)?;
```

### Action Types

```rust
pub enum ActionType {
    Click { button: MouseButton },
    Type { text: String },
    Key { key: String },
    Scroll { direction: ScrollDirection, amount: i32 },
    Move { x: i32, y: i32 },
}

pub enum MouseButton {
    Left,
    Right,
    Middle,
}

pub enum ScrollDirection {
    Up,
    Down,
    Left,
    Right,
}
```

### Safety System

```rust
use luna::input::{SafetyChecker, RiskLevel};

pub trait SafetyChecker {
    /// Check if an action is safe to execute
    fn is_action_safe(&self, action: &InputAction) -> bool;
    
    /// Get the risk level of an action
    fn get_risk_level(&self, action: &InputAction) -> RiskLevel;
}

pub enum RiskLevel {
    Safe,
    Low,
    Medium,
    High,
    Critical,
}
```

## Overlay API

### Visual Feedback

```rust
use luna::overlay::{OverlayManager, Color, Rectangle, Point};

let mut overlay = OverlayManager::default();

// Add highlights
let bounds = Rectangle::new(100.0, 100.0, 200.0, 50.0);
let color = Color::rgb(255, 0, 0);
overlay.add_highlight(bounds, color, Some("Important Button".to_string()));

// Add labels
let position = Point::new(50.0, 75.0);
overlay.add_label(position, "Click here".to_string(), Color::rgb(255, 255, 255));

// Add shapes
overlay.add_circle(Point::new(300.0, 200.0), 50.0, Color::rgba(0, 255, 0, 128));
overlay.add_arrow(Point::new(0.0, 0.0), Point::new(100.0, 100.0), Color::rgb(255, 255, 0));
```

### Animations

```rust
use luna::overlay::animations::{AnimationBuilder, EasingFunction, AnimationType};
use std::time::Duration;

let mut overlay = OverlayManager::default();

// Add element
let id = overlay.add_highlight(bounds, color, None);

// Create fade-in animation
let (animation, easing, auto_reverse, repeat_count) = AnimationBuilder::new(
    AnimationType::FadeIn,
    Duration::from_millis(500)
)
.with_easing(EasingFunction::EaseInOut)
.with_auto_reverse(true)
.build();

// Apply animation
overlay.add_fade_in_animation(&id);
```

### Colors and Styling

```rust
use luna::overlay::Color;

// Color creation
let red = Color::rgb(255, 0, 0);
let semi_transparent = Color::rgba(255, 0, 0, 128);
let with_alpha = red.with_alpha(200);

// Predefined colors for element types
let button_color = Color::rgb(0, 255, 0);     // Green
let textbox_color = Color::rgb(0, 0, 255);    // Blue
let window_color = Color::rgb(255, 165, 0);   // Orange
```

## Utility APIs

### Geometry

```rust
use luna::utils::geometry::{Point, Rectangle, Circle, Polygon};

// Points and distances
let p1 = Point::new(0.0, 0.0);
let p2 = Point::new(3.0, 4.0);
let distance = p1.distance_to(&p2); // 5.0

// Rectangles and intersections
let rect1 = Rectangle::new(0.0, 0.0, 100.0, 50.0);
let rect2 = Rectangle::new(50.0, 25.0, 100.0, 50.0);
let intersection = rect1.intersection(&rect2);

// Containment tests
let point = Point::new(25.0, 25.0);
let contains = rect1.contains_point(&point);
```

### Image Processing

```rust
use luna::utils::image_processing::{Image, sobel_edge_detection, threshold};

// Create and manipulate images
let mut image = Image::new(800, 600, 3); // RGB
let pixel = [255, 128, 0]; // Orange
image.set_pixel(100, 200, &pixel);

// Image operations
let grayscale = image.to_grayscale();
let resized = image.resize(400, 300);
let cropped = image.crop(&Rectangle::new(50.0, 50.0, 200.0, 150.0));

// Computer vision
let edges = sobel_edge_detection(&grayscale);
let binary = threshold(&edges, 128);
```

### Logging

```rust
use luna::utils::logging::{Logger, LogLevel, init_logger};

// Initialize logging
let logger = Logger::new()
    .with_level(LogLevel::Info)
    .with_file("luna.log")?
    .with_console(true);

init_logger(logger);

// Use logging macros
log_info!("Application started");
log_warn!("Warning message: {}", value);
log_error!("Error occurred: {}", error);
log_debug!("Debug info: {:?}", data);
```

### Configuration Management

```rust
use luna::utils::ConfigManager;

// Load/save configuration
let mut config = ConfigManager::new("config.txt")?;

// Set values
config.set("window_width".to_string(), "800".to_string());
config.set("enable_overlay".to_string(), "true".to_string());

// Get typed values
let width = config.get_int("window_width", 1024);
let overlay_enabled = config.get_bool("enable_overlay", true);

// Save to file
config.save_config()?;
```

## Error Handling

### Error Types

```rust
// Main error type
pub enum LunaError {
    InitializationError(String),
    ConfigurationError(String),
    VisionError(VisionError),
    InputError(InputError),
    OverlayError(String),
}

// Vision-specific errors
pub enum VisionError {
    ImageProcessingError(String),
    AnalysisError(String),
    CacheError(String),
}

// Input-specific errors
pub enum InputError {
    SafetyViolation,
    RateLimited,
    PlatformError(String),
    InvalidTarget,
    InvalidAction,
}
```

### Error Handling Patterns

```rust
use luna::{LunaError, VisionError};

// Pattern matching
match luna.analyze_screen(&image) {
    Ok(elements) => {
        // Process successful result
        println!("Found {} elements", elements.len());
    }
    Err(LunaError::VisionError(VisionError::ImageProcessingError(msg))) => {
        eprintln!("Image processing failed: {}", msg);
    }
    Err(LunaError::VisionError(VisionError::AnalysisError(msg))) => {
        eprintln!("Analysis failed: {}", msg);
    }
    Err(e) => {
        eprintln!("Unexpected error: {}", e);
    }
}

// Using Result helpers
let elements = luna.analyze_screen(&image)
    .map_err(|e| format!("Screen analysis failed: {}", e))?;

// Logging errors
if let Err(e) = luna.process_pending_actions() {
    log_error!("Failed to process actions: {}", e);
}
```

## Platform-Specific Features

### Windows

```rust
#[cfg(target_os = "windows")]
use luna::input::windows;

// Windows-specific input functions
let success = windows::send_key_combination("ctrl+c")?;
let window_handle = windows::get_foreground_window()?;
```

### Linux

```rust
#[cfg(target_os = "linux")]
use luna::vision::linux;

// X11/Wayland specific capture
let display_info = linux::get_display_info()?;
let screenshot = linux::capture_x11_screen()?;
```

### macOS

```rust
#[cfg(target_os = "macos")]
use luna::vision::macos;

// Core Graphics integration
let cg_image = macos::capture_screen_cg()?;
let accessibility_enabled = macos::check_accessibility_permissions()?;
```

## Performance Optimization

### Efficient Processing

```rust
use luna::utils::PerformanceMonitor;

let mut monitor = PerformanceMonitor::new();

// Measure performance
let result = monitor.measure("screen_analysis", || {
    luna.analyze_screen(&image)
})?;

// Get statistics
if let Some((avg, min, max)) = monitor.get_stats("screen_analysis") {
    println!("Screen analysis: {:.1}ms avg, {:.1}ms min, {:.1}ms max", 
             avg, min, max);
}
```

### Caching

```rust
use luna::utils::SimpleCache;

// Create cache for UI elements
let mut cache = SimpleCache::new(100, 60); // 100 items, 60 second TTL

// Cache analysis results
let cache_key = image_hash;
if let Some(cached_elements) = cache.get(&cache_key) {
    return Ok(cached_elements);
}

let elements = perform_analysis(&image)?;
cache.set(cache_key, elements.clone());
```

## Testing Support

### Test Utilities

```rust
#[cfg(test)]
use luna::test_utils;

// Create test data
let test_image = test_utils::create_test_image(800, 600);
let test_element = test_utils::create_test_ui_element();

// Verify test results
assert_eq!(test_image.width, 800);
assert_eq!(test_element.element_type, ElementType::Button);
```

### Mock Objects

```rust
use luna::vision::VisionPipeline;

// Create mock vision system for testing
struct MockVision;

impl MockVision {
    fn analyze_screen(&self, _image: &Image) -> Result<Vec<UIElement>, VisionError> {
        Ok(vec![test_utils::create_test_ui_element()])
    }
}
```

## Best Practices

### Resource Management

```rust
// Use RAII pattern for cleanup
{
    let mut luna = luna::init()?;
    // LUNA automatically cleans up when dropped
} // luna.shutdown() called automatically

// Handle errors gracefully
let elements = match luna.analyze_screen(&image) {
    Ok(elements) => elements,
    Err(e) => {
        log_warn!("Analysis failed: {}, using cached results", e);
        cached_elements
    }
};
```

### Performance Tips

1. **Reuse instances**: Create LUNA once and reuse
2. **Cache results**: Use built-in caching for repeated operations  
3. **Batch operations**: Process multiple actions together
4. **Configure appropriately**: Adjust capture FPS and quality settings
5. **Monitor performance**: Use built-in performance monitoring

### Safety Guidelines

1. **Always validate input**: Use safety checkers for all automation
2. **Rate limit actions**: Configure appropriate rate limiting
3. **Handle failures gracefully**: Provide fallback behavior
4. **Log security events**: Monitor for suspicious activity
5. **Use appropriate safety levels**: Configure based on use case

## Examples

See the `examples/` directory for complete working examples:

- `basic_usage.rs` - Simple screen analysis
- `advanced_detection.rs` - Custom UI detection
- `input_automation.rs` - Safe input handling
- `overlay_demo.rs` - Visual feedback system
- `performance_test.rs` - Performance benchmarking

---

For more detailed information, see the inline documentation generated with `cargo doc --open`.