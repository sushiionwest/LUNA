//! # LUNA
//!
//! A computer-use agent prototype: screen analysis through hand-written
//! computer vision and guarded input automation.
//!
//! Note: the CV pipeline and safety layers are implemented; the OS-level
//! screen capture and input injection are currently placeholder stubs.
//! See the README for an honest status overview.
//!
//! ## Quick Start
//!
//! ```rust
//! use luna::{Luna, LunaConfig};
//!
//! # fn main() -> anyhow::Result<()> {
//! let mut luna = Luna::new(LunaConfig::default())?;
//! let analysis = luna.analyze_current_screen()?;
//! println!("Found {} UI elements", analysis.elements.len());
//! # Ok(())
//! # }
//! ```
//!
//! ## Architecture
//!
//! - [`core`] - Coordinator: command -> capture -> analyze -> validate -> act
//! - [`ai`] - Screen analysis and rule-based action planning
//! - [`vision`] - Screen capture and UI element detection
//! - [`input`] - Input actions with safety checks and rate limiting
//! - [`overlay`] - Visual feedback data structures
//! - [`utils`] - Geometry, image processing, logging

pub mod ai;
pub mod core;
pub mod input;
pub mod utils;
pub mod vision;
pub mod overlay;

// Re-export main types for convenient access
pub use core::{Luna, LunaConfig, LunaError};
pub use vision::{UIElement, ElementType, VisionError};
pub use input::{InputAction, ActionType, InputError};
pub use overlay::{OverlayManager, OverlayConfig, Color};
pub use utils::geometry::{Point, Rectangle};

// Re-export commonly used functions
pub use vision::{quick_analyze, find_buttons, find_text_boxes};
pub use vision::screen_capture::{quick_screenshot, screenshot_region};
pub use overlay::{create_ui_highlights, create_simple_highlight};

/// Library version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NAME: &str = env!("CARGO_PKG_NAME");

/// Initialize LUNA with default configuration
///
/// # Example
///
/// ```rust
/// # fn main() -> anyhow::Result<()> {
/// let mut luna = luna::init()?;
/// let analysis = luna.analyze_current_screen()?;
/// # Ok(())
/// # }
/// ```
pub fn init() -> anyhow::Result<Luna> {
    Luna::new(LunaConfig::default())
}

/// Initialize LUNA with custom configuration
///
/// # Example
///
/// ```rust
/// use luna::{LunaConfig, init_with_config};
///
/// # fn main() -> anyhow::Result<()> {
/// let mut config = LunaConfig::default();
/// config.safety.enabled = true;
///
/// let mut luna = init_with_config(config)?;
/// # Ok(())
/// # }
/// ```
pub fn init_with_config(config: LunaConfig) -> anyhow::Result<Luna> {
    Luna::new(config)
}

/// Quick screen analysis without creating a full LUNA instance
///
/// This function performs a one-shot screen capture and analysis,
/// useful for simple use cases that don't need the full application state.
///
/// # Example
///
/// ```rust
/// use luna;
///
/// let elements = luna::analyze_current_screen()?;
/// for element in elements {
///     println!("Found {}: {:.1}% confidence", 
///              element.element_type, 
///              element.confidence * 100.0);
/// }
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn analyze_current_screen() -> Result<Vec<UIElement>, VisionError> {
    let image = quick_screenshot()
        .map_err(|e| VisionError::ImageProcessingError(e.to_string()))?;
    quick_analyze(&image)
}

/// Find all buttons on the current screen
///
/// Convenience function for button detection.
///
/// # Example
///
/// ```rust
/// use luna;
///
/// let buttons = luna::find_buttons_on_screen()?;
/// println!("Found {} buttons", buttons.len());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn find_buttons_on_screen() -> Result<Vec<UIElement>, VisionError> {
    let image = quick_screenshot()
        .map_err(|e| VisionError::ImageProcessingError(e.to_string()))?;
    find_buttons(&image)
}

/// Find all text boxes on the current screen
///
/// Convenience function for text box detection.
///
/// # Example
///
/// ```rust
/// use luna;
///
/// let text_boxes = luna::find_text_boxes_on_screen()?;
/// for text_box in text_boxes {
///     println!("Text box at ({:.0}, {:.0})", 
///              text_box.bounds.x, 
///              text_box.bounds.y);
/// }
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn find_text_boxes_on_screen() -> Result<Vec<UIElement>, VisionError> {
    let image = quick_screenshot()
        .map_err(|e| VisionError::ImageProcessingError(e.to_string()))?;
    find_text_boxes(&image)
}

/// Get library information
pub fn info() -> LibraryInfo {
    LibraryInfo {
        name: NAME.to_string(),
        version: VERSION.to_string(),
        features: get_enabled_features(),
        platform: get_platform_info(),
    }
}

/// Library information structure
#[derive(Debug, Clone)]
pub struct LibraryInfo {
    pub name: String,
    pub version: String,
    pub features: Vec<String>,
    pub platform: PlatformInfo,
}

/// Platform information structure
#[derive(Debug, Clone)]
pub struct PlatformInfo {
    pub os: String,
    pub arch: String,
    pub supports_overlay: bool,
    pub supports_input: bool,
}

fn get_enabled_features() -> Vec<String> {
    let mut features = vec![
        "computer-vision".to_string(),
        "safety-system".to_string(),
        "overlay-system".to_string(),
        "screen-capture".to_string(),
    ];

    #[cfg(target_os = "windows")]
    features.push("windows-input".to_string());

    #[cfg(target_os = "linux")]
    features.push("linux-input".to_string());

    #[cfg(target_os = "macos")]
    features.push("macos-input".to_string());

    features
}

fn get_platform_info() -> PlatformInfo {
    PlatformInfo {
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        supports_overlay: true, // Our overlay system is cross-platform
        supports_input: cfg!(any(target_os = "windows", target_os = "linux", target_os = "macos")),
    }
}

// Integration tests helper functions (only available in test builds)
#[cfg(test)]
pub mod test_utils {
    use super::*;
    use utils::image_processing::Image;

    /// Create a test image with some basic patterns
    pub fn create_test_image(width: usize, height: usize) -> Image {
        let mut image = Image::new(width, height, 3);
        
        // Add some test patterns
        for y in 0..height {
            for x in 0..width {
                let r = ((x as f64 / width as f64) * 255.0) as u8;
                let g = ((y as f64 / height as f64) * 255.0) as u8;
                let b = 128;
                image.set_pixel(x, y, &[r, g, b]);
            }
        }
        
        image
    }

    /// Create a test UI element
    pub fn create_test_ui_element() -> UIElement {
        use std::collections::HashMap;
        
        UIElement {
            bounds: Rectangle::new(10.0, 10.0, 100.0, 50.0),
            element_type: ElementType::Button,
            confidence: 0.8,
            properties: HashMap::new(),
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_library_info() {
        let info = info();
        assert_eq!(info.name, NAME);
        assert_eq!(info.version, VERSION);
        assert!(!info.features.is_empty());
    }

    #[test]
    fn test_platform_info() {
        let platform = get_platform_info();
        assert!(!platform.os.is_empty());
        assert!(!platform.arch.is_empty());
        assert!(platform.supports_overlay); // Should always be true for our implementation
    }

    #[test]
    fn test_quick_functions() {
        // These functions should not panic even if they return errors
        let _ = analyze_current_screen();
        let _ = find_buttons_on_screen();
        let _ = find_text_boxes_on_screen();
    }

    #[test]
    fn test_init_functions() {
        // Test that init functions can be called without panicking
        let result1 = init();
        let result2 = init_with_config(LunaConfig::default());
        
        // We don't require these to succeed in test environment,
        // just that they don't panic
        match (result1, result2) {
            (Ok(_), Ok(_)) => println!("Both init functions succeeded"),
            _ => println!("Init functions returned errors (expected in test environment)"),
        }
    }

    #[test]
    fn test_test_utils() {
        let image = test_utils::create_test_image(100, 50);
        assert_eq!(image.width, 100);
        assert_eq!(image.height, 50);
        assert_eq!(image.channels, 3);

        let element = test_utils::create_test_ui_element();
        assert_eq!(element.element_type, ElementType::Button);
        assert_eq!(element.confidence, 0.8);
    }
}