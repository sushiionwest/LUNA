/*!
 * Luna AI - Lightweight Computer Vision and Pattern Recognition
 * 
 * Simplified AI pipeline using basic computer vision algorithms instead of heavy ML models
 */

use anyhow::Result;
use image::{DynamicImage, Rgb, RgbImage, Luma};
use std::collections::HashMap;
use log::{debug, info, warn};

use crate::core::{ScreenAnalysis, ScreenElement, LunaAction, ElementBounds};

/// Lightweight AI coordinator for screen analysis and action planning
pub struct AICoordinator {
    /// Confidence threshold for element detection
    confidence_threshold: f32,
    /// Maximum number of elements to detect
    max_elements: usize,
    /// Processing statistics
    stats: ProcessingStats,
}

/// Lightweight computer vision model for UI element detection
pub struct VisionProcessor {
    /// Edge detection sensitivity
    edge_threshold: f32,
    /// Minimum element size (pixels)
    min_element_size: u32,
    /// Element classification rules
    classification_rules: HashMap<String, ClassificationRule>,
}

/// Element detection result
#[derive(Debug, Clone)]
pub struct ElementDetection {
    pub element_type: String,
    pub bounds: ElementBounds,
    pub confidence: f32,
    pub text: Option<String>,
    pub attributes: HashMap<String, String>,
}

/// Classification rule for UI elements
#[derive(Debug, Clone)]
struct ClassificationRule {
    pub aspect_ratio_min: f32,
    pub aspect_ratio_max: f32,
    pub area_min: i32,
    pub area_max: i32,
    pub brightness_threshold: Option<f32>,
}

/// Processing statistics
#[derive(Debug, Default, Clone)]
pub struct ProcessingStats {
    pub images_processed: u64,
    pub elements_detected: u64,
    pub total_processing_time_ms: u64,
    pub average_processing_time_ms: f64,
}

impl AICoordinator {
    /// Create new AI coordinator
    pub fn new() -> Self {
        Self {
            confidence_threshold: 0.6,
            max_elements: 50,
            stats: ProcessingStats::default(),
        }
    }

    /// Analyze screen image and detect UI elements
    pub fn analyze_screen(&mut self, image: &DynamicImage) -> Result<ScreenAnalysis> {
        let start_time = std::time::Instant::now();
        
        debug!("Starting screen analysis {}x{}", image.width(), image.height());
        
        // Use lightweight computer vision processor
        let mut vision = VisionProcessor::new();
        let elements = vision.detect_elements(image)?;
        
        // Filter by confidence threshold
        let filtered_elements: Vec<ScreenElement> = elements
            .into_iter()
            .filter(|e| e.confidence >= self.confidence_threshold)
            .take(self.max_elements)
            .map(|e| ScreenElement {
                element_type: e.element_type,
                bounds: e.bounds,
                confidence: e.confidence,
                text: e.text,
                attributes: e.attributes,
            })
            .collect();

        let processing_time = start_time.elapsed();
        let processing_time_ms = processing_time.as_millis() as u64;
        
        // Update statistics
        self.stats.images_processed += 1;
        self.stats.elements_detected += filtered_elements.len() as u64;
        self.stats.total_processing_time_ms += processing_time_ms;
        self.stats.average_processing_time_ms = 
            self.stats.total_processing_time_ms as f64 / self.stats.images_processed as f64;

        info!("Screen analysis complete: {} elements detected in {}ms", 
              filtered_elements.len(), processing_time_ms);

        Ok(ScreenAnalysis {
            elements: filtered_elements,
            confidence: self.calculate_overall_confidence(&filtered_elements),
            processing_time_ms,
            screen_size: (image.width(), image.height()),
        })
    }

    /// Plan actions based on user command and screen analysis
    pub fn plan_actions(&self, command: &str, analysis: &ScreenAnalysis) -> Result<Vec<LunaAction>> {
        debug!("Planning actions for command: '{}'", command);
        
        let command_lower = command.to_lowercase();
        let mut actions = Vec::new();

        // Simple command parsing and action planning
        if command_lower.contains("click") {
            if let Some(element) = self.find_clickable_element(&command_lower, &analysis.elements) {
                let center_x = element.bounds.x + element.bounds.width / 2;
                let center_y = element.bounds.y + element.bounds.height / 2;
                
                actions.push(LunaAction::Click { 
                    x: center_x, 
                    y: center_y 
                });
            }
        } else if command_lower.contains("type") || command_lower.contains("enter") {
            if let Some(text) = self.extract_text_from_command(&command) {
                actions.push(LunaAction::Type { text });
            }
        } else if command_lower.contains("scroll") {
            let direction = if command_lower.contains("up") { "up" }
                          else if command_lower.contains("down") { "down" }
                          else { "down" };
            
            actions.push(LunaAction::Scroll { 
                direction: direction.to_string(),
                amount: 3 
            });
        }

        debug!("Planned {} actions", actions.len());
        Ok(actions)
    }

    /// Get processing statistics
    pub fn get_stats(&self) -> &ProcessingStats {
        &self.stats
    }

    /// Calculate overall confidence from detected elements
    fn calculate_overall_confidence(&self, elements: &[ScreenElement]) -> f32 {
        if elements.is_empty() {
            return 0.0;
        }
        
        let total_confidence: f32 = elements.iter().map(|e| e.confidence).sum();
        total_confidence / elements.len() as f32
    }

    /// Find the best clickable element for a command
    fn find_clickable_element(&self, command: &str, elements: &[ScreenElement]) -> Option<&ScreenElement> {
        // Look for specific element types mentioned in command
        let button_keywords = ["button", "click", "press"];
        let link_keywords = ["link", "navigate", "go to"];
        
        // First, try to find elements by type preference
        for keyword in &button_keywords {
            if command.contains(keyword) {
                if let Some(button) = elements.iter().find(|e| e.element_type == "button") {
                    return Some(button);
                }
            }
        }
        
        for keyword in &link_keywords {
            if command.contains(keyword) {
                if let Some(link) = elements.iter().find(|e| e.element_type == "link") {
                    return Some(link);
                }
            }
        }

        // Look for text matches
        for element in elements {
            if let Some(text) = &element.text {
                let text_lower = text.to_lowercase();
                for word in command.split_whitespace() {
                    if text_lower.contains(word) && word.len() > 2 {
                        return Some(element);
                    }
                }
            }
        }

        // Fall back to first clickable element
        elements.iter()
            .find(|e| matches!(e.element_type.as_str(), "button" | "link" | "icon"))
    }

    /// Extract text to type from command
    fn extract_text_from_command(&self, command: &str) -> Option<String> {
        // Simple text extraction - look for quoted text or text after "type"
        if let Some(start) = command.find('"') {
            if let Some(end) = command[start + 1..].find('"') {
                return Some(command[start + 1..start + 1 + end].to_string());
            }
        }

        // Look for text after "type" keyword
        if let Some(type_pos) = command.to_lowercase().find("type") {
            let after_type = &command[type_pos + 4..].trim();
            if !after_type.is_empty() {
                return Some(after_type.to_string());
            }
        }

        None
    }
}

impl VisionProcessor {
    /// Create new vision processor with default settings
    pub fn new() -> Self {
        let mut classification_rules = HashMap::new();
        
        // Button classification rules
        classification_rules.insert("button".to_string(), ClassificationRule {
            aspect_ratio_min: 1.5,
            aspect_ratio_max: 5.0,
            area_min: 800,
            area_max: 8000,
            brightness_threshold: Some(180.0),
        });

        // Text field classification rules
        classification_rules.insert("textfield".to_string(), ClassificationRule {
            aspect_ratio_min: 3.0,
            aspect_ratio_max: 10.0,
            area_min: 1500,
            area_max: 15000,
            brightness_threshold: Some(200.0),
        });

        // Icon classification rules
        classification_rules.insert("icon".to_string(), ClassificationRule {
            aspect_ratio_min: 0.8,
            aspect_ratio_max: 1.2,
            area_min: 100,
            area_max: 2000,
            brightness_threshold: None,
        });

        Self {
            edge_threshold: 30.0,
            min_element_size: 20,
            classification_rules,
        }
    }

    /// Detect UI elements in image using lightweight computer vision
    pub fn detect_elements(&mut self, image: &DynamicImage) -> Result<Vec<ElementDetection>> {
        let mut elements = Vec::new();
        
        // Convert to RGB for processing
        let rgb_image = image.to_rgb8();
        
        // Step 1: Edge detection using Sobel operator
        let edges = self.detect_edges(&rgb_image);
        
        // Step 2: Find rectangular regions from edges
        let rectangles = self.find_rectangles(&edges, image.width(), image.height());
        
        // Step 3: Classify each rectangle as UI element
        for rect in rectangles {
            if let Some(element) = self.classify_element(&rect, &rgb_image) {
                elements.push(element);
            }
        }

        // Sort by confidence and limit results
        elements.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));
        elements.truncate(50);

        debug!("Detected {} UI elements", elements.len());
        Ok(elements)
    }

    /// Detect edges using Sobel operator
    fn detect_edges(&self, image: &RgbImage) -> Vec<(u32, u32)> {
        let gray_image = DynamicImage::ImageRgb8(image.clone()).to_luma8();
        let (width, height) = gray_image.dimensions();
        let mut edges = Vec::new();
        
        // Sobel kernels
        let sobel_x = [[-1, 0, 1], [-2, 0, 2], [-1, 0, 1]];
        let sobel_y = [[-1, -2, -1], [0, 0, 0], [1, 2, 1]];
        
        for y in 1..height-1 {
            for x in 1..width-1 {
                let mut gx = 0.0;
                let mut gy = 0.0;
                
                // Apply Sobel kernels
                for dy in 0..3 {
                    for dx in 0..3 {
                        let pixel = gray_image.get_pixel(x + dx - 1, y + dy - 1)[0] as f32;
                        gx += pixel * sobel_x[dy][dx] as f32;
                        gy += pixel * sobel_y[dy][dx] as f32;
                    }
                }
                
                let magnitude = (gx * gx + gy * gy).sqrt();
                
                if magnitude > self.edge_threshold {
                    edges.push((x, y));
                }
            }
        }
        
        edges
    }

    /// Find rectangular regions from edge points
    fn find_rectangles(&self, edges: &[(u32, u32)], width: u32, height: u32) -> Vec<ElementBounds> {
        let mut rectangles = Vec::new();
        let mut visited = std::collections::HashSet::new();
        
        for &(x, y) in edges {
            if visited.contains(&(x, y)) {
                continue;
            }
            
            if let Some(rect) = self.grow_rectangle_from_point(x, y, edges, &mut visited) {
                // Validate rectangle size
                if rect.width >= self.min_element_size as i32 && 
                   rect.height >= (self.min_element_size / 2) as i32 &&
                   rect.width <= (width / 2) as i32 && 
                   rect.height <= (height / 2) as i32 {
                    rectangles.push(rect);
                }
            }
        }
        
        rectangles
    }

    /// Grow a rectangle from an edge point using connected components
    fn grow_rectangle_from_point(
        &self,
        start_x: u32, 
        start_y: u32, 
        edges: &[(u32, u32)], 
        visited: &mut std::collections::HashSet<(u32, u32)>
    ) -> Option<ElementBounds> {
        let mut min_x = start_x;
        let mut max_x = start_x;
        let mut min_y = start_y;
        let mut max_y = start_y;
        
        let mut stack = vec![(start_x, start_y)];
        let mut component_size = 0;
        
        while let Some((x, y)) = stack.pop() {
            if visited.contains(&(x, y)) || component_size > 1000 {
                continue;
            }
            
            visited.insert((x, y));
            component_size += 1;
            
            min_x = min_x.min(x);
            max_x = max_x.max(x);
            min_y = min_y.min(y);
            max_y = max_y.max(y);
            
            // Find nearby edge points
            for &(nx, ny) in edges {
                if !visited.contains(&(nx, ny)) {
                    let dx = (x as i32 - nx as i32).abs();
                    let dy = (y as i32 - ny as i32).abs();
                    
                    if dx <= 3 && dy <= 3 {
                        stack.push((nx, ny));
                    }
                }
            }
        }
        
        // Check if component forms a reasonable rectangle
        if component_size > 20 {
            Some(ElementBounds {
                x: min_x as i32,
                y: min_y as i32,
                width: (max_x - min_x) as i32,
                height: (max_y - min_y) as i32,
            })
        } else {
            None
        }
    }

    /// Classify a rectangle as a UI element type
    fn classify_element(&self, rect: &ElementBounds, image: &RgbImage) -> Option<ElementDetection> {
        let aspect_ratio = rect.width as f32 / rect.height as f32;
        let area = rect.width * rect.height;
        let brightness = self.calculate_average_brightness(image, rect);
        
        // Try to match against classification rules
        for (element_type, rule) in &self.classification_rules {
            if aspect_ratio >= rule.aspect_ratio_min && 
               aspect_ratio <= rule.aspect_ratio_max &&
               area >= rule.area_min && 
               area <= rule.area_max {
                
                // Check brightness threshold if specified
                if let Some(brightness_threshold) = rule.brightness_threshold {
                    if brightness < brightness_threshold {
                        continue;
                    }
                }
                
                let confidence = self.calculate_confidence(rect, element_type, aspect_ratio, area);
                
                return Some(ElementDetection {
                    element_type: element_type.clone(),
                    bounds: rect.clone(),
                    confidence,
                    text: None, // TODO: Implement simple OCR
                    attributes: self.extract_attributes(rect, element_type),
                });
            }
        }
        
        // Default classification
        if area > 500 {
            Some(ElementDetection {
                element_type: "element".to_string(),
                bounds: rect.clone(),
                confidence: 0.3,
                text: None,
                attributes: HashMap::new(),
            })
        } else {
            None
        }
    }

    /// Calculate average brightness of a rectangular region
    fn calculate_average_brightness(&self, image: &RgbImage, rect: &ElementBounds) -> f32 {
        let mut total_brightness = 0.0;
        let mut pixel_count = 0;
        
        let end_x = ((rect.x + rect.width) as u32).min(image.width());
        let end_y = ((rect.y + rect.height) as u32).min(image.height());
        
        for y in (rect.y as u32)..end_y {
            for x in (rect.x as u32)..end_x {
                let pixel = image.get_pixel(x, y);
                let brightness = 0.299 * pixel[0] as f32 + 0.587 * pixel[1] as f32 + 0.114 * pixel[2] as f32;
                total_brightness += brightness;
                pixel_count += 1;
            }
        }
        
        if pixel_count > 0 {
            total_brightness / pixel_count as f32
        } else {
            0.0
        }
    }

    /// Calculate confidence for element classification
    fn calculate_confidence(&self, rect: &ElementBounds, element_type: &str, aspect_ratio: f32, area: i32) -> f32 {
        let mut confidence = 0.5;
        
        // Boost confidence based on element type characteristics
        match element_type {
            "button" => {
                if (1.5..=5.0).contains(&aspect_ratio) && (800..=8000).contains(&area) {
                    confidence += 0.3;
                }
            }
            "textfield" => {
                if aspect_ratio > 3.0 && (1500..=15000).contains(&area) {
                    confidence += 0.3;
                }
            }
            "icon" => {
                if (0.8..=1.2).contains(&aspect_ratio) && area < 2000 {
                    confidence += 0.4;
                }
            }
            _ => confidence -= 0.1,
        }
        
        // Penalize extreme sizes
        if area < 100 || area > 50000 {
            confidence -= 0.2;
        }
        
        confidence.clamp(0.0, 1.0)
    }

    /// Extract element attributes
    fn extract_attributes(&self, rect: &ElementBounds, element_type: &str) -> HashMap<String, String> {
        let mut attributes = HashMap::new();
        
        attributes.insert("area".to_string(), (rect.width * rect.height).to_string());
        attributes.insert("aspect_ratio".to_string(), (rect.width as f32 / rect.height as f32).to_string());
        
        match element_type {
            "button" => {
                attributes.insert("clickable".to_string(), "true".to_string());
            }
            "textfield" => {
                attributes.insert("editable".to_string(), "true".to_string());
            }
            "icon" => {
                attributes.insert("clickable".to_string(), "true".to_string());
            }
            _ => {}
        }
        
        attributes
    }
}

impl Default for AICoordinator {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for VisionProcessor {
    fn default() -> Self {
        Self::new()
    }
}

// Re-export for backward compatibility
pub use AICoordinator as AIVisionPipeline;