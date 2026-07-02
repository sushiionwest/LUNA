// Text recognition and OCR functionality with minimal dependencies
// Custom implementation for text extraction from images

use crate::utils::geometry::{Point, Rectangle};
use crate::utils::image_processing::{Image, sobel_edge_detection, threshold, find_connected_components};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct TextRegion {
    pub bounds: Rectangle,
    pub text: String,
    pub confidence: f64,
    pub language: Option<String>,
    pub font_size: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct CharacterCandidate {
    pub bounds: Rectangle,
    pub character: char,
    pub confidence: f64,
}

pub struct TextRecognizer {
    character_templates: HashMap<char, Vec<Pattern>>,
    min_char_width: usize,
    max_char_width: usize,
    min_char_height: usize,
    max_char_height: usize,
}

#[derive(Debug, Clone)]
struct Pattern {
    width: usize,
    height: usize,
    pixels: Vec<bool>,
}

impl TextRecognizer {
    pub fn new() -> Self {
        let mut recognizer = Self {
            character_templates: HashMap::new(),
            min_char_width: 5,
            max_char_width: 50,
            min_char_height: 8,
            max_char_height: 80,
        };
        
        recognizer.initialize_basic_templates();
        recognizer
    }

    pub fn recognize_text(&self, image: &Image) -> Result<Vec<TextRegion>, TextRecognitionError> {
        // Convert to grayscale and preprocess
        let gray = image.to_grayscale();
        let processed = self.preprocess_for_ocr(&gray);
        
        // Find text lines
        let text_lines = self.find_text_lines(&processed)?;
        
        let mut regions = Vec::new();
        
        for line in text_lines {
            // Extract characters from each line
            let characters = self.extract_characters_from_line(&processed, &line)?;
            
            // Recognize each character
            let mut recognized_text = String::new();
            let mut total_confidence = 0.0;
            
            for char_region in characters {
                if let Ok(char_candidate) = self.recognize_character(&processed, &char_region) {
                    recognized_text.push(char_candidate.character);
                    total_confidence += char_candidate.confidence;
                }
            }
            
            if !recognized_text.is_empty() {
                let avg_confidence = total_confidence / recognized_text.len() as f64;
                
                regions.push(TextRegion {
                    bounds: line,
                    text: recognized_text,
                    confidence: avg_confidence,
                    language: None,
                    font_size: Some(line.height),
                });
            }
        }
        
        Ok(regions)
    }

    pub fn recognize_text_in_region(&self, image: &Image, region: &Rectangle) -> Result<TextRegion, TextRecognitionError> {
        let cropped = image.crop(region);
        let mut results = self.recognize_text(&cropped)?;
        
        if results.is_empty() {
            return Ok(TextRegion {
                bounds: *region,
                text: String::new(),
                confidence: 0.0,
                language: None,
                font_size: None,
            });
        }
        
        // Merge all recognized text from the region
        let combined_text: String = results.iter().map(|r| &r.text).collect::<Vec<_>>().join(" ");
        let avg_confidence = results.iter().map(|r| r.confidence).sum::<f64>() / results.len() as f64;
        
        Ok(TextRegion {
            bounds: *region,
            text: combined_text,
            confidence: avg_confidence,
            language: None,
            font_size: results.first().and_then(|r| r.font_size),
        })
    }

    fn preprocess_for_ocr(&self, image: &Image) -> Image {
        // Apply threshold to create binary image
        let binary = threshold(image, 128);
        
        // TODO: Could add more preprocessing like:
        // - Noise reduction
        // - Skew correction
        // - Contrast enhancement
        
        binary
    }

    fn find_text_lines(&self, binary: &Image) -> Result<Vec<Rectangle>, TextRecognitionError> {
        let mut text_lines = Vec::new();
        
        // Use horizontal projection to find text lines
        let horizontal_projection = self.calculate_horizontal_projection(binary);
        
        // Find runs of text pixels
        let mut in_text_line = false;
        let mut line_start = 0;
        
        for (y, &pixel_count) in horizontal_projection.iter().enumerate() {
            let has_text = pixel_count > binary.width / 20; // Threshold for text presence
            
            if has_text && !in_text_line {
                // Start of new text line
                line_start = y;
                in_text_line = true;
            } else if !has_text && in_text_line {
                // End of text line
                let line_height = y - line_start;
                
                if line_height >= self.min_char_height && line_height <= self.max_char_height {
                    // Find the horizontal bounds of this text line
                    if let Some(line_bounds) = self.find_line_horizontal_bounds(binary, line_start, y) {
                        text_lines.push(line_bounds);
                    }
                }
                
                in_text_line = false;
            }
        }
        
        // Handle case where text line extends to the bottom of the image
        if in_text_line {
            let line_height = binary.height - line_start;
            if line_height >= self.min_char_height && line_height <= self.max_char_height {
                if let Some(line_bounds) = self.find_line_horizontal_bounds(binary, line_start, binary.height) {
                    text_lines.push(line_bounds);
                }
            }
        }
        
        Ok(text_lines)
    }

    fn calculate_horizontal_projection(&self, binary: &Image) -> Vec<usize> {
        let mut projection = vec![0; binary.height];
        
        for y in 0..binary.height {
            for x in 0..binary.width {
                if let Some(pixel) = binary.get_pixel(x, y) {
                    if pixel[0] > 0 {
                        projection[y] += 1;
                    }
                }
            }
        }
        
        projection
    }

    fn find_line_horizontal_bounds(&self, binary: &Image, start_y: usize, end_y: usize) -> Option<Rectangle> {
        let mut min_x = binary.width;
        let mut max_x = 0;
        
        for y in start_y..end_y {
            for x in 0..binary.width {
                if let Some(pixel) = binary.get_pixel(x, y) {
                    if pixel[0] > 0 {
                        min_x = min_x.min(x);
                        max_x = max_x.max(x);
                    }
                }
            }
        }
        
        if min_x < max_x {
            Some(Rectangle::new(
                min_x as f64,
                start_y as f64,
                (max_x - min_x) as f64,
                (end_y - start_y) as f64,
            ))
        } else {
            None
        }
    }

    fn extract_characters_from_line(&self, binary: &Image, line: &Rectangle) -> Result<Vec<Rectangle>, TextRecognitionError> {
        let line_image = binary.crop(line);
        
        // Use vertical projection to find character boundaries
        let vertical_projection = self.calculate_vertical_projection(&line_image);
        
        let mut characters = Vec::new();
        let mut in_character = false;
        let mut char_start = 0;
        
        for (x, &pixel_count) in vertical_projection.iter().enumerate() {
            let has_text = pixel_count > line_image.height / 10; // Threshold for character presence
            
            if has_text && !in_character {
                // Start of new character
                char_start = x;
                in_character = true;
            } else if !has_text && in_character {
                // End of character
                let char_width = x - char_start;
                
                if char_width >= self.min_char_width && char_width <= self.max_char_width {
                    characters.push(Rectangle::new(
                        (line.x + char_start as f64),
                        line.y,
                        char_width as f64,
                        line.height,
                    ));
                }
                
                in_character = false;
            }
        }
        
        // Handle case where character extends to the end of the line
        if in_character {
            let char_width = line_image.width - char_start;
            if char_width >= self.min_char_width && char_width <= self.max_char_width {
                characters.push(Rectangle::new(
                    (line.x + char_start as f64),
                    line.y,
                    char_width as f64,
                    line.height,
                ));
            }
        }
        
        Ok(characters)
    }

    fn calculate_vertical_projection(&self, binary: &Image) -> Vec<usize> {
        let mut projection = vec![0; binary.width];
        
        for x in 0..binary.width {
            for y in 0..binary.height {
                if let Some(pixel) = binary.get_pixel(x, y) {
                    if pixel[0] > 0 {
                        projection[x] += 1;
                    }
                }
            }
        }
        
        projection
    }

    fn recognize_character(&self, image: &Image, bounds: &Rectangle) -> Result<CharacterCandidate, TextRecognitionError> {
        let char_image = image.crop(bounds);
        
        let mut best_match = ' ';
        let mut best_confidence = 0.0;
        
        // Try to match against known character templates
        for (&character, templates) in &self.character_templates {
            for template in templates {
                let confidence = self.match_template(&char_image, template);
                
                if confidence > best_confidence {
                    best_confidence = confidence;
                    best_match = character;
                }
            }
        }
        
        // If confidence is too low, try some heuristics
        if best_confidence < 0.3 {
            best_match = self.guess_character_by_features(&char_image);
            best_confidence = 0.2; // Low confidence for guessed characters
        }
        
        Ok(CharacterCandidate {
            bounds: *bounds,
            character: best_match,
            confidence: best_confidence,
        })
    }

    fn match_template(&self, image: &Image, template: &Pattern) -> f64 {
        // Resize image to match template size for comparison
        let resized = image.resize(template.width, template.height);
        let binary = threshold(&resized, 128);
        
        let mut matching_pixels = 0;
        let mut total_pixels = 0;
        
        for y in 0..template.height {
            for x in 0..template.width {
                if let Some(pixel) = binary.get_pixel(x, y) {
                    let image_has_pixel = pixel[0] > 0;
                    let template_has_pixel = template.pixels[y * template.width + x];
                    
                    if image_has_pixel == template_has_pixel {
                        matching_pixels += 1;
                    }
                    total_pixels += 1;
                }
            }
        }
        
        matching_pixels as f64 / total_pixels as f64
    }

    fn guess_character_by_features(&self, image: &Image) -> char {
        // Simple feature-based character recognition
        let aspect_ratio = image.width as f64 / image.height as f64;
        let pixel_density = self.calculate_pixel_density(image);
        
        // Very basic heuristics
        if aspect_ratio < 0.3 {
            if pixel_density > 0.7 { 'l' } else { 'i' }
        } else if aspect_ratio > 1.5 {
            if pixel_density > 0.4 { 'm' } else { '-' }
        } else if pixel_density < 0.2 {
            '.'
        } else if pixel_density > 0.6 {
            'O'
        } else {
            'a' // Default guess
        }
    }

    fn calculate_pixel_density(&self, image: &Image) -> f64 {
        let binary = threshold(image, 128);
        let mut pixel_count = 0;
        let total_pixels = binary.width * binary.height;
        
        for y in 0..binary.height {
            for x in 0..binary.width {
                if let Some(pixel) = binary.get_pixel(x, y) {
                    if pixel[0] > 0 {
                        pixel_count += 1;
                    }
                }
            }
        }
        
        pixel_count as f64 / total_pixels as f64
    }

    fn initialize_basic_templates(&mut self) {
        // Create very basic templates for common characters
        // In a real implementation, these would be loaded from a font or trained data
        
        // Template for 'A'
        let a_pattern = Pattern {
            width: 8,
            height: 12,
            pixels: vec![
                false, false, true,  true,  true,  true,  false, false,
                false, true,  false, false, false, false, true,  false,
                false, true,  false, false, false, false, true,  false,
                false, true,  false, false, false, false, true,  false,
                false, true,  true,  true,  true,  true,  true,  false,
                false, true,  false, false, false, false, true,  false,
                false, true,  false, false, false, false, true,  false,
                false, true,  false, false, false, false, true,  false,
                false, true,  false, false, false, false, true,  false,
                false, true,  false, false, false, false, true,  false,
                false, false, false, false, false, false, false, false,
                false, false, false, false, false, false, false, false,
            ],
        };
        
        self.character_templates.entry('A').or_insert_with(Vec::new).push(a_pattern);
        
        // Template for 'O'
        let o_pattern = Pattern {
            width: 8,
            height: 12,
            pixels: vec![
                false, false, true,  true,  true,  true,  false, false,
                false, true,  false, false, false, false, true,  false,
                false, true,  false, false, false, false, true,  false,
                false, true,  false, false, false, false, true,  false,
                false, true,  false, false, false, false, true,  false,
                false, true,  false, false, false, false, true,  false,
                false, true,  false, false, false, false, true,  false,
                false, true,  false, false, false, false, true,  false,
                false, true,  false, false, false, false, true,  false,
                false, false, true,  true,  true,  true,  false, false,
                false, false, false, false, false, false, false, false,
                false, false, false, false, false, false, false, false,
            ],
        };
        
        self.character_templates.entry('O').or_insert_with(Vec::new).push(o_pattern);
        
        // Add more basic templates as needed...
        // For a full implementation, you would want templates for all alphanumeric characters
    }

    pub fn add_character_template(&mut self, character: char, pattern: Pattern) {
        self.character_templates.entry(character).or_insert_with(Vec::new).push(pattern);
    }
}

impl Default for TextRecognizer {
    fn default() -> Self {
        Self::new()
    }
}

// Utility functions for text processing
pub fn extract_text_from_image(image: &Image) -> Result<String, TextRecognitionError> {
    let recognizer = TextRecognizer::new();
    let regions = recognizer.recognize_text(image)?;
    
    Ok(regions.into_iter()
        .map(|region| region.text)
        .collect::<Vec<_>>()
        .join(" "))
}

pub fn find_text_containing(image: &Image, search_text: &str) -> Result<Vec<TextRegion>, TextRecognitionError> {
    let recognizer = TextRecognizer::new();
    let regions = recognizer.recognize_text(image)?;
    
    Ok(regions.into_iter()
        .filter(|region| region.text.to_lowercase().contains(&search_text.to_lowercase()))
        .collect())
}

pub fn get_text_at_point(image: &Image, point: &Point) -> Result<Option<String>, TextRecognitionError> {
    let recognizer = TextRecognizer::new();
    let regions = recognizer.recognize_text(image)?;
    
    for region in regions {
        if region.bounds.contains_point(point) {
            return Ok(Some(region.text));
        }
    }
    
    Ok(None)
}

#[derive(Debug)]
pub enum TextRecognitionError {
    ImageProcessingError(String),
    TemplateError(String),
    RecognitionError(String),
}

impl std::fmt::Display for TextRecognitionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TextRecognitionError::ImageProcessingError(msg) => write!(f, "Image processing error: {}", msg),
            TextRecognitionError::TemplateError(msg) => write!(f, "Template error: {}", msg),
            TextRecognitionError::RecognitionError(msg) => write!(f, "Recognition error: {}", msg),
        }
    }
}

impl std::error::Error for TextRecognitionError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_recognizer_creation() {
        let recognizer = TextRecognizer::new();
        assert!(!recognizer.character_templates.is_empty());
    }

    #[test]
    fn test_horizontal_projection() {
        let recognizer = TextRecognizer::new();
        let mut image = Image::new(10, 5, 1);
        
        // Add some pixels in the middle row
        for x in 0..10 {
            image.set_pixel(x, 2, &[255]);
        }
        
        let projection = recognizer.calculate_horizontal_projection(&image);
        assert_eq!(projection[2], 10);
        assert_eq!(projection[0], 0);
        assert_eq!(projection[4], 0);
    }

    #[test]
    fn test_vertical_projection() {
        let recognizer = TextRecognizer::new();
        let mut image = Image::new(5, 10, 1);
        
        // Add some pixels in the middle column
        for y in 0..10 {
            image.set_pixel(2, y, &[255]);
        }
        
        let projection = recognizer.calculate_vertical_projection(&image);
        assert_eq!(projection[2], 10);
        assert_eq!(projection[0], 0);
        assert_eq!(projection[4], 0);
    }

    #[test]
    fn test_pixel_density_calculation() {
        let recognizer = TextRecognizer::new();
        let mut image = Image::new(10, 10, 1);
        
        // Fill half the image with white pixels
        for y in 0..5 {
            for x in 0..10 {
                image.set_pixel(x, y, &[255]);
            }
        }
        
        let density = recognizer.calculate_pixel_density(&image);
        assert!((density - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_extract_text_from_image() {
        let image = Image::new(100, 50, 1);
        let result = extract_text_from_image(&image);
        assert!(result.is_ok());
    }

    #[test]
    fn test_character_template_matching() {
        let recognizer = TextRecognizer::new();
        
        // Create a simple test image
        let test_image = Image::new(8, 12, 1);
        
        // Get the 'A' template for testing
        if let Some(templates) = recognizer.character_templates.get(&'A') {
            if let Some(template) = templates.first() {
                let confidence = recognizer.match_template(&test_image, template);
                assert!(confidence >= 0.0 && confidence <= 1.0);
            }
        }
    }
}