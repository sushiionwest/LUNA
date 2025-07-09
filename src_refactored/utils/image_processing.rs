// Basic image processing utilities with minimal dependencies
// Custom implementations for common computer vision operations

use super::geometry::{Point, Rectangle};

#[derive(Debug, Clone)]
pub struct Image {
    pub width: usize,
    pub height: usize,
    pub data: Vec<u8>,
    pub channels: usize,
}

impl Image {
    pub fn new(width: usize, height: usize, channels: usize) -> Self {
        Self {
            width,
            height,
            data: vec![0; width * height * channels],
            channels,
        }
    }

    pub fn from_rgb_data(width: usize, height: usize, data: Vec<u8>) -> Self {
        Self {
            width,
            height,
            data,
            channels: 3,
        }
    }

    pub fn from_rgba_data(width: usize, height: usize, data: Vec<u8>) -> Self {
        Self {
            width,
            height,
            data,
            channels: 4,
        }
    }

    pub fn get_pixel(&self, x: usize, y: usize) -> Option<&[u8]> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let index = (y * self.width + x) * self.channels;
        Some(&self.data[index..index + self.channels])
    }

    pub fn set_pixel(&mut self, x: usize, y: usize, pixel: &[u8]) {
        if x >= self.width || y >= self.height || pixel.len() != self.channels {
            return;
        }
        let index = (y * self.width + x) * self.channels;
        self.data[index..index + self.channels].copy_from_slice(pixel);
    }

    pub fn to_grayscale(&self) -> Image {
        let mut gray = Image::new(self.width, self.height, 1);
        
        for y in 0..self.height {
            for x in 0..self.width {
                if let Some(pixel) = self.get_pixel(x, y) {
                    let gray_value = match self.channels {
                        1 => pixel[0],
                        3 => rgb_to_gray(pixel[0], pixel[1], pixel[2]),
                        4 => rgb_to_gray(pixel[0], pixel[1], pixel[2]),
                        _ => 0,
                    };
                    gray.set_pixel(x, y, &[gray_value]);
                }
            }
        }
        
        gray
    }

    pub fn resize(&self, new_width: usize, new_height: usize) -> Image {
        let mut resized = Image::new(new_width, new_height, self.channels);
        
        let x_scale = self.width as f64 / new_width as f64;
        let y_scale = self.height as f64 / new_height as f64;
        
        for y in 0..new_height {
            for x in 0..new_width {
                let src_x = (x as f64 * x_scale) as usize;
                let src_y = (y as f64 * y_scale) as usize;
                
                if let Some(pixel) = self.get_pixel(src_x, src_y) {
                    resized.set_pixel(x, y, pixel);
                }
            }
        }
        
        resized
    }

    pub fn crop(&self, rect: &Rectangle) -> Image {
        let x = rect.x as usize;
        let y = rect.y as usize;
        let width = rect.width as usize;
        let height = rect.height as usize;
        
        let mut cropped = Image::new(width, height, self.channels);
        
        for cy in 0..height {
            for cx in 0..width {
                let src_x = x + cx;
                let src_y = y + cy;
                
                if let Some(pixel) = self.get_pixel(src_x, src_y) {
                    cropped.set_pixel(cx, cy, pixel);
                }
            }
        }
        
        cropped
    }
}

fn rgb_to_gray(r: u8, g: u8, b: u8) -> u8 {
    // Standard luminance formula
    (0.299 * r as f64 + 0.587 * g as f64 + 0.114 * b as f64) as u8
}

// Edge detection using Sobel operator
pub fn sobel_edge_detection(image: &Image) -> Image {
    let gray = if image.channels == 1 {
        image.clone()
    } else {
        image.to_grayscale()
    };

    let mut edges = Image::new(image.width, image.height, 1);

    // Sobel kernels
    let sobel_x = [
        [-1, 0, 1],
        [-2, 0, 2],
        [-1, 0, 1],
    ];

    let sobel_y = [
        [-1, -2, -1],
        [0, 0, 0],
        [1, 2, 1],
    ];

    for y in 1..image.height - 1 {
        for x in 1..image.width - 1 {
            let mut gx = 0i32;
            let mut gy = 0i32;

            for ky in 0..3 {
                for kx in 0..3 {
                    let pixel_x = x + kx - 1;
                    let pixel_y = y + ky - 1;
                    
                    if let Some(pixel) = gray.get_pixel(pixel_x, pixel_y) {
                        let value = pixel[0] as i32;
                        gx += value * sobel_x[ky][kx];
                        gy += value * sobel_y[ky][kx];
                    }
                }
            }

            let magnitude = ((gx * gx + gy * gy) as f64).sqrt() as u8;
            edges.set_pixel(x, y, &[magnitude]);
        }
    }

    edges
}

// Gaussian blur for noise reduction
pub fn gaussian_blur(image: &Image, radius: usize) -> Image {
    let kernel = generate_gaussian_kernel(radius);
    apply_convolution(image, &kernel)
}

fn generate_gaussian_kernel(radius: usize) -> Vec<Vec<f64>> {
    let size = radius * 2 + 1;
    let mut kernel = vec![vec![0.0; size]; size];
    let sigma = radius as f64 / 3.0;
    let two_sigma_sq = 2.0 * sigma * sigma;
    let mut sum = 0.0;

    for y in 0..size {
        for x in 0..size {
            let dx = x as f64 - radius as f64;
            let dy = y as f64 - radius as f64;
            let value = (-((dx * dx + dy * dy) / two_sigma_sq)).exp();
            kernel[y][x] = value;
            sum += value;
        }
    }

    // Normalize kernel
    for y in 0..size {
        for x in 0..size {
            kernel[y][x] /= sum;
        }
    }

    kernel
}

fn apply_convolution(image: &Image, kernel: &[Vec<f64>]) -> Image {
    let mut result = image.clone();
    let kernel_size = kernel.len();
    let kernel_radius = kernel_size / 2;

    for y in kernel_radius..image.height - kernel_radius {
        for x in kernel_radius..image.width - kernel_radius {
            let mut new_pixel = vec![0.0; image.channels];

            for ky in 0..kernel_size {
                for kx in 0..kernel_size {
                    let pixel_x = x + kx - kernel_radius;
                    let pixel_y = y + ky - kernel_radius;
                    
                    if let Some(pixel) = image.get_pixel(pixel_x, pixel_y) {
                        let weight = kernel[ky][kx];
                        for c in 0..image.channels {
                            new_pixel[c] += pixel[c] as f64 * weight;
                        }
                    }
                }
            }

            let final_pixel: Vec<u8> = new_pixel.into_iter()
                .map(|v| v.clamp(0.0, 255.0) as u8)
                .collect();
            result.set_pixel(x, y, &final_pixel);
        }
    }

    result
}

// Thresholding for binary images
pub fn threshold(image: &Image, threshold: u8) -> Image {
    let gray = if image.channels == 1 {
        image.clone()
    } else {
        image.to_grayscale()
    };

    let mut binary = Image::new(image.width, image.height, 1);

    for y in 0..image.height {
        for x in 0..image.width {
            if let Some(pixel) = gray.get_pixel(x, y) {
                let value = if pixel[0] > threshold { 255 } else { 0 };
                binary.set_pixel(x, y, &[value]);
            }
        }
    }

    binary
}

// Find connected components for object detection
pub fn find_connected_components(binary_image: &Image) -> Vec<Vec<Point>> {
    let mut visited = vec![vec![false; binary_image.width]; binary_image.height];
    let mut components = Vec::new();

    for y in 0..binary_image.height {
        for x in 0..binary_image.width {
            if !visited[y][x] {
                if let Some(pixel) = binary_image.get_pixel(x, y) {
                    if pixel[0] > 0 {
                        let component = flood_fill(binary_image, &mut visited, x, y);
                        if !component.is_empty() {
                            components.push(component);
                        }
                    }
                }
            }
        }
    }

    components
}

fn flood_fill(image: &Image, visited: &mut [Vec<bool>], start_x: usize, start_y: usize) -> Vec<Point> {
    let mut component = Vec::new();
    let mut stack = vec![(start_x, start_y)];

    while let Some((x, y)) = stack.pop() {
        if x >= image.width || y >= image.height || visited[y][x] {
            continue;
        }

        if let Some(pixel) = image.get_pixel(x, y) {
            if pixel[0] == 0 {
                continue;
            }
        }

        visited[y][x] = true;
        component.push(Point::new(x as f64, y as f64));

        // Add neighbors
        if x > 0 { stack.push((x - 1, y)); }
        if x < image.width - 1 { stack.push((x + 1, y)); }
        if y > 0 { stack.push((x, y - 1)); }
        if y < image.height - 1 { stack.push((x, y + 1)); }
    }

    component
}

// Simple template matching
pub fn template_match(image: &Image, template: &Image) -> Vec<(Point, f64)> {
    let mut matches = Vec::new();
    
    if template.width > image.width || template.height > image.height {
        return matches;
    }

    for y in 0..=image.height - template.height {
        for x in 0..=image.width - template.width {
            let similarity = calculate_normalized_cross_correlation(image, template, x, y);
            if similarity > 0.8 { // Threshold for match
                matches.push((Point::new(x as f64, y as f64), similarity));
            }
        }
    }

    matches
}

fn calculate_normalized_cross_correlation(image: &Image, template: &Image, offset_x: usize, offset_y: usize) -> f64 {
    let mut sum_image = 0.0;
    let mut sum_template = 0.0;
    let mut sum_product = 0.0;
    let mut sum_image_sq = 0.0;
    let mut sum_template_sq = 0.0;
    let mut count = 0;

    for ty in 0..template.height {
        for tx in 0..template.width {
            let ix = offset_x + tx;
            let iy = offset_y + ty;

            if let (Some(img_pixel), Some(temp_pixel)) = (image.get_pixel(ix, iy), template.get_pixel(tx, ty)) {
                let img_val = img_pixel[0] as f64;
                let temp_val = temp_pixel[0] as f64;

                sum_image += img_val;
                sum_template += temp_val;
                sum_product += img_val * temp_val;
                sum_image_sq += img_val * img_val;
                sum_template_sq += temp_val * temp_val;
                count += 1;
            }
        }
    }

    if count == 0 {
        return 0.0;
    }

    let count_f = count as f64;
    let numerator = sum_product - (sum_image * sum_template) / count_f;
    let denominator = ((sum_image_sq - sum_image * sum_image / count_f) * 
                      (sum_template_sq - sum_template * sum_template / count_f)).sqrt();

    if denominator == 0.0 {
        0.0
    } else {
        numerator / denominator
    }
}

// Brightness and contrast adjustment
pub fn adjust_brightness_contrast(image: &Image, brightness: i32, contrast: f64) -> Image {
    let mut adjusted = image.clone();
    
    for y in 0..image.height {
        for x in 0..image.width {
            if let Some(pixel) = image.get_pixel(x, y) {
                let mut new_pixel = Vec::new();
                
                for &channel in pixel {
                    let mut value = channel as f64;
                    value = value * contrast + brightness as f64;
                    value = value.clamp(0.0, 255.0);
                    new_pixel.push(value as u8);
                }
                
                adjusted.set_pixel(x, y, &new_pixel);
            }
        }
    }
    
    adjusted
}

// Histogram calculation
pub fn calculate_histogram(image: &Image, channel: usize) -> Vec<u32> {
    let mut histogram = vec![0u32; 256];
    
    if channel >= image.channels {
        return histogram;
    }
    
    for y in 0..image.height {
        for x in 0..image.width {
            if let Some(pixel) = image.get_pixel(x, y) {
                histogram[pixel[channel] as usize] += 1;
            }
        }
    }
    
    histogram
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_creation() {
        let image = Image::new(100, 100, 3);
        assert_eq!(image.width, 100);
        assert_eq!(image.height, 100);
        assert_eq!(image.channels, 3);
        assert_eq!(image.data.len(), 100 * 100 * 3);
    }

    #[test]
    fn test_pixel_operations() {
        let mut image = Image::new(10, 10, 3);
        let pixel = [255, 128, 0];
        
        image.set_pixel(5, 5, &pixel);
        assert_eq!(image.get_pixel(5, 5), Some(&pixel[..]));
    }

    #[test]
    fn test_grayscale_conversion() {
        let mut image = Image::new(2, 2, 3);
        image.set_pixel(0, 0, &[255, 0, 0]); // Red
        image.set_pixel(1, 1, &[0, 255, 0]); // Green
        
        let gray = image.to_grayscale();
        assert_eq!(gray.channels, 1);
        
        // Red should be darker than green in grayscale
        let red_gray = gray.get_pixel(0, 0).unwrap()[0];
        let green_gray = gray.get_pixel(1, 1).unwrap()[0];
        assert!(red_gray < green_gray);
    }

    #[test]
    fn test_image_resize() {
        let image = Image::new(100, 100, 3);
        let resized = image.resize(50, 50);
        
        assert_eq!(resized.width, 50);
        assert_eq!(resized.height, 50);
        assert_eq!(resized.channels, 3);
    }

    #[test]
    fn test_threshold() {
        let mut image = Image::new(3, 3, 1);
        image.set_pixel(0, 0, &[100]);
        image.set_pixel(1, 1, &[200]);
        image.set_pixel(2, 2, &[50]);
        
        let binary = threshold(&image, 128);
        
        assert_eq!(binary.get_pixel(0, 0).unwrap()[0], 0);   // 100 < 128
        assert_eq!(binary.get_pixel(1, 1).unwrap()[0], 255); // 200 > 128
        assert_eq!(binary.get_pixel(2, 2).unwrap()[0], 0);   // 50 < 128
    }

    #[test]
    fn test_histogram() {
        let mut image = Image::new(2, 2, 1);
        image.set_pixel(0, 0, &[100]);
        image.set_pixel(0, 1, &[100]);
        image.set_pixel(1, 0, &[200]);
        image.set_pixel(1, 1, &[200]);
        
        let histogram = calculate_histogram(&image, 0);
        
        assert_eq!(histogram[100], 2);
        assert_eq!(histogram[200], 2);
        assert_eq!(histogram[150], 0);
    }
}