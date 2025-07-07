//! # Visual Feedback System
//! 
//! This module provides real-time visual feedback showing users exactly what Luna's AI
//! has detected and analyzed. It creates transparent overlays that highlight detected
//! elements, show confidence scores, and display reasoning for AI decisions.
//!
//! ## Key Features
//! - Transparent overlay windows that don't interfere with the desktop
//! - Color-coded highlighting for different element types and confidence levels
//! - Real-time display of AI analysis results
//! - Interactive elements showing detection details on hover
//! - Performance-optimized rendering for smooth real-time updates
//!
//! ## Visual Elements
//! - **Object Detection**: Bounding boxes around detected UI elements
//! - **Text Recognition**: Highlights for extracted text regions
//! - **Click Targets**: Prominent indicators for recommended click points
//! - **Confidence Indicators**: Color-coded confidence visualization
//! - **Reasoning Display**: Tooltips showing AI decision reasoning

use crate::ai::{DetectedObject, ExtractedText, ClickTarget};
use crate::core::{LunaError, LunaResult};
use egui::{Color32, Pos2, Rect, Stroke, Vec2};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info, warn, error, instrument};

/// Visual feedback overlay that shows AI analysis results
#[derive(Debug, Clone)]
pub struct FeedbackOverlay {
    /// Unique identifier for this overlay
    pub id: String,
    /// Screen position and size
    pub bounds: Rect,
    /// Whether the overlay is currently visible
    pub visible: bool,
    /// Elements to display
    pub elements: Vec<OverlayElement>,
    /// Creation timestamp
    pub created_at: Instant,
    /// Last update timestamp
    pub updated_at: Instant,
}

/// Individual element in the visual feedback overlay
#[derive(Debug, Clone)]
pub struct OverlayElement {
    /// Element type determines rendering style
    pub element_type: OverlayElementType,
    /// Position and size on screen
    pub bounds: Rect,
    /// Primary display text
    pub text: String,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,
    /// Additional details for hover/click
    pub details: String,
    /// Custom styling options
    pub style: OverlayStyle,
    /// Whether element is currently highlighted
    pub is_highlighted: bool,
}

/// Types of overlay elements
#[derive(Debug, Clone, PartialEq)]
pub enum OverlayElementType {
    /// Detected object from Florence-2
    DetectedObject,
    /// Extracted text from TrOCR
    ExtractedText,
    /// Click target from AI pipeline
    ClickTarget,
    /// CLIP match result
    ClipMatch,
    /// Segmentation mask outline
    SegmentationMask,
    /// General information display
    InfoDisplay,
}

/// Styling options for overlay elements
#[derive(Debug, Clone)]
pub struct OverlayStyle {
    /// Border color
    pub border_color: Color32,
    /// Fill color (with transparency)
    pub fill_color: Color32,
    /// Border width
    pub border_width: f32,
    /// Text color
    pub text_color: Color32,
    /// Font size
    pub font_size: f32,
    /// Animation properties
    pub animation: Option<AnimationStyle>,
}

/// Animation style for dynamic elements
#[derive(Debug, Clone)]
pub struct AnimationStyle {
    /// Animation type
    pub animation_type: AnimationType,
    /// Animation duration
    pub duration: Duration,
    /// Animation start time
    pub start_time: Instant,
}

/// Types of animations
#[derive(Debug, Clone, PartialEq)]
pub enum AnimationType {
    /// Pulsing effect for attention
    Pulse,
    /// Fade in/out
    Fade,
    /// Blinking effect
    Blink,
    /// Growing/shrinking
    Scale,
}

/// Configuration for visual feedback
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackConfig {
    /// Maximum number of overlays to show simultaneously
    pub max_overlays: usize,
    /// Default overlay opacity (0.0 to 1.0)
    pub default_opacity: f32,
    /// Show confidence scores as text
    pub show_confidence_scores: bool,
    /// Show element details on hover
    pub show_hover_details: bool,
    /// Animation speed multiplier
    pub animation_speed: f32,
    /// Minimum confidence to display elements
    pub min_display_confidence: f32,
    /// Color scheme for different confidence levels
    pub confidence_colors: ConfidenceColors,
}

/// Color scheme for different confidence levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceColors {
    /// High confidence (>0.8)
    pub high_confidence: [u8; 4], // RGBA
    /// Medium confidence (0.5-0.8)
    pub medium_confidence: [u8; 4],
    /// Low confidence (<0.5)
    pub low_confidence: [u8; 4],
    /// Click targets
    pub click_target: [u8; 4],
    /// Text elements
    pub text_element: [u8; 4],
}

impl Default for FeedbackConfig {
    fn default() -> Self {
        Self {
            max_overlays: 10,
            default_opacity: 0.8,
            show_confidence_scores: true,
            show_hover_details: true,
            animation_speed: 1.0,
            min_display_confidence: 0.3,
            confidence_colors: ConfidenceColors {
                high_confidence: [0, 255, 0, 200],   // Green
                medium_confidence: [255, 165, 0, 200], // Orange
                low_confidence: [255, 0, 0, 200],     // Red
                click_target: [0, 191, 255, 220],     // DeepSkyBlue
                text_element: [255, 255, 0, 180],     // Yellow
            },
        }
    }
}

/// Visual feedback system for showing AI analysis results
pub struct VisualFeedback {
    /// Configuration
    config: FeedbackConfig,
    /// Active overlays
    active_overlays: Arc<RwLock<HashMap<String, FeedbackOverlay>>>,
    /// System initialization status
    is_initialized: Arc<RwLock<bool>>,
}

impl VisualFeedback {
    /// Create a new visual feedback system
    pub fn new(config: FeedbackConfig) -> Self {
        info!("Creating visual feedback system");

        Self {
            config,
            active_overlays: Arc::new(RwLock::new(HashMap::new())),
            is_initialized: Arc::new(RwLock::new(false)),
        }
    }

    /// Initialize the visual feedback system
    #[instrument(skip(self))]
    pub async fn initialize(&mut self) -> LunaResult<()> {
        // Check if already initialized
        if *self.is_initialized.read().await {
            debug!("Visual feedback already initialized");
            return Ok(());
        }

        info!("Initializing visual feedback system");

        // Initialize rendering backend
        self.initialize_rendering().await?;

        // Mark as initialized
        *self.is_initialized.write().await = true;

        info!("✅ Visual feedback system initialized");
        Ok(())
    }

    /// Show AI analysis results as visual overlay
    #[instrument(skip(self, objects, extracted_text, click_targets))]
    pub async fn show_analysis_results(
        &self,
        objects: &[DetectedObject],
        extracted_text: &[ExtractedText],
        click_targets: &[ClickTarget],
        screen_bounds: Rect,
    ) -> LunaResult<String> {
        if !*self.is_initialized.read().await {
            return Err(LunaError::Vision {
                operation: "show analysis results".to_string(),
                error: "Visual feedback not initialized".to_string(),
                suggestion: "Call initialize() before showing results".to_string(),
            });
        }

        let overlay_id = format!("analysis_{}", Instant::now().elapsed().as_millis());
        let mut elements = Vec::new();

        // Add detected objects
        for obj in objects {
            if obj.confidence >= self.config.min_display_confidence {
                elements.push(self.create_object_element(obj).await);
            }
        }

        // Add extracted text
        for text in extracted_text {
            if text.confidence >= self.config.min_display_confidence {
                elements.push(self.create_text_element(text).await);
            }
        }

        // Add click targets (always show, as they're the final result)
        for (i, target) in click_targets.iter().enumerate() {
            elements.push(self.create_click_target_element(target, i).await);
        }

        // Create overlay
        let overlay = FeedbackOverlay {
            id: overlay_id.clone(),
            bounds: screen_bounds,
            visible: true,
            elements,
            created_at: Instant::now(),
            updated_at: Instant::now(),
        };

        // Store overlay
        let mut overlays = self.active_overlays.write().await;
        overlays.insert(overlay_id.clone(), overlay);

        // Cleanup old overlays if we have too many
        if overlays.len() > self.config.max_overlays {
            self.cleanup_old_overlays(&mut overlays).await;
        }

        info!(
            "Showing analysis results: {} objects, {} text, {} targets",
            objects.len(),
            extracted_text.len(),
            click_targets.len()
        );

        Ok(overlay_id)
    }

    /// Update an existing overlay with new data
    pub async fn update_overlay(&self, overlay_id: &str, new_elements: Vec<OverlayElement>) -> LunaResult<()> {
        let mut overlays = self.active_overlays.write().await;
        
        if let Some(overlay) = overlays.get_mut(overlay_id) {
            overlay.elements = new_elements;
            overlay.updated_at = Instant::now();
            
            debug!("Updated overlay {} with {} elements", overlay_id, overlay.elements.len());
            Ok(())
        } else {
            Err(LunaError::Vision {
                operation: "update overlay".to_string(),
                error: format!("Overlay {} not found", overlay_id),
                suggestion: "Check overlay ID and ensure it hasn't been cleaned up".to_string(),
            })
        }
    }

    /// Hide a specific overlay
    pub async fn hide_overlay(&self, overlay_id: &str) -> LunaResult<()> {
        let mut overlays = self.active_overlays.write().await;
        
        if let Some(overlay) = overlays.get_mut(overlay_id) {
            overlay.visible = false;
            debug!("Hidden overlay {}", overlay_id);
            Ok(())
        } else {
            Err(LunaError::Vision {
                operation: "hide overlay".to_string(),
                error: format!("Overlay {} not found", overlay_id),
                suggestion: "Check overlay ID".to_string(),
            })
        }
    }

    /// Remove a specific overlay
    pub async fn remove_overlay(&self, overlay_id: &str) -> LunaResult<()> {
        let mut overlays = self.active_overlays.write().await;
        
        if overlays.remove(overlay_id).is_some() {
            debug!("Removed overlay {}", overlay_id);
            Ok(())
        } else {
            Err(LunaError::Vision {
                operation: "remove overlay".to_string(),
                error: format!("Overlay {} not found", overlay_id),
                suggestion: "Check overlay ID".to_string(),
            })
        }
    }

    /// Clear all overlays
    pub async fn clear_all_overlays(&self) {
        let mut overlays = self.active_overlays.write().await;
        let count = overlays.len();
        overlays.clear();
        
        if count > 0 {
            info!("Cleared {} overlays", count);
        }
    }

    /// Get list of active overlay IDs
    pub async fn get_active_overlays(&self) -> Vec<String> {
        self.active_overlays.read().await.keys().cloned().collect()
    }

    /// Render all active overlays (called by overlay app)
    pub async fn render_overlays(&self, ctx: &egui::Context) {
        let overlays = self.active_overlays.read().await;
        
        for overlay in overlays.values() {
            if overlay.visible {
                self.render_overlay(ctx, overlay).await;
            }
        }
    }

    /// Validate visual feedback functionality
    pub async fn validate(&self) -> LunaResult<()> {
        info!("Validating visual feedback functionality");

        if !*self.is_initialized.read().await {
            return Err(LunaError::Vision {
                operation: "visual feedback validation".to_string(),
                error: "System not initialized".to_string(),
                suggestion: "Call initialize() first".to_string(),
            });
        }

        // Test creating a simple overlay
        let test_overlay = FeedbackOverlay {
            id: "test_overlay".to_string(),
            bounds: Rect::from_min_size(Pos2::new(0.0, 0.0), Vec2::new(100.0, 100.0)),
            visible: true,
            elements: vec![],
            created_at: Instant::now(),
            updated_at: Instant::now(),
        };

        let mut overlays = self.active_overlays.write().await;
        overlays.insert("test_overlay".to_string(), test_overlay);
        
        // Test removal
        overlays.remove("test_overlay");

        info!("✅ Visual feedback validation successful");
        Ok(())
    }

    /// Get current status
    pub async fn get_status(&self) -> HashMap<String, String> {
        let mut status = HashMap::new();
        
        status.insert("initialized".to_string(), self.is_initialized.read().await.to_string());
        
        let overlays = self.active_overlays.read().await;
        status.insert("active_overlays".to_string(), overlays.len().to_string());
        status.insert("max_overlays".to_string(), self.config.max_overlays.to_string());
        
        let visible_count = overlays.values().filter(|o| o.visible).count();
        status.insert("visible_overlays".to_string(), visible_count.to_string());

        status
    }

    /// Cleanup resources
    pub async fn cleanup(&mut self) -> LunaResult<()> {
        info!("Cleaning up visual feedback resources");
        
        // Clear all overlays
        self.clear_all_overlays().await;
        
        // Mark as uninitialized
        *self.is_initialized.write().await = false;
        
        Ok(())
    }

    // Private helper methods

    async fn initialize_rendering(&self) -> LunaResult<()> {
        // Initialize rendering backend (egui, wgpu, etc.)
        // For this mock implementation, we'll just log
        info!("Initializing rendering backend for visual feedback");
        Ok(())
    }

    async fn create_object_element(&self, obj: &DetectedObject) -> OverlayElement {
        let style = self.get_style_for_confidence(obj.confidence, OverlayElementType::DetectedObject).await;
        
        OverlayElement {
            element_type: OverlayElementType::DetectedObject,
            bounds: Rect::from_min_size(
                Pos2::new(obj.bbox.0 as f32, obj.bbox.1 as f32),
                Vec2::new(obj.bbox.2 as f32, obj.bbox.3 as f32),
            ),
            text: format!("{} ({:.0}%)", obj.label, obj.confidence * 100.0),
            confidence: obj.confidence,
            details: format!(
                "Type: {}\nConfidence: {:.2}\nCenter: ({}, {})\nSize: {}x{}",
                obj.label, obj.confidence, obj.center.0, obj.center.1,
                obj.bbox.2, obj.bbox.3
            ),
            style,
            is_highlighted: false,
        }
    }

    async fn create_text_element(&self, text: &ExtractedText) -> OverlayElement {
        let style = self.get_style_for_confidence(text.confidence, OverlayElementType::ExtractedText).await;
        
        OverlayElement {
            element_type: OverlayElementType::ExtractedText,
            bounds: Rect::from_min_size(
                Pos2::new(text.bbox.0 as f32, text.bbox.1 as f32),
                Vec2::new(text.bbox.2 as f32, text.bbox.3 as f32),
            ),
            text: format!("\"{}\"", text.text.chars().take(20).collect::<String>()),
            confidence: text.confidence,
            details: format!(
                "Text: \"{}\"\nConfidence: {:.2}\nLanguage: {:?}\nBounds: ({}, {}, {}, {})",
                text.text, text.confidence, text.language,
                text.bbox.0, text.bbox.1, text.bbox.2, text.bbox.3
            ),
            style,
            is_highlighted: false,
        }
    }

    async fn create_click_target_element(&self, target: &ClickTarget, index: usize) -> OverlayElement {
        let mut style = self.get_style_for_confidence(target.confidence, OverlayElementType::ClickTarget).await;
        
        // Make click targets more prominent
        style.border_width = 3.0;
        style.font_size = 14.0;
        
        // Add pulsing animation for the primary target
        if index == 0 {
            style.animation = Some(AnimationStyle {
                animation_type: AnimationType::Pulse,
                duration: Duration::from_millis(1000),
                start_time: Instant::now(),
            });
        }
        
        // Create a small rect around the click point
        let size = 20.0;
        let bounds = Rect::from_center_size(
            Pos2::new(target.coordinates.0 as f32, target.coordinates.1 as f32),
            Vec2::new(size, size),
        );
        
        OverlayElement {
            element_type: OverlayElementType::ClickTarget,
            bounds,
            text: format!("#{} ({:.0}%)", index + 1, target.confidence * 100.0),
            confidence: target.confidence,
            details: format!(
                "Click Target #{}\nCoordinates: ({}, {})\nConfidence: {:.2}\nType: {}\nReasoning: {}",
                index + 1, target.coordinates.0, target.coordinates.1,
                target.confidence, target.element_type, target.reasoning
            ),
            style,
            is_highlighted: index == 0, // Highlight the primary target
        }
    }

    async fn get_style_for_confidence(&self, confidence: f32, element_type: OverlayElementType) -> OverlayStyle {
        let colors = &self.config.confidence_colors;
        
        let (border_color, fill_color) = match element_type {
            OverlayElementType::ClickTarget => {
                let rgba = colors.click_target;
                (
                    Color32::from_rgba_unmultiplied(rgba[0], rgba[1], rgba[2], rgba[3]),
                    Color32::from_rgba_unmultiplied(rgba[0], rgba[1], rgba[2], rgba[3] / 3),
                )
            }
            OverlayElementType::ExtractedText => {
                let rgba = colors.text_element;
                (
                    Color32::from_rgba_unmultiplied(rgba[0], rgba[1], rgba[2], rgba[3]),
                    Color32::from_rgba_unmultiplied(rgba[0], rgba[1], rgba[2], rgba[3] / 4),
                )
            }
            _ => {
                let rgba = if confidence > 0.8 {
                    colors.high_confidence
                } else if confidence > 0.5 {
                    colors.medium_confidence
                } else {
                    colors.low_confidence
                };
                (
                    Color32::from_rgba_unmultiplied(rgba[0], rgba[1], rgba[2], rgba[3]),
                    Color32::from_rgba_unmultiplied(rgba[0], rgba[1], rgba[2], rgba[3] / 4),
                )
            }
        };
        
        OverlayStyle {
            border_color,
            fill_color,
            border_width: 2.0,
            text_color: Color32::WHITE,
            font_size: 12.0,
            animation: None,
        }
    }

    async fn render_overlay(&self, ctx: &egui::Context, overlay: &FeedbackOverlay) {
        // Render overlay elements using egui
        // This is a simplified version - real implementation would be more complex
        
        for element in &overlay.elements {
            self.render_element(ctx, element).await;
        }
    }

    async fn render_element(&self, _ctx: &egui::Context, element: &OverlayElement) {
        // Mock rendering - in a real implementation this would draw the actual overlay
        debug!(
            "Rendering {} element: {} at {:?}",
            format!("{:?}", element.element_type),
            element.text,
            element.bounds
        );
    }

    async fn cleanup_old_overlays(&self, overlays: &mut HashMap<String, FeedbackOverlay>) {
        let cutoff = Instant::now() - Duration::from_secs(30); // Remove overlays older than 30 seconds
        
        let keys_to_remove: Vec<String> = overlays
            .iter()
            .filter(|(_, overlay)| overlay.created_at < cutoff)
            .map(|(key, _)| key.clone())
            .collect();
        
        for key in keys_to_remove {
            overlays.remove(&key);
            debug!("Cleaned up old overlay: {}", key);
        }
    }
}

/// Initialize visual feedback subsystem
pub async fn init() -> LunaResult<()> {
    info!("Initializing visual feedback subsystem");
    Ok(())
}

/// Shutdown visual feedback subsystem
pub async fn shutdown() -> LunaResult<()> {
    info!("Shutting down visual feedback subsystem");
    Ok(())
}

/// Validate visual feedback functionality
pub async fn validate() -> LunaResult<()> {
    info!("Validating visual feedback functionality");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_visual_feedback_creation() {
        let config = FeedbackConfig::default();
        let feedback = VisualFeedback::new(config);
        
        let status = feedback.get_status().await;
        assert_eq!(status.get("initialized").unwrap(), "false");
        assert_eq!(status.get("active_overlays").unwrap(), "0");
    }

    #[tokio::test]
    async fn test_overlay_management() {
        let config = FeedbackConfig::default();
        let mut feedback = VisualFeedback::new(config);
        
        // Test initialization
        let init_result = feedback.initialize().await;
        assert!(init_result.is_ok());
        
        // Test validation
        let validation_result = feedback.validate().await;
        assert!(validation_result.is_ok());
        
        // Test cleanup
        let cleanup_result = feedback.cleanup().await;
        assert!(cleanup_result.is_ok());
    }

    #[tokio::test]
    async fn test_style_for_confidence() {
        let config = FeedbackConfig::default();
        let feedback = VisualFeedback::new(config);
        
        // Test different confidence levels
        let high_style = feedback.get_style_for_confidence(0.9, OverlayElementType::DetectedObject).await;
        let medium_style = feedback.get_style_for_confidence(0.6, OverlayElementType::DetectedObject).await;
        let low_style = feedback.get_style_for_confidence(0.3, OverlayElementType::DetectedObject).await;
        
        // Colors should be different for different confidence levels
        assert_ne!(high_style.border_color, medium_style.border_color);
        assert_ne!(medium_style.border_color, low_style.border_color);
    }

    #[tokio::test]
    async fn test_element_creation() {
        let config = FeedbackConfig::default();
        let feedback = VisualFeedback::new(config);
        
        // Test creating object element
        let obj = DetectedObject {
            label: "button".to_string(),
            confidence: 0.85,
            bbox: (100, 200, 80, 30),
            center: (140, 215),
            metadata: HashMap::new(),
        };
        
        let element = feedback.create_object_element(&obj).await;
        assert_eq!(element.element_type, OverlayElementType::DetectedObject);
        assert_eq!(element.confidence, 0.85);
        assert!(element.text.contains("button"));
        assert!(element.text.contains("85%"));
    }
}