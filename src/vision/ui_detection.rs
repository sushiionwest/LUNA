// UI element detection with specialized algorithms
// Advanced detection for different UI component types

use crate::utils::geometry::{Point, Rectangle};
use crate::utils::image_processing::{Image, sobel_edge_detection, threshold, gaussian_blur};
use super::{UIElement, ElementType, VisionError};
use std::collections::HashMap;

pub struct UIDetector {
    button_detector: ButtonDetector,
    text_detector: TextDetector,
    window_detector: WindowDetector,
    menu_detector: MenuDetector,
}

impl UIDetector {
    pub fn new() -> Self {
        Self {
            button_detector: ButtonDetector::new(),
            text_detector: TextDetector::new(),
            window_detector: WindowDetector::new(),
            menu_detector: MenuDetector::new(),
        }
    }

    pub fn detect_all_elements(&self, image: &Image) -> Result<Vec<UIElement>, VisionError> {
        let mut elements = Vec::new();

        // Detect different element types
        elements.extend(self.button_detector.detect(image)?);
        elements.extend(self.text_detector.detect(image)?);
        elements.extend(self.window_detector.detect(image)?);
        elements.extend(self.menu_detector.detect(image)?);

        // Sort by confidence
        elements.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());

        Ok(elements)
    }

    pub fn detect_buttons(&self, image: &Image) -> Result<Vec<UIElement>, VisionError> {
        self.button_detector.detect(image)
    }

    pub fn detect_text_elements(&self, image: &Image) -> Result<Vec<UIElement>, VisionError> {
        self.text_detector.detect(image)
    }

    pub fn detect_windows(&self, image: &Image) -> Result<Vec<UIElement>, VisionError> {
        self.window_detector.detect(image)
    }

    pub fn detect_menus(&self, image: &Image) -> Result<Vec<UIElement>, VisionError> {
        self.menu_detector.detect(image)
    }
}

impl Default for UIDetector {
    fn default() -> Self {
        Self::new()
    }
}

// Button detection specialized for button-like UI elements
pub struct ButtonDetector {
    min_width: f64,
    max_width: f64,
    min_height: f64,
    max_height: f64,
    edge_threshold: u8,
}

impl ButtonDetector {
    pub fn new() -> Self {
        Self {
            min_width: 50.0,
            max_width: 300.0,
            min_height: 20.0,
            max_height: 80.0,
            edge_threshold: 60,
        }
    }

    pub fn detect(&self, image: &Image) -> Result<Vec<UIElement>, VisionError> {
        let mut buttons = Vec::new();
        
        // Preprocess image
        let gray = image.to_grayscale();
        let blurred = gaussian_blur(&gray, 1);
        let edges = sobel_edge_detection(&blurred);
        
        // Find rectangular regions with strong edges
        let candidates = self.find_button_candidates(&edges)?;
        
        for candidate in candidates {
            if self.is_valid_button_size(&candidate) {
                let confidence = self.calculate_button_confidence(image, &candidate);
                
                if confidence > 0.5 {
                    let properties = self.extract_button_properties(image, &candidate);
                    
                    buttons.push(UIElement {
                        bounds: candidate,
                        element_type: ElementType::Button,
                        confidence,
                        properties,
                    });
                }
            }
        }
        
        Ok(buttons)
    }

    fn find_button_candidates(&self, edges: &Image) -> Result<Vec<Rectangle>, VisionError> {
        let binary = threshold(edges, self.edge_threshold);
        let mut candidates = Vec::new();
        
        // Use a sliding window approach to find rectangular regions
        let window_sizes = [(50, 30), (80, 25), (100, 35), (120, 40)];
        
        for &(w, h) in &window_sizes {
            for y in 0..binary.height.saturating_sub(h) {
                for x in 0..binary.width.saturating_sub(w) {
                    let rect = Rectangle::new(x as f64, y as f64, w as f64, h as f64);
                    
                    if self.has_button_like_edges(&binary, &rect) {
                        candidates.push(rect);
                    }
                }
            }
        }
        
        // Remove overlapping candidates
        candidates = self.remove_overlapping_rectangles(candidates);
        
        Ok(candidates)
    }

    fn has_button_like_edges(&self, binary: &Image, rect: &Rectangle) -> bool {
        let mut edge_counts = [0; 4]; // top, right, bottom, left
        
        // Check top and bottom edges
        for x in rect.x as usize..(rect.x + rect.width) as usize {
            if x < binary.width {
                // Top edge
                if let Some(pixel) = binary.get_pixel(x, rect.y as usize) {
                    if pixel[0] > 0 { edge_counts[0] += 1; }
                }
                // Bottom edge
                if let Some(pixel) = binary.get_pixel(x, (rect.y + rect.height) as usize) {
                    if pixel[0] > 0 { edge_counts[2] += 1; }
                }
            }
        }
        
        // Check left and right edges
        for y in rect.y as usize..(rect.y + rect.height) as usize {
            if y < binary.height {
                // Left edge
                if let Some(pixel) = binary.get_pixel(rect.x as usize, y) {
                    if pixel[0] > 0 { edge_counts[3] += 1; }
                }
                // Right edge
                if let Some(pixel) = binary.get_pixel((rect.x + rect.width) as usize, y) {
                    if pixel[0] > 0 { edge_counts[1] += 1; }
                }
            }
        }
        
        // Button should have edges on all sides
        let min_edge_ratio = 0.3;
        let width_edges = edge_counts[0] as f64 / rect.width;
        let height_edges = edge_counts[3] as f64 / rect.height;
        
        width_edges > min_edge_ratio && height_edges > min_edge_ratio
    }

    fn is_valid_button_size(&self, rect: &Rectangle) -> bool {
        rect.width >= self.min_width && rect.width <= self.max_width &&
        rect.height >= self.min_height && rect.height <= self.max_height
    }

    fn calculate_button_confidence(&self, image: &Image, rect: &Rectangle) -> f64 {
        let roi = image.crop(rect);
        
        // Calculate various features
        let uniformity = self.calculate_color_uniformity(&roi);
        let contrast = self.calculate_edge_contrast(&roi);
        let aspect_ratio = rect.width / rect.height;
        
        // Ideal button characteristics
        let aspect_score = if aspect_ratio > 1.5 && aspect_ratio < 8.0 { 1.0 } else { 0.5 };
        let size_score = if rect.area() > 1000.0 && rect.area() < 15000.0 { 1.0 } else { 0.7 };
        
        (uniformity * 0.3 + contrast * 0.3 + aspect_score * 0.2 + size_score * 0.2).min(1.0)
    }

    fn calculate_color_uniformity(&self, image: &Image) -> f64 {
        let gray = image.to_grayscale();
        let mut brightness_histogram = vec![0; 256];
        
        for y in 0..gray.height {
            for x in 0..gray.width {
                if let Some(pixel) = gray.get_pixel(x, y) {
                    brightness_histogram[pixel[0] as usize] += 1;
                }
            }
        }
        
        // Calculate entropy (lower entropy = more uniform)
        let total_pixels = gray.width * gray.height;
        let mut entropy = 0.0;
        
        for &count in &brightness_histogram {
            if count > 0 {
                let p = count as f64 / total_pixels as f64;
                entropy -= p * p.ln();
            }
        }
        
        // Normalize entropy (buttons should have low entropy)
        1.0 - (entropy / 8.0).min(1.0)
    }

    fn calculate_edge_contrast(&self, image: &Image) -> f64 {
        let edges = sobel_edge_detection(image);
        let mut edge_strength = 0.0;
        let mut count = 0;
        
        for y in 0..edges.height {
            for x in 0..edges.width {
                if let Some(pixel) = edges.get_pixel(x, y) {
                    edge_strength += pixel[0] as f64;
                    count += 1;
                }
            }
        }
        
        if count > 0 {
            (edge_strength / count as f64) / 255.0
        } else {
            0.0
        }
    }

    fn extract_button_properties(&self, image: &Image, rect: &Rectangle) -> HashMap<String, String> {
        let mut properties = HashMap::new();
        let roi = image.crop(rect);
        
        // Calculate average color
        let avg_color = self.calculate_average_color(&roi);
        properties.insert("avg_red".to_string(), avg_color.0.to_string());
        properties.insert("avg_green".to_string(), avg_color.1.to_string());
        properties.insert("avg_blue".to_string(), avg_color.2.to_string());
        
        // Button style classification
        let button_style = if avg_color.0 > 200 && avg_color.1 > 200 && avg_color.2 > 200 {
            "light"
        } else if avg_color.0 < 100 && avg_color.1 < 100 && avg_color.2 < 100 {
            "dark"
        } else {
            "medium"
        };
        properties.insert("style".to_string(), button_style.to_string());
        
        properties.insert("area".to_string(), rect.area().to_string());
        properties.insert("aspect_ratio".to_string(), (rect.width / rect.height).to_string());
        
        properties
    }

    fn calculate_average_color(&self, image: &Image) -> (u8, u8, u8) {
        if image.channels < 3 {
            return (128, 128, 128);
        }
        
        let mut sum_r = 0u64;
        let mut sum_g = 0u64;
        let mut sum_b = 0u64;
        let mut count = 0;
        
        for y in 0..image.height {
            for x in 0..image.width {
                if let Some(pixel) = image.get_pixel(x, y) {
                    sum_r += pixel[0] as u64;
                    sum_g += pixel[1] as u64;
                    sum_b += pixel[2] as u64;
                    count += 1;
                }
            }
        }
        
        if count > 0 {
            ((sum_r / count) as u8, (sum_g / count) as u8, (sum_b / count) as u8)
        } else {
            (128, 128, 128)
        }
    }

    fn remove_overlapping_rectangles(&self, mut rects: Vec<Rectangle>) -> Vec<Rectangle> {
        rects.sort_by(|a, b| b.area().partial_cmp(&a.area()).unwrap());
        let mut filtered = Vec::new();
        
        for rect in rects {
            let mut overlaps = false;
            
            for existing in &filtered {
                if let Some(intersection) = rect.intersection(existing) {
                    let overlap_ratio = intersection.area() / rect.area().min(existing.area());
                    if overlap_ratio > 0.3 {
                        overlaps = true;
                        break;
                    }
                }
            }
            
            if !overlaps {
                filtered.push(rect);
            }
        }
        
        filtered
    }
}

// Text element detection (labels, text boxes, etc.)
pub struct TextDetector {
    min_text_height: f64,
    max_text_height: f64,
    min_aspect_ratio: f64,
}

impl TextDetector {
    pub fn new() -> Self {
        Self {
            min_text_height: 10.0,
            max_text_height: 50.0,
            min_aspect_ratio: 2.0,
        }
    }

    pub fn detect(&self, image: &Image) -> Result<Vec<UIElement>, VisionError> {
        let mut text_elements = Vec::new();
        
        // Use edge detection to find text-like patterns
        let gray = image.to_grayscale();
        let edges = sobel_edge_detection(&gray);
        
        // Find horizontal text lines
        let text_regions = self.find_text_regions(&edges)?;
        
        for region in text_regions {
            let confidence = self.calculate_text_confidence(image, &region);
            
            if confidence > 0.4 {
                let element_type = if region.height < 25.0 && region.width / region.height > 5.0 {
                    ElementType::Label
                } else {
                    ElementType::TextBox
                };
                
                text_elements.push(UIElement {
                    bounds: region,
                    element_type,
                    confidence,
                    properties: HashMap::new(),
                });
            }
        }
        
        Ok(text_elements)
    }

    fn find_text_regions(&self, edges: &Image) -> Result<Vec<Rectangle>, VisionError> {
        let binary = threshold(edges, 30);
        let mut regions = Vec::new();
        
        // Scan for horizontal text patterns
        for y in 0..binary.height {
            let mut line_segments = Vec::new();
            let mut current_segment_start = None;
            
            for x in 0..binary.width {
                if let Some(pixel) = binary.get_pixel(x, y) {
                    if pixel[0] > 0 {
                        if current_segment_start.is_none() {
                            current_segment_start = Some(x);
                        }
                    } else if let Some(start) = current_segment_start {
                        if x - start > 20 { // Minimum text width
                            line_segments.push((start, x));
                        }
                        current_segment_start = None;
                    }
                }
            }
            
            // Close final segment if needed
            if let Some(start) = current_segment_start {
                if binary.width - start > 20 {
                    line_segments.push((start, binary.width));
                }
            }
            
            // Convert segments to rectangles
            for (start_x, end_x) in line_segments {
                let width = end_x - start_x;
                if width > 30 { // Minimum text width
                    // Find the height of this text line
                    let height = self.estimate_text_height(&binary, start_x, y, width);
                    
                    if height >= self.min_text_height && height <= self.max_text_height {
                        let rect = Rectangle::new(
                            start_x as f64,
                            (y as f64 - height / 2.0).max(0.0),
                            width as f64,
                            height,
                        );
                        regions.push(rect);
                    }
                }
            }
        }
        
        Ok(regions)
    }

    fn estimate_text_height(&self, binary: &Image, start_x: usize, center_y: usize, width: usize) -> f64 {
        let mut top_y = center_y;
        let mut bottom_y = center_y;
        
        // Search upward for text boundary
        for y in (0..center_y).rev() {
            let mut edge_count = 0;
            for x in start_x..start_x + width {
                if let Some(pixel) = binary.get_pixel(x, y) {
                    if pixel[0] > 0 {
                        edge_count += 1;
                    }
                }
            }
            
            if edge_count < width / 10 {
                top_y = y + 1;
                break;
            }
        }
        
        // Search downward for text boundary
        for y in center_y..binary.height {
            let mut edge_count = 0;
            for x in start_x..start_x + width {
                if let Some(pixel) = binary.get_pixel(x, y) {
                    if pixel[0] > 0 {
                        edge_count += 1;
                    }
                }
            }
            
            if edge_count < width / 10 {
                bottom_y = y - 1;
                break;
            }
        }
        
        (bottom_y - top_y) as f64
    }

    fn calculate_text_confidence(&self, image: &Image, rect: &Rectangle) -> f64 {
        let roi = image.crop(rect);
        let gray = roi.to_grayscale();
        
        // Text should have good horizontal structure
        let horizontal_consistency = self.measure_horizontal_consistency(&gray);
        
        // Text should have appropriate aspect ratio
        let aspect_ratio = rect.width / rect.height;
        let aspect_score = if aspect_ratio >= self.min_aspect_ratio { 1.0 } else { 0.5 };
        
        horizontal_consistency * 0.7 + aspect_score * 0.3
    }

    fn measure_horizontal_consistency(&self, image: &Image) -> f64 {
        let mut consistency_scores = Vec::new();
        
        for y in 0..image.height {
            let mut pixel_changes = 0;
            let mut last_brightness = 0;
            
            for x in 0..image.width {
                if let Some(pixel) = image.get_pixel(x, y) {
                    let brightness = pixel[0];
                    if x > 0 && (brightness as i32 - last_brightness as i32).abs() > 50 {
                        pixel_changes += 1;
                    }
                    last_brightness = brightness;
                }
            }
            
            // Text lines should have moderate number of changes
            let changes_ratio = pixel_changes as f64 / image.width as f64;
            if changes_ratio > 0.1 && changes_ratio < 0.8 {
                consistency_scores.push(1.0);
            } else {
                consistency_scores.push(0.0);
            }
        }
        
        if consistency_scores.is_empty() {
            0.0
        } else {
            consistency_scores.iter().sum::<f64>() / consistency_scores.len() as f64
        }
    }
}

// Window detection for application windows
pub struct WindowDetector {
    min_window_size: f64,
}

impl WindowDetector {
    pub fn new() -> Self {
        Self {
            min_window_size: 50000.0, // Minimum area for a window
        }
    }

    pub fn detect(&self, image: &Image) -> Result<Vec<UIElement>, VisionError> {
        let mut windows = Vec::new();
        
        // Windows typically have title bars and distinct rectangular boundaries
        let gray = image.to_grayscale();
        let edges = sobel_edge_detection(&gray);
        
        let candidates = self.find_window_candidates(&edges)?;
        
        for candidate in candidates {
            if candidate.area() >= self.min_window_size {
                let confidence = self.calculate_window_confidence(image, &candidate);
                
                if confidence > 0.4 {
                    windows.push(UIElement {
                        bounds: candidate,
                        element_type: ElementType::Window,
                        confidence,
                        properties: HashMap::new(),
                    });
                }
            }
        }
        
        Ok(windows)
    }

    fn find_window_candidates(&self, edges: &Image) -> Result<Vec<Rectangle>, VisionError> {
        // Look for large rectangular regions with strong top edges (title bars)
        let binary = threshold(edges, 40);
        let mut candidates = Vec::new();
        
        // Scan for horizontal lines that could be title bars
        for y in 0..binary.height.saturating_sub(100) {
            for x in 0..binary.width.saturating_sub(200) {
                // Check if this looks like a title bar start
                if self.has_title_bar_like_pattern(&binary, x, y) {
                    // Try to find the window bounds
                    if let Some(window_rect) = self.trace_window_bounds(&binary, x, y) {
                        candidates.push(window_rect);
                    }
                }
            }
        }
        
        Ok(candidates)
    }

    fn has_title_bar_like_pattern(&self, binary: &Image, x: usize, y: usize) -> bool {
        // Check for a horizontal line of edges (title bar)
        let mut edge_count = 0;
        let check_width = 100.min(binary.width - x);
        
        for check_x in x..x + check_width {
            if let Some(pixel) = binary.get_pixel(check_x, y) {
                if pixel[0] > 0 {
                    edge_count += 1;
                }
            }
        }
        
        edge_count as f64 / check_width as f64 > 0.5
    }

    fn trace_window_bounds(&self, binary: &Image, start_x: usize, start_y: usize) -> Option<Rectangle> {
        // Simple window boundary detection
        let mut width = 200; // Minimum window width
        let mut height = 150; // Minimum window height
        
        // Find right edge
        for x in start_x + 200..binary.width {
            if self.has_vertical_edge(&binary, x, start_y, 50) {
                width = x - start_x;
                break;
            }
        }
        
        // Find bottom edge
        for y in start_y + 150..binary.height {
            if self.has_horizontal_edge(&binary, start_x, y, width) {
                height = y - start_y;
                break;
            }
        }
        
        Some(Rectangle::new(start_x as f64, start_y as f64, width as f64, height as f64))
    }

    fn has_vertical_edge(&self, binary: &Image, x: usize, start_y: usize, height: usize) -> bool {
        let mut edge_count = 0;
        let check_height = height.min(binary.height - start_y);
        
        for y in start_y..start_y + check_height {
            if let Some(pixel) = binary.get_pixel(x, y) {
                if pixel[0] > 0 {
                    edge_count += 1;
                }
            }
        }
        
        edge_count as f64 / check_height as f64 > 0.3
    }

    fn has_horizontal_edge(&self, binary: &Image, start_x: usize, y: usize, width: usize) -> bool {
        let mut edge_count = 0;
        let check_width = width.min(binary.width - start_x);
        
        for x in start_x..start_x + check_width {
            if let Some(pixel) = binary.get_pixel(x, y) {
                if pixel[0] > 0 {
                    edge_count += 1;
                }
            }
        }
        
        edge_count as f64 / check_width as f64 > 0.3
    }

    fn calculate_window_confidence(&self, _image: &Image, rect: &Rectangle) -> f64 {
        // Windows should be reasonably large and have good aspect ratios
        let area_score = (rect.area() / 100000.0).min(1.0);
        let aspect_ratio = rect.width / rect.height;
        let aspect_score = if aspect_ratio > 0.5 && aspect_ratio < 3.0 { 1.0 } else { 0.5 };
        
        area_score * 0.6 + aspect_score * 0.4
    }
}

// Menu detection for dropdown menus, context menus, etc.
pub struct MenuDetector;

impl MenuDetector {
    pub fn new() -> Self {
        Self
    }

    pub fn detect(&self, image: &Image) -> Result<Vec<UIElement>, VisionError> {
        // Placeholder for menu detection
        // Menus typically have vertical lists of items with consistent spacing
        let mut menus = Vec::new();
        
        // For now, return empty vector
        // Real implementation would look for vertical arrangements of text/buttons
        
        Ok(menus)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ui_detector_creation() {
        let detector = UIDetector::new();
        // Just verify it can be created without panicking
        assert!(true);
    }

    #[test]
    fn test_button_detector() {
        let detector = ButtonDetector::new();
        let test_image = Image::new(100, 100, 3);
        
        let result = detector.detect(&test_image);
        assert!(result.is_ok());
    }

    #[test]
    fn test_text_detector() {
        let detector = TextDetector::new();
        let test_image = Image::new(100, 100, 3);
        
        let result = detector.detect(&test_image);
        assert!(result.is_ok());
    }

    #[test]
    fn test_button_size_validation() {
        let detector = ButtonDetector::new();
        
        let valid_button = Rectangle::new(0.0, 0.0, 100.0, 30.0);
        let too_small = Rectangle::new(0.0, 0.0, 20.0, 10.0);
        let too_large = Rectangle::new(0.0, 0.0, 500.0, 200.0);
        
        assert!(detector.is_valid_button_size(&valid_button));
        assert!(!detector.is_valid_button_size(&too_small));
        assert!(!detector.is_valid_button_size(&too_large));
    }
}