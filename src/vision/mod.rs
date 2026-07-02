// Computer vision module with custom implementations
// Replaces heavy AI/ML frameworks with efficient custom algorithms

use crate::utils::geometry::{Point, Rectangle};
use crate::utils::image_processing::{Image, sobel_edge_detection, threshold, find_connected_components};
use std::collections::HashMap;

pub mod screen_capture;
pub mod ui_detection;
pub mod text_recognition;

#[derive(Debug, Clone)]
pub struct VisionConfig {
    pub edge_threshold: u8,
    pub min_element_size: usize,
    pub max_element_size: usize,
    pub brightness_threshold: u8,
    pub contrast_threshold: f64,
}

impl Default for VisionConfig {
    fn default() -> Self {
        Self {
            edge_threshold: 50,
            min_element_size: 10,
            max_element_size: 1000,
            brightness_threshold: 128,
            contrast_threshold: 0.3,
        }
    }
}

#[derive(Debug, Clone)]
pub struct UIElement {
    pub bounds: Rectangle,
    pub element_type: ElementType,
    pub confidence: f64,
    pub properties: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq)]
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

impl std::fmt::Display for ElementType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ElementType::Button => write!(f, "Button"),
            ElementType::TextBox => write!(f, "TextBox"),
            ElementType::Label => write!(f, "Label"),
            ElementType::Menu => write!(f, "Menu"),
            ElementType::Window => write!(f, "Window"),
            ElementType::Icon => write!(f, "Icon"),
            ElementType::Image => write!(f, "Image"),
            ElementType::Unknown => write!(f, "Unknown"),
        }
    }
}

pub struct VisionPipeline {
    config: VisionConfig,
    cache: ElementCache,
}

impl VisionPipeline {
    pub fn new(config: VisionConfig) -> Self {
        Self {
            config,
            cache: ElementCache::new(),
        }
    }

    pub fn analyze_screen(&mut self, image: &Image) -> Result<Vec<UIElement>, VisionError> {
        // Check cache first
        let image_hash = self.calculate_image_hash(image);
        if let Some(cached_elements) = self.cache.get(&image_hash) {
            return Ok(cached_elements);
        }

        // Convert to grayscale for processing
        let gray_image = image.to_grayscale();
        
        // Step 1: Edge detection
        let edges = sobel_edge_detection(&gray_image);
        
        // Step 2: Find potential UI elements through edge grouping
        let edge_components = self.find_edge_rectangles(&edges)?;
        
        // Step 3: Classify each component
        let mut elements = Vec::new();
        for component in edge_components {
            if let Ok(element) = self.classify_component(image, &component) {
                elements.push(element);
            }
        }

        // Step 4: Filter and refine results
        elements = self.filter_elements(elements);
        
        // Cache results
        self.cache.set(image_hash, elements.clone());
        
        Ok(elements)
    }

    fn calculate_image_hash(&self, image: &Image) -> u64 {
        // Simple hash based on image properties and sample pixels
        let mut hash = 0u64;
        hash ^= image.width as u64;
        hash ^= (image.height as u64) << 16;
        
        // Sample a few pixels for content-based hashing
        let step_x = image.width / 10;
        let step_y = image.height / 10;
        
        for y in (0..image.height).step_by(step_y.max(1)) {
            for x in (0..image.width).step_by(step_x.max(1)) {
                if let Some(pixel) = image.get_pixel(x, y) {
                    hash ^= pixel[0] as u64;
                    hash = hash.wrapping_mul(31);
                }
            }
        }
        
        hash
    }

    fn find_edge_rectangles(&self, edges: &Image) -> Result<Vec<Rectangle>, VisionError> {
        // Apply threshold to edge image
        let binary = threshold(edges, self.config.edge_threshold);
        
        // Find connected components
        let components = find_connected_components(&binary);
        
        let mut rectangles = Vec::new();
        
        for component in components {
            if component.len() < self.config.min_element_size || 
               component.len() > self.config.max_element_size {
                continue;
            }
            
            // Calculate bounding rectangle
            let bounds = self.calculate_bounding_rect(&component);
            
            // Filter out very thin or very wide rectangles (likely noise)
            let aspect_ratio = bounds.width / bounds.height;
            if aspect_ratio > 0.1 && aspect_ratio < 10.0 {
                rectangles.push(bounds);
            }
        }
        
        Ok(rectangles)
    }

    fn calculate_bounding_rect(&self, points: &[Point]) -> Rectangle {
        if points.is_empty() {
            return Rectangle::new(0.0, 0.0, 0.0, 0.0);
        }
        
        let mut min_x = points[0].x;
        let mut max_x = points[0].x;
        let mut min_y = points[0].y;
        let mut max_y = points[0].y;
        
        for point in points.iter().skip(1) {
            min_x = min_x.min(point.x);
            max_x = max_x.max(point.x);
            min_y = min_y.min(point.y);
            max_y = max_y.max(point.y);
        }
        
        Rectangle::new(min_x, min_y, max_x - min_x, max_y - min_y)
    }

    fn classify_component(&self, image: &Image, bounds: &Rectangle) -> Result<UIElement, VisionError> {
        // Extract the region of interest
        let roi = image.crop(bounds);
        
        // Analyze properties
        let brightness = self.calculate_average_brightness(&roi);
        let edge_density = self.calculate_edge_density(&roi);
        let aspect_ratio = bounds.width / bounds.height;
        
        // Classification logic based on visual properties
        let (element_type, confidence) = self.classify_by_properties(
            bounds, brightness, edge_density, aspect_ratio
        );
        
        let mut properties = HashMap::new();
        properties.insert("brightness".to_string(), brightness.to_string());
        properties.insert("edge_density".to_string(), edge_density.to_string());
        properties.insert("aspect_ratio".to_string(), aspect_ratio.to_string());
        
        Ok(UIElement {
            bounds: *bounds,
            element_type,
            confidence,
            properties,
        })
    }

    fn calculate_average_brightness(&self, image: &Image) -> f64 {
        let gray = if image.channels == 1 {
            image.clone()
        } else {
            image.to_grayscale()
        };
        
        let mut sum = 0u64;
        let mut count = 0;
        
        for y in 0..gray.height {
            for x in 0..gray.width {
                if let Some(pixel) = gray.get_pixel(x, y) {
                    sum += pixel[0] as u64;
                    count += 1;
                }
            }
        }
        
        if count > 0 {
            sum as f64 / count as f64
        } else {
            0.0
        }
    }

    fn calculate_edge_density(&self, image: &Image) -> f64 {
        let edges = sobel_edge_detection(image);
        let binary = threshold(&edges, 50);
        
        let mut edge_pixels = 0;
        let total_pixels = binary.width * binary.height;
        
        for y in 0..binary.height {
            for x in 0..binary.width {
                if let Some(pixel) = binary.get_pixel(x, y) {
                    if pixel[0] > 0 {
                        edge_pixels += 1;
                    }
                }
            }
        }
        
        edge_pixels as f64 / total_pixels as f64
    }

    fn classify_by_properties(
        &self,
        bounds: &Rectangle,
        brightness: f64,
        edge_density: f64,
        aspect_ratio: f64,
    ) -> (ElementType, f64) {
        let area = bounds.area();
        
        // Button detection: moderate size, high edge density, roughly square/rectangular
        if area > 500.0 && area < 50000.0 && edge_density > 0.3 && aspect_ratio > 0.2 && aspect_ratio < 5.0 {
            return (ElementType::Button, 0.8);
        }
        
        // Text box detection: rectangular, moderate edge density, moderate brightness
        if aspect_ratio > 2.0 && edge_density > 0.2 && edge_density < 0.6 && brightness > 200.0 {
            return (ElementType::TextBox, 0.7);
        }
        
        // Window detection: large area, moderate edge density
        if area > 100000.0 && edge_density > 0.1 && edge_density < 0.4 {
            return (ElementType::Window, 0.6);
        }
        
        // Icon detection: small, roughly square, high contrast
        if area < 2000.0 && aspect_ratio > 0.5 && aspect_ratio < 2.0 && edge_density > 0.4 {
            return (ElementType::Icon, 0.7);
        }
        
        // Label detection: high aspect ratio, low edge density
        if aspect_ratio > 3.0 && edge_density < 0.3 {
            return (ElementType::Label, 0.6);
        }
        
        // Menu detection: moderate size, high aspect ratio or high edge density
        if (aspect_ratio > 1.5 || edge_density > 0.5) && area > 1000.0 && area < 100000.0 {
            return (ElementType::Menu, 0.5);
        }
        
        (ElementType::Unknown, 0.3)
    }

    fn filter_elements(&self, mut elements: Vec<UIElement>) -> Vec<UIElement> {
        // Remove overlapping elements, keeping the one with higher confidence
        elements.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        
        let mut filtered = Vec::new();
        
        for element in elements {
            let mut overlaps = false;
            
            for existing in &filtered {
                if element.bounds.intersects(&existing.bounds) {
                    let intersection = element.bounds.intersection(&existing.bounds);
                    if let Some(inter) = intersection {
                        let overlap_ratio = inter.area() / element.bounds.area().min(existing.bounds.area());
                        if overlap_ratio > 0.5 {
                            overlaps = true;
                            break;
                        }
                    }
                }
            }
            
            if !overlaps && element.confidence > 0.4 {
                filtered.push(element);
            }
        }
        
        filtered
    }

    pub fn find_element_by_type(&self, elements: &[UIElement], element_type: ElementType) -> Vec<&UIElement> {
        elements.iter()
            .filter(|element| element.element_type == element_type)
            .collect()
    }

    pub fn find_elements_in_region(&self, elements: &[UIElement], region: &Rectangle) -> Vec<&UIElement> {
        elements.iter()
            .filter(|element| region.intersects(&element.bounds))
            .collect()
    }
}

// Simple cache for vision results
struct ElementCache {
    cache: HashMap<u64, Vec<UIElement>>,
    max_size: usize,
}

impl ElementCache {
    fn new() -> Self {
        Self {
            cache: HashMap::new(),
            max_size: 100,
        }
    }

    fn get(&self, hash: &u64) -> Option<Vec<UIElement>> {
        self.cache.get(hash).cloned()
    }

    fn set(&mut self, hash: u64, elements: Vec<UIElement>) {
        if self.cache.len() >= self.max_size {
            // Simple eviction: remove a random entry
            if let Some(key) = self.cache.keys().next().cloned() {
                self.cache.remove(&key);
            }
        }
        self.cache.insert(hash, elements);
    }
}

#[derive(Debug)]
pub enum VisionError {
    ImageProcessingError(String),
    AnalysisError(String),
    CacheError(String),
}

impl std::fmt::Display for VisionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VisionError::ImageProcessingError(msg) => write!(f, "Image processing error: {}", msg),
            VisionError::AnalysisError(msg) => write!(f, "Analysis error: {}", msg),
            VisionError::CacheError(msg) => write!(f, "Cache error: {}", msg),
        }
    }
}

impl std::error::Error for VisionError {}

// Convenience functions for common operations
pub fn quick_analyze(image: &Image) -> Result<Vec<UIElement>, VisionError> {
    let mut pipeline = VisionPipeline::new(VisionConfig::default());
    pipeline.analyze_screen(image)
}

pub fn find_buttons(image: &Image) -> Result<Vec<UIElement>, VisionError> {
    let elements = quick_analyze(image)?;
    Ok(elements.into_iter()
        .filter(|e| e.element_type == ElementType::Button)
        .collect())
}

pub fn find_text_boxes(image: &Image) -> Result<Vec<UIElement>, VisionError> {
    let elements = quick_analyze(image)?;
    Ok(elements.into_iter()
        .filter(|e| e.element_type == ElementType::TextBox)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vision_config() {
        let config = VisionConfig::default();
        assert_eq!(config.edge_threshold, 50);
        assert_eq!(config.min_element_size, 10);
    }

    #[test]
    fn test_element_type_display() {
        assert_eq!(format!("{}", ElementType::Button), "Button");
        assert_eq!(format!("{}", ElementType::TextBox), "TextBox");
    }

    #[test]
    fn test_bounding_rect_calculation() {
        let pipeline = VisionPipeline::new(VisionConfig::default());
        let points = vec![
            Point::new(10.0, 10.0),
            Point::new(20.0, 10.0),
            Point::new(10.0, 20.0),
            Point::new(20.0, 20.0),
        ];
        
        let rect = pipeline.calculate_bounding_rect(&points);
        assert_eq!(rect.x, 10.0);
        assert_eq!(rect.y, 10.0);
        assert_eq!(rect.width, 10.0);
        assert_eq!(rect.height, 10.0);
    }

    #[test]
    fn test_brightness_calculation() {
        let pipeline = VisionPipeline::new(VisionConfig::default());
        let mut image = Image::new(2, 2, 1);
        
        // Set all pixels to value 100
        for y in 0..2 {
            for x in 0..2 {
                image.set_pixel(x, y, &[100]);
            }
        }
        
        let brightness = pipeline.calculate_average_brightness(&image);
        assert_eq!(brightness, 100.0);
    }

    #[test]
    fn test_element_filtering() {
        let pipeline = VisionPipeline::new(VisionConfig::default());
        
        let elements = vec![
            UIElement {
                bounds: Rectangle::new(0.0, 0.0, 10.0, 10.0),
                element_type: ElementType::Button,
                confidence: 0.8,
                properties: HashMap::new(),
            },
            UIElement {
                bounds: Rectangle::new(5.0, 5.0, 10.0, 10.0), // Overlaps with first
                element_type: ElementType::Button,
                confidence: 0.6,
                properties: HashMap::new(),
            },
            UIElement {
                bounds: Rectangle::new(20.0, 20.0, 10.0, 10.0), // No overlap
                element_type: ElementType::TextBox,
                confidence: 0.7,
                properties: HashMap::new(),
            },
        ];
        
        let filtered = pipeline.filter_elements(elements);
        
        // Should keep the higher confidence overlapping element and the non-overlapping one
        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].confidence, 0.8); // Higher confidence button
        assert_eq!(filtered[1].element_type, ElementType::TextBox);
    }
}