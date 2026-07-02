// Rendering engine for overlay elements with minimal dependencies
// Custom implementation without heavy graphics libraries

use super::{OverlayElement, OverlayElementType, Color};
use crate::utils::geometry::{Point, Rectangle};
use crate::utils::image_processing::Image;
use std::collections::HashMap;

pub struct Renderer {
    canvas_width: usize,
    canvas_height: usize,
    font_cache: FontCache,
}

impl Renderer {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            canvas_width: width,
            canvas_height: height,
            font_cache: FontCache::new(),
        }
    }

    pub fn render_overlay(&self, elements: &[&OverlayElement]) -> Result<Image, RenderError> {
        // Create transparent canvas
        let mut canvas = Image::new(self.canvas_width, self.canvas_height, 4); // RGBA
        
        // Clear canvas with transparent pixels
        for y in 0..self.canvas_height {
            for x in 0..self.canvas_width {
                canvas.set_pixel(x, y, &[0, 0, 0, 0]); // Transparent
            }
        }

        // Render elements in order (first elements appear behind later ones)
        for element in elements {
            if element.visible {
                self.render_element(&mut canvas, element)?;
            }
        }

        Ok(canvas)
    }

    fn render_element(&self, canvas: &mut Image, element: &OverlayElement) -> Result<(), RenderError> {
        match &element.element_type {
            OverlayElementType::Highlight => {
                self.render_highlight(canvas, element)?;
            }
            OverlayElementType::Label => {
                self.render_label(canvas, element)?;
            }
            OverlayElementType::Border => {
                self.render_border(canvas, element)?;
            }
            OverlayElementType::Arrow => {
                self.render_arrow(canvas, element)?;
            }
            OverlayElementType::Circle => {
                self.render_circle(canvas, element)?;
            }
            OverlayElementType::Custom(_) => {
                // Custom elements can be implemented by extending this
                self.render_highlight(canvas, element)?; // Fallback to highlight
            }
        }

        Ok(())
    }

    fn render_highlight(&self, canvas: &mut Image, element: &OverlayElement) -> Result<(), RenderError> {
        // Draw semi-transparent filled rectangle
        self.fill_rectangle(canvas, &element.bounds, element.color)?;
        
        // Draw border
        let border_color = Color::rgba(element.color.r, element.color.g, element.color.b, 255);
        self.draw_rectangle_outline(canvas, &element.bounds, border_color, 2)?;
        
        // Draw text if present
        if let Some(ref text) = element.text {
            let text_pos = Point::new(
                element.bounds.x + 5.0,
                element.bounds.y - 5.0,
            );
            self.draw_text(canvas, text, text_pos, Color::rgb(255, 255, 255))?;
        }

        Ok(())
    }

    fn render_label(&self, canvas: &mut Image, element: &OverlayElement) -> Result<(), RenderError> {
        if let Some(ref text) = element.text {
            // Draw text background
            let bg_color = Color::rgba(0, 0, 0, 180); // Semi-transparent black
            self.fill_rectangle(canvas, &element.bounds, bg_color)?;
            
            // Draw text
            let text_pos = Point::new(element.bounds.x + 2.0, element.bounds.y + 2.0);
            self.draw_text(canvas, text, text_pos, element.color)?;
        }

        Ok(())
    }

    fn render_border(&self, canvas: &mut Image, element: &OverlayElement) -> Result<(), RenderError> {
        self.draw_rectangle_outline(canvas, &element.bounds, element.color, 3)?;
        Ok(())
    }

    fn render_arrow(&self, canvas: &mut Image, element: &OverlayElement) -> Result<(), RenderError> {
        // Extract arrow coordinates from properties
        let start_x = element.properties.get("start_x")
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(element.bounds.x);
        let start_y = element.properties.get("start_y")
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(element.bounds.y);
        let end_x = element.properties.get("end_x")
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(element.bounds.x + element.bounds.width);
        let end_y = element.properties.get("end_y")
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(element.bounds.y + element.bounds.height);

        let start = Point::new(start_x, start_y);
        let end = Point::new(end_x, end_y);

        self.draw_arrow(canvas, start, end, element.color)?;
        Ok(())
    }

    fn render_circle(&self, canvas: &mut Image, element: &OverlayElement) -> Result<(), RenderError> {
        // Extract circle properties
        let center_x = element.properties.get("center_x")
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(element.bounds.x + element.bounds.width / 2.0);
        let center_y = element.properties.get("center_y")
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(element.bounds.y + element.bounds.height / 2.0);
        let radius = element.properties.get("radius")
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(element.bounds.width / 2.0);

        let center = Point::new(center_x, center_y);
        self.draw_circle(canvas, center, radius, element.color)?;
        Ok(())
    }

    fn fill_rectangle(&self, canvas: &mut Image, rect: &Rectangle, color: Color) -> Result<(), RenderError> {
        let start_x = (rect.x as usize).min(canvas.width);
        let start_y = (rect.y as usize).min(canvas.height);
        let end_x = ((rect.x + rect.width) as usize).min(canvas.width);
        let end_y = ((rect.y + rect.height) as usize).min(canvas.height);

        for y in start_y..end_y {
            for x in start_x..end_x {
                let pixel = [color.r, color.g, color.b, color.a];
                self.blend_pixel(canvas, x, y, &pixel);
            }
        }

        Ok(())
    }

    fn draw_rectangle_outline(&self, canvas: &mut Image, rect: &Rectangle, color: Color, thickness: usize) -> Result<(), RenderError> {
        let pixel = [color.r, color.g, color.b, color.a];

        // Draw top and bottom edges
        for t in 0..thickness {
            self.draw_horizontal_line(canvas, rect.x as i32, (rect.x + rect.width) as i32, (rect.y + t as f64) as i32, &pixel);
            self.draw_horizontal_line(canvas, rect.x as i32, (rect.x + rect.width) as i32, (rect.y + rect.height - t as f64) as i32, &pixel);
        }

        // Draw left and right edges
        for t in 0..thickness {
            self.draw_vertical_line(canvas, (rect.x + t as f64) as i32, rect.y as i32, (rect.y + rect.height) as i32, &pixel);
            self.draw_vertical_line(canvas, (rect.x + rect.width - t as f64) as i32, rect.y as i32, (rect.y + rect.height) as i32, &pixel);
        }

        Ok(())
    }

    fn draw_line(&self, canvas: &mut Image, start: Point, end: Point, color: Color) -> Result<(), RenderError> {
        let pixel = [color.r, color.g, color.b, color.a];
        
        // Bresenham's line algorithm
        let x0 = start.x as i32;
        let y0 = start.y as i32;
        let x1 = end.x as i32;
        let y1 = end.y as i32;

        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        let mut x = x0;
        let mut y = y0;

        loop {
            if x >= 0 && x < canvas.width as i32 && y >= 0 && y < canvas.height as i32 {
                self.blend_pixel(canvas, x as usize, y as usize, &pixel);
            }

            if x == x1 && y == y1 { break; }

            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }

        Ok(())
    }

    fn draw_arrow(&self, canvas: &mut Image, start: Point, end: Point, color: Color) -> Result<(), RenderError> {
        // Draw main line
        self.draw_line(canvas, start, end, color)?;

        // Calculate arrowhead
        let angle = (end.y - start.y).atan2(end.x - start.x);
        let arrowhead_length = 15.0;
        let arrowhead_angle = 0.5; // radians

        let arrowhead1 = Point::new(
            end.x - arrowhead_length * (angle - arrowhead_angle).cos(),
            end.y - arrowhead_length * (angle - arrowhead_angle).sin(),
        );

        let arrowhead2 = Point::new(
            end.x - arrowhead_length * (angle + arrowhead_angle).cos(),
            end.y - arrowhead_length * (angle + arrowhead_angle).sin(),
        );

        // Draw arrowhead
        self.draw_line(canvas, end, arrowhead1, color)?;
        self.draw_line(canvas, end, arrowhead2, color)?;

        Ok(())
    }

    fn draw_circle(&self, canvas: &mut Image, center: Point, radius: f64, color: Color) -> Result<(), RenderError> {
        let pixel = [color.r, color.g, color.b, color.a];
        
        // Midpoint circle algorithm
        let cx = center.x as i32;
        let cy = center.y as i32;
        let r = radius as i32;

        let mut x = r;
        let mut y = 0;
        let mut err = 0;

        while x >= y {
            self.plot_circle_points(canvas, cx, cy, x, y, &pixel);
            
            if err <= 0 {
                y += 1;
                err += 2 * y + 1;
            }
            
            if err > 0 {
                x -= 1;
                err -= 2 * x + 1;
            }
        }

        Ok(())
    }

    fn plot_circle_points(&self, canvas: &mut Image, cx: i32, cy: i32, x: i32, y: i32, pixel: &[u8; 4]) {
        let points = [
            (cx + x, cy + y), (cx - x, cy + y),
            (cx + x, cy - y), (cx - x, cy - y),
            (cx + y, cy + x), (cx - y, cy + x),
            (cx + y, cy - x), (cx - y, cy - x),
        ];

        for (px, py) in points {
            if px >= 0 && px < canvas.width as i32 && py >= 0 && py < canvas.height as i32 {
                self.blend_pixel(canvas, px as usize, py as usize, pixel);
            }
        }
    }

    fn draw_text(&self, canvas: &mut Image, text: &str, position: Point, color: Color) -> Result<(), RenderError> {
        // Simple bitmap font rendering
        let char_width = 8;
        let char_height = 12;
        let mut x_offset = 0;

        for ch in text.chars() {
            if let Some(bitmap) = self.font_cache.get_character_bitmap(ch) {
                self.draw_character_bitmap(
                    canvas,
                    &bitmap,
                    Point::new(position.x + x_offset as f64, position.y),
                    color,
                )?;
            }
            x_offset += char_width;
        }

        Ok(())
    }

    fn draw_character_bitmap(&self, canvas: &mut Image, bitmap: &CharacterBitmap, position: Point, color: Color) -> Result<(), RenderError> {
        let pixel = [color.r, color.g, color.b, color.a];
        let start_x = position.x as i32;
        let start_y = position.y as i32;

        for y in 0..bitmap.height {
            for x in 0..bitmap.width {
                if bitmap.pixels[y * bitmap.width + x] {
                    let px = start_x + x as i32;
                    let py = start_y + y as i32;
                    
                    if px >= 0 && px < canvas.width as i32 && py >= 0 && py < canvas.height as i32 {
                        self.blend_pixel(canvas, px as usize, py as usize, &pixel);
                    }
                }
            }
        }

        Ok(())
    }

    fn draw_horizontal_line(&self, canvas: &mut Image, x1: i32, x2: i32, y: i32, pixel: &[u8; 4]) {
        if y < 0 || y >= canvas.height as i32 { return; }
        
        let start_x = x1.max(0) as usize;
        let end_x = x2.min(canvas.width as i32) as usize;
        
        for x in start_x..end_x {
            self.blend_pixel(canvas, x, y as usize, pixel);
        }
    }

    fn draw_vertical_line(&self, canvas: &mut Image, x: i32, y1: i32, y2: i32, pixel: &[u8; 4]) {
        if x < 0 || x >= canvas.width as i32 { return; }
        
        let start_y = y1.max(0) as usize;
        let end_y = y2.min(canvas.height as i32) as usize;
        
        for y in start_y..end_y {
            self.blend_pixel(canvas, x as usize, y, pixel);
        }
    }

    fn blend_pixel(&self, canvas: &mut Image, x: usize, y: usize, new_pixel: &[u8; 4]) {
        if let Some(existing_pixel) = canvas.get_pixel(x, y) {
            if existing_pixel.len() >= 4 {
                let alpha = new_pixel[3] as f64 / 255.0;
                let inv_alpha = 1.0 - alpha;

                let blended = [
                    (new_pixel[0] as f64 * alpha + existing_pixel[0] as f64 * inv_alpha) as u8,
                    (new_pixel[1] as f64 * alpha + existing_pixel[1] as f64 * inv_alpha) as u8,
                    (new_pixel[2] as f64 * alpha + existing_pixel[2] as f64 * inv_alpha) as u8,
                    (new_pixel[3] as f64 + existing_pixel[3] as f64 * inv_alpha).min(255.0) as u8,
                ];

                canvas.set_pixel(x, y, &blended);
            }
        }
    }
}

// Simple bitmap font system
struct FontCache {
    character_bitmaps: HashMap<char, CharacterBitmap>,
}

#[derive(Debug, Clone)]
struct CharacterBitmap {
    width: usize,
    height: usize,
    pixels: Vec<bool>,
}

impl FontCache {
    fn new() -> Self {
        let mut cache = Self {
            character_bitmaps: HashMap::new(),
        };
        cache.initialize_basic_font();
        cache
    }

    fn get_character_bitmap(&self, ch: char) -> Option<&CharacterBitmap> {
        self.character_bitmaps.get(&ch)
    }

    fn initialize_basic_font(&mut self) {
        // Create basic 8x12 bitmap font for common characters
        // This is a simplified font - in a real implementation you'd load from a font file

        // Letter 'A'
        let a_bitmap = CharacterBitmap {
            width: 8,
            height: 12,
            pixels: vec![
                false, false, false, true,  true,  false, false, false,
                false, false, true,  false, false, true,  false, false,
                false, false, true,  false, false, true,  false, false,
                false, true,  false, false, false, false, true,  false,
                false, true,  false, false, false, false, true,  false,
                false, true,  true,  true,  true,  true,  true,  false,
                false, true,  false, false, false, false, true,  false,
                true,  false, false, false, false, false, false, true,
                true,  false, false, false, false, false, false, true,
                true,  false, false, false, false, false, false, true,
                false, false, false, false, false, false, false, false,
                false, false, false, false, false, false, false, false,
            ],
        };
        
        self.character_bitmaps.insert('A', a_bitmap);

        // Add space character
        let space_bitmap = CharacterBitmap {
            width: 8,
            height: 12,
            pixels: vec![false; 8 * 12],
        };
        
        self.character_bitmaps.insert(' ', space_bitmap);

        // Add more characters as needed...
        // For a complete implementation, you'd want the full ASCII set
        
        // Simple fallback for unknown characters
        let unknown_bitmap = CharacterBitmap {
            width: 8,
            height: 12,
            pixels: vec![
                true,  true,  true,  true,  true,  true,  true,  true,
                true,  false, false, false, false, false, false, true,
                true,  false, false, false, false, false, false, true,
                true,  false, false, false, false, false, false, true,
                true,  false, false, false, false, false, false, true,
                true,  false, false, false, false, false, false, true,
                true,  false, false, false, false, false, false, true,
                true,  false, false, false, false, false, false, true,
                true,  false, false, false, false, false, false, true,
                true,  true,  true,  true,  true,  true,  true,  true,
                false, false, false, false, false, false, false, false,
                false, false, false, false, false, false, false, false,
            ],
        };
        
        self.character_bitmaps.insert('?', unknown_bitmap);
    }
}

#[derive(Debug)]
pub enum RenderError {
    InvalidDimensions,
    FontError(String),
    DrawingError(String),
}

impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RenderError::InvalidDimensions => write!(f, "Invalid canvas dimensions"),
            RenderError::FontError(msg) => write!(f, "Font error: {}", msg),
            RenderError::DrawingError(msg) => write!(f, "Drawing error: {}", msg),
        }
    }
}

impl std::error::Error for RenderError {}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::{OverlayElement, OverlayElementType};

    #[test]
    fn test_renderer_creation() {
        let renderer = Renderer::new(800, 600);
        assert_eq!(renderer.canvas_width, 800);
        assert_eq!(renderer.canvas_height, 600);
    }

    #[test]
    fn test_render_empty_overlay() {
        let renderer = Renderer::new(100, 100);
        let elements: Vec<&OverlayElement> = vec![];
        
        let result = renderer.render_overlay(&elements);
        assert!(result.is_ok());
        
        let canvas = result.unwrap();
        assert_eq!(canvas.width, 100);
        assert_eq!(canvas.height, 100);
        assert_eq!(canvas.channels, 4); // RGBA
    }

    #[test]
    fn test_fill_rectangle() {
        let renderer = Renderer::new(100, 100);
        let mut canvas = Image::new(100, 100, 4);
        let rect = Rectangle::new(10.0, 10.0, 20.0, 20.0);
        let color = Color::rgb(255, 0, 0);

        let result = renderer.fill_rectangle(&mut canvas, &rect, color);
        assert!(result.is_ok());

        // Check that pixels in the rectangle have been set
        if let Some(pixel) = canvas.get_pixel(15, 15) {
            assert_eq!(pixel[0], 255); // Red channel
            assert_eq!(pixel[1], 0);   // Green channel
            assert_eq!(pixel[2], 0);   // Blue channel
        }
    }

    #[test]
    fn test_draw_line() {
        let renderer = Renderer::new(100, 100);
        let mut canvas = Image::new(100, 100, 4);
        let start = Point::new(0.0, 0.0);
        let end = Point::new(10.0, 10.0);
        let color = Color::rgb(0, 255, 0);

        let result = renderer.draw_line(&mut canvas, start, end, color);
        assert!(result.is_ok());

        // Check that the start and end points have been set
        if let Some(pixel) = canvas.get_pixel(0, 0) {
            assert_eq!(pixel[1], 255); // Green channel at start
        }
        if let Some(pixel) = canvas.get_pixel(10, 10) {
            assert_eq!(pixel[1], 255); // Green channel at end
        }
    }

    #[test]
    fn test_font_cache() {
        let font_cache = FontCache::new();
        
        // Should have basic characters
        assert!(font_cache.get_character_bitmap('A').is_some());
        assert!(font_cache.get_character_bitmap(' ').is_some());
        assert!(font_cache.get_character_bitmap('?').is_some());
    }

    #[test]
    fn test_color_blending() {
        let renderer = Renderer::new(10, 10);
        let mut canvas = Image::new(10, 10, 4);
        
        // Set initial pixel to semi-transparent red
        canvas.set_pixel(5, 5, &[255, 0, 0, 128]);
        
        // Blend with semi-transparent blue
        let blue_pixel = [0, 0, 255, 128];
        renderer.blend_pixel(&mut canvas, 5, 5, &blue_pixel);
        
        // Result should be a blend of red and blue
        if let Some(pixel) = canvas.get_pixel(5, 5) {
            assert!(pixel[0] > 0); // Should have some red
            assert!(pixel[2] > 0); // Should have some blue
        }
    }
}