/*!
 * Visual Feedback Overlay - Shows Luna's actions before execution
 * 
 * Displays highlights, countdown timers, and action previews
 */

use anyhow::Result;
use image::DynamicImage;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::core::LunaAction;

/// Visual overlay system for showing Luna's intended actions
pub struct VisualOverlay {
    /// Whether overlay is currently visible
    visible: bool,
    /// Current overlay elements
    elements: Vec<OverlayElement>,
    /// Overlay window handle (simplified for portable app)
    window_info: Option<OverlayWindow>,
    /// Configuration
    config: OverlayConfig,
    /// Animation state
    animation_state: AnimationState,
}

/// Configuration for the visual overlay
#[derive(Debug, Clone)]
pub struct OverlayConfig {
    pub show_highlights: bool,
    pub show_countdown: bool,
    pub show_action_preview: bool,
    pub highlight_color: [u8; 4], // RGBA
    pub countdown_color: [u8; 4],
    pub overlay_opacity: f32,
    pub countdown_duration_ms: u64,
    pub highlight_thickness: u32,
}

impl Default for OverlayConfig {
    fn default() -> Self {
        Self {
            show_highlights: true,
            show_countdown: true,
            show_action_preview: true,
            highlight_color: [100, 149, 237, 200], // Semi-transparent cornflower blue
            countdown_color: [255, 100, 100, 255], // Red countdown
            overlay_opacity: 0.8,
            countdown_duration_ms: 3000, // 3 seconds
            highlight_thickness: 3,
        }
    }
}

/// Animation state for smooth transitions
#[derive(Debug, Clone)]
struct AnimationState {
    start_time: Option<Instant>,
    duration_ms: u64,
    current_frame: u32,
    total_frames: u32,
}

impl Default for AnimationState {
    fn default() -> Self {
        Self {
            start_time: None,
            duration_ms: 3000,
            current_frame: 0,
            total_frames: 60, // 60 frames for 3 seconds (20 FPS)
        }
    }
}

/// Overlay window information
#[derive(Debug, Clone)]
struct OverlayWindow {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    created: bool,
}

/// Element to display in overlay
#[derive(Debug, Clone)]
pub struct OverlayElement {
    pub element_type: OverlayElementType,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub text: Option<String>,
    pub color: [u8; 4],
    pub visible: bool,
    pub animation_progress: f32, // 0.0 to 1.0
}

/// Types of overlay elements
#[derive(Debug, Clone)]
pub enum OverlayElementType {
    Highlight,      // Rectangle highlight around target
    Circle,         // Circle highlight for click point
    Arrow,          // Arrow pointing to target
    Text,           // Text label
    Countdown,      // Countdown timer
    Preview,        // Action preview
}

impl VisualOverlay {
    /// Create new visual overlay
    pub fn new() -> Result<Self> {
        debug!("Initializing visual overlay system");
        
        Ok(Self {
            visible: false,
            elements: Vec::new(),
            window_info: None,
            config: OverlayConfig::default(),
            animation_state: AnimationState::default(),
        })
    }
    
    /// Show action preview with countdown
    pub async fn show_action_preview(&mut self, actions: &[LunaAction], screenshot: &DynamicImage) -> Result<()> {
        info!("Showing action preview for {} actions", actions.len());
        
        // Clear existing elements
        self.elements.clear();
        
        // Add highlight elements for each action
        for (i, action) in actions.iter().enumerate() {
            self.add_action_highlight(action, i).await?;
        }
        
        // Add countdown timer
        if self.config.show_countdown {
            self.add_countdown_timer().await?;
        }
        
        // Make overlay visible
        self.visible = true;
        
        // Start animation
        self.start_animation().await;
        
        debug!("Action preview displayed with {} elements", self.elements.len());
        Ok(())
    }
    
    /// Add highlight for a specific action
    async fn add_action_highlight(&mut self, action: &LunaAction, index: usize) -> Result<()> {
        match action {
            LunaAction::Click { x, y } | 
            LunaAction::RightClick { x, y } | 
            LunaAction::DoubleClick { x, y } => {
                // Add circle highlight for click actions
                self.elements.push(OverlayElement {
                    element_type: OverlayElementType::Circle,
                    x: *x - 15,
                    y: *y - 15,
                    width: 30,
                    height: 30,
                    text: Some(format!("{}", index + 1)),
                    color: self.config.highlight_color,
                    visible: true,
                    animation_progress: 0.0,
                });
                
                // Add action type label
                let action_text = match action {
                    LunaAction::Click { .. } => "Click",
                    LunaAction::RightClick { .. } => "Right Click",
                    LunaAction::DoubleClick { .. } => "Double Click",
                    _ => "Action",
                };
                
                self.elements.push(OverlayElement {
                    element_type: OverlayElementType::Text,
                    x: *x - 25,
                    y: *y - 40,
                    width: 50,
                    height: 20,
                    text: Some(action_text.to_string()),
                    color: [255, 255, 255, 255], // White text
                    visible: true,
                    animation_progress: 0.0,
                });
            }
            
            LunaAction::Drag { from_x, from_y, to_x, to_y } => {
                // Add arrow from start to end point
                self.elements.push(OverlayElement {
                    element_type: OverlayElementType::Arrow,
                    x: *from_x,
                    y: *from_y,
                    width: *to_x - *from_x,
                    height: *to_y - *from_y,
                    text: Some("Drag".to_string()),
                    color: [255, 165, 0, 200], // Orange for drag
                    visible: true,
                    animation_progress: 0.0,
                });
                
                // Highlight start point
                self.elements.push(OverlayElement {
                    element_type: OverlayElementType::Circle,
                    x: *from_x - 10,
                    y: *from_y - 10,
                    width: 20,
                    height: 20,
                    text: None,
                    color: [0, 255, 0, 200], // Green for start
                    visible: true,
                    animation_progress: 0.0,
                });
                
                // Highlight end point
                self.elements.push(OverlayElement {
                    element_type: OverlayElementType::Circle,
                    x: *to_x - 10,
                    y: *to_y - 10,
                    width: 20,
                    height: 20,
                    text: None,
                    color: [255, 0, 0, 200], // Red for end
                    visible: true,
                    animation_progress: 0.0,
                });
            }
            
            LunaAction::Type { text } => {
                // Show text preview (position at center of screen for now)
                self.elements.push(OverlayElement {
                    element_type: OverlayElementType::Text,
                    x: 400, // Center-ish
                    y: 300,
                    width: 200,
                    height: 30,
                    text: Some(format!("Type: \"{}\"", text)),
                    color: [100, 255, 100, 255], // Light green for typing
                    visible: true,
                    animation_progress: 0.0,
                });
            }
            
            LunaAction::KeyCombo { keys } => {
                // Show key combination
                self.elements.push(OverlayElement {
                    element_type: OverlayElementType::Text,
                    x: 400,
                    y: 250,
                    width: 200,
                    height: 30,
                    text: Some(format!("Keys: {}", keys.join(" + "))),
                    color: [255, 255, 100, 255], // Yellow for key combos
                    visible: true,
                    animation_progress: 0.0,
                });
            }
            
            LunaAction::Scroll { x, y, direction } => {
                // Show scroll indicator
                let direction_text = match direction {
                    crate::core::ScrollDirection::Up => "↑ Scroll Up",
                    crate::core::ScrollDirection::Down => "↓ Scroll Down", 
                    crate::core::ScrollDirection::Left => "← Scroll Left",
                    crate::core::ScrollDirection::Right => "→ Scroll Right",
                };
                
                self.elements.push(OverlayElement {
                    element_type: OverlayElementType::Text,
                    x: *x - 50,
                    y: *y - 20,
                    width: 100,
                    height: 40,
                    text: Some(direction_text.to_string()),
                    color: [200, 100, 255, 255], // Purple for scroll
                    visible: true,
                    animation_progress: 0.0,
                });
            }
            
            LunaAction::Wait { milliseconds } => {
                // Show wait indicator
                self.elements.push(OverlayElement {
                    element_type: OverlayElementType::Text,
                    x: 400,
                    y: 350,
                    width: 200,
                    height: 30,
                    text: Some(format!("Wait {}ms", milliseconds)),
                    color: [150, 150, 150, 255], // Gray for wait
                    visible: true,
                    animation_progress: 0.0,
                });
            }
            
            _ => {
                // Generic action highlight
                self.elements.push(OverlayElement {
                    element_type: OverlayElementType::Text,
                    x: 400,
                    y: 200,
                    width: 200,
                    height: 30,
                    text: Some("Unknown Action".to_string()),
                    color: [255, 255, 255, 255],
                    visible: true,
                    animation_progress: 0.0,
                });
            }
        }
        
        Ok(())
    }
    
    /// Add countdown timer element
    async fn add_countdown_timer(&mut self) -> Result<()> {
        // Add countdown in top-center of screen
        self.elements.push(OverlayElement {
            element_type: OverlayElementType::Countdown,
            x: 350, // Approximate center
            y: 50,  // Top of screen
            width: 100,
            height: 50,
            text: Some("3".to_string()), // Start at 3 seconds
            color: self.config.countdown_color,
            visible: true,
            animation_progress: 0.0,
        });
        
        // Add instructional text
        self.elements.push(OverlayElement {
            element_type: OverlayElementType::Text,
            x: 250,
            y: 110,
            width: 300,
            height: 30,
            text: Some("Press ESC to cancel".to_string()),
            color: [255, 255, 255, 180], // Semi-transparent white
            visible: true,
            animation_progress: 0.0,
        });
        
        Ok(())
    }
    
    /// Start animation sequence
    async fn start_animation(&mut self) {
        self.animation_state.start_time = Some(Instant::now());
        self.animation_state.current_frame = 0;
        
        debug!("Starting overlay animation for {} ms", self.animation_state.duration_ms);
        
        // In a real implementation, this would start a timer/animation loop
        // For the portable app, we'll simulate the countdown
        tokio::spawn(async move {
            for i in (1..=3).rev() {
                tokio::time::sleep(Duration::from_secs(1)).await;
                debug!("Countdown: {}", i);
            }
            debug!("Countdown complete, executing actions");
        });
    }
    
    /// Update animation frame
    pub async fn update_animation(&mut self) -> bool {
        if let Some(start_time) = self.animation_state.start_time {
            let elapsed = start_time.elapsed();
            let elapsed_ms = elapsed.as_millis() as u64;
            
            if elapsed_ms >= self.animation_state.duration_ms {
                // Animation complete
                self.animation_state.start_time = None;
                return false;
            }
            
            // Update animation progress
            let progress = elapsed_ms as f32 / self.animation_state.duration_ms as f32;
            
            // Update countdown text
            let remaining_seconds = ((self.animation_state.duration_ms - elapsed_ms) / 1000) + 1;
            
            for element in &mut self.elements {
                element.animation_progress = progress;
                
                if matches!(element.element_type, OverlayElementType::Countdown) {
                    element.text = Some(remaining_seconds.to_string());
                }
            }
            
            self.animation_state.current_frame += 1;
            return true;
        }
        
        false
    }
    
    /// Hide the overlay
    pub async fn hide(&mut self) {
        info!("Hiding visual overlay");
        self.visible = false;
        self.elements.clear();
        self.animation_state.start_time = None;
        
        // In a real implementation, would hide the overlay window
        debug!("Overlay hidden");
    }
    
    /// Check if overlay is visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }
    
    /// Get current overlay elements
    pub fn get_elements(&self) -> &[OverlayElement] {
        &self.elements
    }
    
    /// Set overlay configuration
    pub fn set_config(&mut self, config: OverlayConfig) {
        self.config = config;
        debug!("Overlay configuration updated");
    }
    
    /// Get overlay statistics
    pub fn get_stats(&self) -> OverlayStats {
        OverlayStats {
            visible: self.visible,
            element_count: self.elements.len(),
            animation_active: self.animation_state.start_time.is_some(),
            animation_progress: self.animation_state.current_frame as f32 / self.animation_state.total_frames as f32,
        }
    }
    
    /// Force hide overlay (emergency)
    pub async fn force_hide(&mut self) {
        warn!("Force hiding overlay");
        self.hide().await;
    }
    
    /// Create overlay for element detection preview
    pub async fn show_detection_preview(&mut self, elements: &[crate::core::ScreenElement]) -> Result<()> {
        debug!("Showing detection preview for {} elements", elements.len());
        
        self.elements.clear();
        
        for (i, element) in elements.iter().enumerate() {
            // Add highlight rectangle
            self.elements.push(OverlayElement {
                element_type: OverlayElementType::Highlight,
                x: element.bounds.x,
                y: element.bounds.y,
                width: element.bounds.width,
                height: element.bounds.height,
                text: element.text.clone(),
                color: if element.clickable {
                    [0, 255, 0, 150] // Green for clickable
                } else {
                    [255, 255, 0, 150] // Yellow for non-clickable
                },
                visible: true,
                animation_progress: 0.0,
            });
            
            // Add element type label
            if i < 10 { // Limit labels to avoid clutter
                self.elements.push(OverlayElement {
                    element_type: OverlayElementType::Text,
                    x: element.bounds.x,
                    y: element.bounds.y - 20,
                    width: 100,
                    height: 20,
                    text: Some(format!("{}: {:.1}%", element.element_type, element.confidence * 100.0)),
                    color: [255, 255, 255, 255],
                    visible: true,
                    animation_progress: 0.0,
                });
            }
        }
        
        self.visible = true;
        
        // Auto-hide after 5 seconds
        let mut overlay_clone = self.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(5)).await;
            overlay_clone.hide().await;
        });
        
        Ok(())
    }
}

// Implement Clone for the overlay (needed for spawning tasks)
impl Clone for VisualOverlay {
    fn clone(&self) -> Self {
        Self {
            visible: self.visible,
            elements: self.elements.clone(),
            window_info: self.window_info.clone(),
            config: self.config.clone(),
            animation_state: self.animation_state.clone(),
        }
    }
}

/// Overlay statistics
#[derive(Debug, Clone)]
pub struct OverlayStats {
    pub visible: bool,
    pub element_count: usize,
    pub animation_active: bool,
    pub animation_progress: f32,
}

impl OverlayStats {
    pub fn is_active(&self) -> bool {
        self.visible || self.animation_active
    }
}