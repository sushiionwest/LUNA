// Visual overlay system with minimal dependencies
// Custom implementation for drawing UI overlays without heavy GUI frameworks

use crate::utils::geometry::{Point, Rectangle};
use crate::vision::{UIElement, ElementType};
use std::collections::HashMap;
use std::time::{Duration, Instant};

pub mod rendering;
pub mod animations;

#[derive(Debug, Clone)]
pub struct OverlayConfig {
    pub enable_highlights: bool,
    pub enable_labels: bool,
    pub enable_animations: bool,
    pub highlight_color: Color,
    pub label_color: Color,
    pub border_width: f64,
    pub font_size: f64,
    pub fade_duration: Duration,
}

impl Default for OverlayConfig {
    fn default() -> Self {
        Self {
            enable_highlights: true,
            enable_labels: true,
            enable_animations: true,
            highlight_color: Color::rgba(0, 255, 0, 128), // Semi-transparent green
            label_color: Color::rgb(255, 255, 255), // White
            border_width: 2.0,
            font_size: 12.0,
            fade_duration: Duration::from_millis(300),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub fn transparent() -> Self {
        Self { r: 0, g: 0, b: 0, a: 0 }
    }

    pub fn with_alpha(&self, alpha: u8) -> Self {
        Self {
            r: self.r,
            g: self.g,
            b: self.b,
            a: alpha,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OverlayElement {
    pub id: String,
    pub element_type: OverlayElementType,
    pub bounds: Rectangle,
    pub color: Color,
    pub text: Option<String>,
    pub visible: bool,
    pub created_at: Instant,
    pub properties: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub enum OverlayElementType {
    Highlight,
    Label,
    Border,
    Arrow,
    Circle,
    Custom(String),
}

pub struct OverlayManager {
    config: OverlayConfig,
    elements: HashMap<String, OverlayElement>,
    animations: HashMap<String, Animation>,
    next_id: u64,
}

impl OverlayManager {
    pub fn new(config: OverlayConfig) -> Self {
        Self {
            config,
            elements: HashMap::new(),
            animations: HashMap::new(),
            next_id: 0,
        }
    }

    pub fn add_ui_element_highlights(&mut self, ui_elements: &[UIElement]) {
        for element in ui_elements {
            let color = self.get_color_for_element_type(&element.element_type);
            let id = self.generate_id();
            
            let overlay_element = OverlayElement {
                id: id.clone(),
                element_type: OverlayElementType::Highlight,
                bounds: element.bounds,
                color,
                text: Some(format!("{} ({:.1}%)", element.element_type, element.confidence * 100.0)),
                visible: true,
                created_at: Instant::now(),
                properties: HashMap::new(),
            };
            
            self.elements.insert(id.clone(), overlay_element);
            
            // Add fade-in animation if enabled
            if self.config.enable_animations {
                self.add_fade_in_animation(&id);
            }
        }
    }

    pub fn add_highlight(&mut self, bounds: Rectangle, color: Color, text: Option<String>) -> String {
        let id = self.generate_id();
        
        let overlay_element = OverlayElement {
            id: id.clone(),
            element_type: OverlayElementType::Highlight,
            bounds,
            color,
            text,
            visible: true,
            created_at: Instant::now(),
            properties: HashMap::new(),
        };
        
        self.elements.insert(id.clone(), overlay_element);
        id
    }

    pub fn add_label(&mut self, position: Point, text: String, color: Color) -> String {
        let id = self.generate_id();
        
        // Create a small rectangle around the text position
        let bounds = Rectangle::new(
            position.x,
            position.y,
            text.len() as f64 * self.config.font_size * 0.6, // Rough text width estimation
            self.config.font_size,
        );
        
        let overlay_element = OverlayElement {
            id: id.clone(),
            element_type: OverlayElementType::Label,
            bounds,
            color,
            text: Some(text),
            visible: true,
            created_at: Instant::now(),
            properties: HashMap::new(),
        };
        
        self.elements.insert(id.clone(), overlay_element);
        id
    }

    pub fn add_arrow(&mut self, start: Point, end: Point, color: Color) -> String {
        let id = self.generate_id();
        
        // Create bounds that encompass the arrow
        let min_x = start.x.min(end.x);
        let max_x = start.x.max(end.x);
        let min_y = start.y.min(end.y);
        let max_y = start.y.max(end.y);
        
        let bounds = Rectangle::new(min_x, min_y, max_x - min_x, max_y - min_y);
        
        let mut properties = HashMap::new();
        properties.insert("start_x".to_string(), start.x.to_string());
        properties.insert("start_y".to_string(), start.y.to_string());
        properties.insert("end_x".to_string(), end.x.to_string());
        properties.insert("end_y".to_string(), end.y.to_string());
        
        let overlay_element = OverlayElement {
            id: id.clone(),
            element_type: OverlayElementType::Arrow,
            bounds,
            color,
            text: None,
            visible: true,
            created_at: Instant::now(),
            properties,
        };
        
        self.elements.insert(id.clone(), overlay_element);
        id
    }

    pub fn add_circle(&mut self, center: Point, radius: f64, color: Color) -> String {
        let id = self.generate_id();
        
        let bounds = Rectangle::new(
            center.x - radius,
            center.y - radius,
            radius * 2.0,
            radius * 2.0,
        );
        
        let mut properties = HashMap::new();
        properties.insert("center_x".to_string(), center.x.to_string());
        properties.insert("center_y".to_string(), center.y.to_string());
        properties.insert("radius".to_string(), radius.to_string());
        
        let overlay_element = OverlayElement {
            id: id.clone(),
            element_type: OverlayElementType::Circle,
            bounds,
            color,
            text: None,
            visible: true,
            created_at: Instant::now(),
            properties,
        };
        
        self.elements.insert(id.clone(), overlay_element);
        id
    }

    pub fn remove_element(&mut self, id: &str) {
        self.elements.remove(id);
        self.animations.remove(id);
    }

    pub fn clear_all(&mut self) {
        self.elements.clear();
        self.animations.clear();
    }

    pub fn clear_older_than(&mut self, duration: Duration) {
        let cutoff_time = Instant::now() - duration;
        
        self.elements.retain(|id, element| {
            let should_keep = element.created_at > cutoff_time;
            if !should_keep {
                self.animations.remove(id);
            }
            should_keep
        });
    }

    pub fn set_element_visibility(&mut self, id: &str, visible: bool) {
        if let Some(element) = self.elements.get_mut(id) {
            element.visible = visible;
        }
    }

    pub fn update_animations(&mut self, delta_time: Duration) {
        let current_time = Instant::now();
        let mut finished_animations = Vec::new();
        
        for (id, animation) in &mut self.animations {
            animation.update(delta_time);
            
            // Apply animation to element
            if let Some(element) = self.elements.get_mut(id) {
                animation.apply_to_element(element);
            }
            
            if animation.is_finished(current_time) {
                finished_animations.push(id.clone());
            }
        }
        
        // Remove finished animations
        for id in finished_animations {
            self.animations.remove(&id);
        }
    }

    pub fn get_visible_elements(&self) -> Vec<&OverlayElement> {
        self.elements.values()
            .filter(|element| element.visible)
            .collect()
    }

    pub fn get_element(&self, id: &str) -> Option<&OverlayElement> {
        self.elements.get(id)
    }

    pub fn get_elements_at_point(&self, point: &Point) -> Vec<&OverlayElement> {
        self.elements.values()
            .filter(|element| element.visible && element.bounds.contains_point(point))
            .collect()
    }

    fn generate_id(&mut self) -> String {
        let id = format!("overlay_{}", self.next_id);
        self.next_id += 1;
        id
    }

    fn get_color_for_element_type(&self, element_type: &ElementType) -> Color {
        match element_type {
            ElementType::Button => Color::rgb(0, 255, 0),     // Green
            ElementType::TextBox => Color::rgb(0, 0, 255),    // Blue
            ElementType::Label => Color::rgb(255, 255, 0),    // Yellow
            ElementType::Menu => Color::rgb(255, 0, 255),     // Magenta
            ElementType::Window => Color::rgb(255, 165, 0),   // Orange
            ElementType::Icon => Color::rgb(0, 255, 255),     // Cyan
            ElementType::Image => Color::rgb(128, 0, 128),    // Purple
            ElementType::Unknown => Color::rgb(128, 128, 128), // Gray
        }
    }

    fn add_fade_in_animation(&mut self, element_id: &str) {
        let animation = Animation::new(
            AnimationType::FadeIn,
            self.config.fade_duration,
            Instant::now(),
        );
        
        self.animations.insert(element_id.to_string(), animation);
    }

    pub fn add_fade_out_animation(&mut self, element_id: &str) {
        let animation = Animation::new(
            AnimationType::FadeOut,
            self.config.fade_duration,
            Instant::now(),
        );
        
        self.animations.insert(element_id.to_string(), animation);
    }

    pub fn highlight_element_sequence(&mut self, elements: &[UIElement], delay_between: Duration) {
        for (index, element) in elements.iter().enumerate() {
            let color = self.get_color_for_element_type(&element.element_type);
            let id = self.generate_id();
            
            let overlay_element = OverlayElement {
                id: id.clone(),
                element_type: OverlayElementType::Highlight,
                bounds: element.bounds,
                color,
                text: Some(format!("{} #{}", element.element_type, index + 1)),
                visible: false, // Start invisible
                created_at: Instant::now(),
                properties: HashMap::new(),
            };
            
            self.elements.insert(id.clone(), overlay_element);
            
            // Add delayed fade-in animation
            let start_time = Instant::now() + delay_between * index as u32;
            let animation = Animation::new(
                AnimationType::FadeIn,
                self.config.fade_duration,
                start_time,
            );
            
            self.animations.insert(id, animation);
        }
    }
}

impl Default for OverlayManager {
    fn default() -> Self {
        Self::new(OverlayConfig::default())
    }
}

#[derive(Debug, Clone)]
pub struct Animation {
    animation_type: AnimationType,
    duration: Duration,
    start_time: Instant,
    progress: f64,
}

#[derive(Debug, Clone)]
pub enum AnimationType {
    FadeIn,
    FadeOut,
    Scale(f64, f64), // from_scale, to_scale
    Move(Point, Point), // from_position, to_position
    Pulse,
}

impl Animation {
    pub fn new(animation_type: AnimationType, duration: Duration, start_time: Instant) -> Self {
        Self {
            animation_type,
            duration,
            start_time,
            progress: 0.0,
        }
    }

    pub fn update(&mut self, _delta_time: Duration) {
        let elapsed = self.start_time.elapsed();
        self.progress = (elapsed.as_secs_f64() / self.duration.as_secs_f64()).min(1.0);
    }

    pub fn apply_to_element(&self, element: &mut OverlayElement) {
        if self.progress <= 0.0 {
            return; // Animation hasn't started yet
        }
        
        match &self.animation_type {
            AnimationType::FadeIn => {
                element.visible = true;
                let alpha = (self.progress * element.color.a as f64) as u8;
                element.color = element.color.with_alpha(alpha);
            }
            AnimationType::FadeOut => {
                let alpha = ((1.0 - self.progress) * element.color.a as f64) as u8;
                element.color = element.color.with_alpha(alpha);
                
                if self.progress >= 1.0 {
                    element.visible = false;
                }
            }
            AnimationType::Scale(from_scale, to_scale) => {
                let current_scale = from_scale + (to_scale - from_scale) * self.progress;
                let center = element.bounds.center();
                let new_width = element.bounds.width * current_scale;
                let new_height = element.bounds.height * current_scale;
                
                element.bounds = Rectangle::new(
                    center.x - new_width / 2.0,
                    center.y - new_height / 2.0,
                    new_width,
                    new_height,
                );
            }
            AnimationType::Move(from_pos, to_pos) => {
                let current_x = from_pos.x + (to_pos.x - from_pos.x) * self.progress;
                let current_y = from_pos.y + (to_pos.y - from_pos.y) * self.progress;
                
                let width = element.bounds.width;
                let height = element.bounds.height;
                element.bounds = Rectangle::new(current_x, current_y, width, height);
            }
            AnimationType::Pulse => {
                // Create a pulsing effect by modulating alpha
                let pulse = (self.progress * std::f64::consts::PI * 4.0).sin().abs();
                let alpha = (pulse * element.color.a as f64) as u8;
                element.color = element.color.with_alpha(alpha);
            }
        }
    }

    pub fn is_finished(&self, current_time: Instant) -> bool {
        current_time >= self.start_time + self.duration
    }
}

// Utility functions for common overlay operations
pub fn create_ui_highlights(ui_elements: &[UIElement]) -> OverlayManager {
    let mut manager = OverlayManager::default();
    manager.add_ui_element_highlights(ui_elements);
    manager
}

pub fn create_simple_highlight(bounds: Rectangle, color: Color) -> OverlayManager {
    let mut manager = OverlayManager::default();
    manager.add_highlight(bounds, color, None);
    manager
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vision::UIElement;

    #[test]
    fn test_overlay_manager_creation() {
        let manager = OverlayManager::default();
        assert!(manager.elements.is_empty());
        assert!(manager.animations.is_empty());
    }

    #[test]
    fn test_add_highlight() {
        let mut manager = OverlayManager::default();
        let bounds = Rectangle::new(10.0, 10.0, 100.0, 50.0);
        let color = Color::rgb(255, 0, 0);
        
        let id = manager.add_highlight(bounds, color, Some("Test".to_string()));
        
        assert_eq!(manager.elements.len(), 1);
        assert!(manager.elements.contains_key(&id));
        
        let element = manager.get_element(&id).unwrap();
        assert_eq!(element.bounds, bounds);
        assert_eq!(element.text, Some("Test".to_string()));
    }

    #[test]
    fn test_add_label() {
        let mut manager = OverlayManager::default();
        let position = Point::new(50.0, 75.0);
        let text = "Label Text".to_string();
        let color = Color::rgb(255, 255, 255);
        
        let id = manager.add_label(position, text.clone(), color);
        
        let element = manager.get_element(&id).unwrap();
        assert_eq!(element.text, Some(text));
        assert!(element.bounds.contains_point(&position));
    }

    #[test]
    fn test_remove_element() {
        let mut manager = OverlayManager::default();
        let bounds = Rectangle::new(0.0, 0.0, 10.0, 10.0);
        let color = Color::rgb(255, 0, 0);
        
        let id = manager.add_highlight(bounds, color, None);
        assert_eq!(manager.elements.len(), 1);
        
        manager.remove_element(&id);
        assert_eq!(manager.elements.len(), 0);
    }

    #[test]
    fn test_clear_all() {
        let mut manager = OverlayManager::default();
        let bounds = Rectangle::new(0.0, 0.0, 10.0, 10.0);
        let color = Color::rgb(255, 0, 0);
        
        manager.add_highlight(bounds, color, None);
        manager.add_label(Point::new(5.0, 5.0), "Test".to_string(), color);
        
        assert_eq!(manager.elements.len(), 2);
        
        manager.clear_all();
        assert_eq!(manager.elements.len(), 0);
    }

    #[test]
    fn test_element_visibility() {
        let mut manager = OverlayManager::default();
        let bounds = Rectangle::new(0.0, 0.0, 10.0, 10.0);
        let color = Color::rgb(255, 0, 0);
        
        let id = manager.add_highlight(bounds, color, None);
        
        // Should be visible by default
        assert!(manager.get_element(&id).unwrap().visible);
        assert_eq!(manager.get_visible_elements().len(), 1);
        
        // Hide element
        manager.set_element_visibility(&id, false);
        assert!(!manager.get_element(&id).unwrap().visible);
        assert_eq!(manager.get_visible_elements().len(), 0);
        
        // Show element again
        manager.set_element_visibility(&id, true);
        assert!(manager.get_element(&id).unwrap().visible);
        assert_eq!(manager.get_visible_elements().len(), 1);
    }

    #[test]
    fn test_color_creation() {
        let color1 = Color::rgb(255, 128, 64);
        assert_eq!(color1.r, 255);
        assert_eq!(color1.g, 128);
        assert_eq!(color1.b, 64);
        assert_eq!(color1.a, 255);
        
        let color2 = Color::rgba(100, 50, 25, 128);
        assert_eq!(color2.a, 128);
        
        let transparent = Color::transparent();
        assert_eq!(transparent.a, 0);
        
        let with_alpha = color1.with_alpha(100);
        assert_eq!(with_alpha.a, 100);
        assert_eq!(with_alpha.r, 255); // Other channels should remain the same
    }

    #[test]
    fn test_animation_progress() {
        let start_time = Instant::now();
        let duration = Duration::from_millis(100);
        let mut animation = Animation::new(AnimationType::FadeIn, duration, start_time);
        
        // Initially should have 0 progress
        assert_eq!(animation.progress, 0.0);
        
        // Update after half the duration
        std::thread::sleep(Duration::from_millis(50));
        animation.update(Duration::from_millis(50));
        
        // Should be roughly halfway
        assert!(animation.progress > 0.3 && animation.progress < 0.7);
    }

    #[test]
    fn test_get_elements_at_point() {
        let mut manager = OverlayManager::default();
        
        let bounds1 = Rectangle::new(0.0, 0.0, 10.0, 10.0);
        let bounds2 = Rectangle::new(5.0, 5.0, 10.0, 10.0); // Overlapping
        let bounds3 = Rectangle::new(20.0, 20.0, 10.0, 10.0); // Separate
        
        let color = Color::rgb(255, 0, 0);
        manager.add_highlight(bounds1, color, None);
        manager.add_highlight(bounds2, color, None);
        manager.add_highlight(bounds3, color, None);
        
        // Point in overlap area
        let point1 = Point::new(7.0, 7.0);
        let elements1 = manager.get_elements_at_point(&point1);
        assert_eq!(elements1.len(), 2); // Should find both overlapping elements
        
        // Point in separate area
        let point2 = Point::new(25.0, 25.0);
        let elements2 = manager.get_elements_at_point(&point2);
        assert_eq!(elements2.len(), 1); // Should find only one element
        
        // Point in empty space
        let point3 = Point::new(50.0, 50.0);
        let elements3 = manager.get_elements_at_point(&point3);
        assert_eq!(elements3.len(), 0); // Should find no elements
    }
}