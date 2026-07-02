// Screen capture functionality with minimal dependencies
// Cross-platform screen capture implementation

use crate::utils::image_processing::Image;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct CaptureConfig {
    pub target_fps: u32,
    pub compression_quality: u8,
    pub capture_cursor: bool,
    pub capture_region: Option<CaptureRegion>,
}

#[derive(Debug, Clone)]
pub struct CaptureRegion {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            target_fps: 30,
            compression_quality: 85,
            capture_cursor: false,
            capture_region: None,
        }
    }
}

pub struct ScreenCapture {
    config: CaptureConfig,
    last_capture_time: Option<Instant>,
    frame_interval: Duration,
}

impl ScreenCapture {
    pub fn new(config: CaptureConfig) -> Self {
        let frame_interval = Duration::from_millis(1000 / config.target_fps as u64);
        
        Self {
            config,
            last_capture_time: None,
            frame_interval,
        }
    }

    pub fn capture_screen(&mut self) -> Result<Image, CaptureError> {
        // Rate limiting
        if let Some(last_time) = self.last_capture_time {
            let elapsed = last_time.elapsed();
            if elapsed < self.frame_interval {
                std::thread::sleep(self.frame_interval - elapsed);
            }
        }

        let image = match self.config.capture_region {
            Some(ref region) => self.capture_region(region)?,
            None => self.capture_full_screen()?,
        };

        self.last_capture_time = Some(Instant::now());
        Ok(image)
    }

    #[cfg(target_os = "windows")]
    fn capture_full_screen(&self) -> Result<Image, CaptureError> {
        // Simplified Windows implementation
        // In a real implementation, would use Windows GDI or DXGI
        self.windows_capture_screen()
    }

    #[cfg(target_os = "linux")]
    fn capture_full_screen(&self) -> Result<Image, CaptureError> {
        // Simplified Linux implementation
        // In a real implementation, would use X11 or Wayland
        self.linux_capture_screen()
    }

    #[cfg(target_os = "macos")]
    fn capture_full_screen(&self) -> Result<Image, CaptureError> {
        // Simplified macOS implementation
        // In a real implementation, would use Core Graphics
        self.macos_capture_screen()
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    fn capture_full_screen(&self) -> Result<Image, CaptureError> {
        // Fallback for unsupported platforms
        self.create_dummy_screen()
    }

    fn capture_region(&self, region: &CaptureRegion) -> Result<Image, CaptureError> {
        let full_screen = self.capture_full_screen()?;
        
        // Crop to the specified region
        let crop_rect = crate::utils::geometry::Rectangle::new(
            region.x as f64,
            region.y as f64,
            region.width as f64,
            region.height as f64,
        );
        
        Ok(full_screen.crop(&crop_rect))
    }

    #[cfg(target_os = "windows")]
    fn windows_capture_screen(&self) -> Result<Image, CaptureError> {
        // Placeholder implementation
        // Real implementation would use:
        // - GetDC(NULL) to get screen DC
        // - CreateCompatibleDC and CreateCompatibleBitmap
        // - BitBlt to copy screen content
        // - GetDIBits to get raw pixel data
        
        println!("Windows screen capture - would use GDI/DXGI");
        self.create_test_pattern(1920, 1080)
    }

    #[cfg(target_os = "linux")]
    fn linux_capture_screen(&self) -> Result<Image, CaptureError> {
        // Placeholder implementation
        // Real implementation would use:
        // - X11: XGetImage with root window
        // - Wayland: wlr-screencopy or similar protocol
        
        println!("Linux screen capture - would use X11/Wayland");
        self.create_test_pattern(1920, 1080)
    }

    #[cfg(target_os = "macos")]
    fn macos_capture_screen(&self) -> Result<Image, CaptureError> {
        // Placeholder implementation
        // Real implementation would use:
        // - CGDisplayCreateImage
        // - CGImageGetDataProvider and CGDataProviderCopyData
        
        println!("macOS screen capture - would use Core Graphics");
        self.create_test_pattern(1920, 1080)
    }

    fn create_dummy_screen(&self) -> Result<Image, CaptureError> {
        println!("Unsupported platform - creating dummy screen");
        self.create_test_pattern(1920, 1080)
    }

    fn create_test_pattern(&self, width: usize, height: usize) -> Result<Image, CaptureError> {
        let mut image = Image::new(width, height, 3);
        
        // Create a test pattern with gradients and some UI-like elements
        for y in 0..height {
            for x in 0..width {
                let r = ((x as f64 / width as f64) * 255.0) as u8;
                let g = ((y as f64 / height as f64) * 255.0) as u8;
                let b = 128;
                
                // Add some "button-like" rectangles
                let in_button1 = x > 100 && x < 300 && y > 100 && y < 150;
                let in_button2 = x > 400 && x < 600 && y > 200 && y < 250;
                let in_textbox = x > 100 && x < 500 && y > 300 && y < 330;
                
                let pixel = if in_button1 || in_button2 {
                    [200, 200, 200] // Light gray for buttons
                } else if in_textbox {
                    [255, 255, 255] // White for text box
                } else {
                    [r, g, b] // Gradient background
                };
                
                image.set_pixel(x, y, &pixel);
            }
        }
        
        Ok(image)
    }

    pub fn get_screen_dimensions(&self) -> Result<(u32, u32), CaptureError> {
        #[cfg(target_os = "windows")]
        {
            // Would use GetSystemMetrics(SM_CXSCREEN) and GetSystemMetrics(SM_CYSCREEN)
            Ok((1920, 1080))
        }
        
        #[cfg(target_os = "linux")]
        {
            // Would use X11 DisplayWidth/DisplayHeight or Wayland output info
            Ok((1920, 1080))
        }
        
        #[cfg(target_os = "macos")]
        {
            // Would use CGDisplayPixelsWide/CGDisplayPixelsHigh
            Ok((1920, 1080))
        }
        
        #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
        {
            Ok((1920, 1080))
        }
    }

    pub fn list_displays(&self) -> Result<Vec<DisplayInfo>, CaptureError> {
        // Placeholder - would enumerate actual displays on each platform
        Ok(vec![DisplayInfo {
            id: 0,
            name: "Primary Display".to_string(),
            width: 1920,
            height: 1080,
            x: 0,
            y: 0,
            is_primary: true,
        }])
    }

    pub fn capture_window(&self, window_id: u64) -> Result<Image, CaptureError> {
        // Placeholder for window-specific capture
        println!("Window capture for ID: {}", window_id);
        self.create_test_pattern(800, 600)
    }
}

#[derive(Debug, Clone)]
pub struct DisplayInfo {
    pub id: u32,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub x: i32,
    pub y: i32,
    pub is_primary: bool,
}

// Async screen capture for non-blocking operation
pub struct AsyncScreenCapture {
    capture: ScreenCapture,
    capture_thread: Option<std::thread::JoinHandle<()>>,
    frame_receiver: Option<std::sync::mpsc::Receiver<Result<Image, CaptureError>>>,
    should_stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl AsyncScreenCapture {
    pub fn new(config: CaptureConfig) -> Self {
        Self {
            capture: ScreenCapture::new(config),
            capture_thread: None,
            frame_receiver: None,
            should_stop: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    pub fn start_capture(&mut self) -> Result<(), CaptureError> {
        if self.capture_thread.is_some() {
            return Err(CaptureError::AlreadyRunning);
        }

        let (tx, rx) = std::sync::mpsc::channel();
        let should_stop = self.should_stop.clone();
        let mut capture = ScreenCapture::new(self.capture.config.clone());

        let handle = std::thread::spawn(move || {
            while !should_stop.load(std::sync::atomic::Ordering::Relaxed) {
                match capture.capture_screen() {
                    Ok(image) => {
                        if tx.send(Ok(image)).is_err() {
                            break; // Receiver disconnected
                        }
                    }
                    Err(e) => {
                        if tx.send(Err(e)).is_err() {
                            break; // Receiver disconnected
                        }
                    }
                }
            }
        });

        self.capture_thread = Some(handle);
        self.frame_receiver = Some(rx);
        Ok(())
    }

    pub fn stop_capture(&mut self) {
        self.should_stop.store(true, std::sync::atomic::Ordering::Relaxed);
        
        if let Some(handle) = self.capture_thread.take() {
            let _ = handle.join();
        }
        
        self.frame_receiver = None;
    }

    pub fn get_latest_frame(&self) -> Option<Result<Image, CaptureError>> {
        if let Some(ref receiver) = self.frame_receiver {
            receiver.try_recv().ok()
        } else {
            None
        }
    }
}

impl Drop for AsyncScreenCapture {
    fn drop(&mut self) {
        self.stop_capture();
    }
}

#[derive(Debug)]
pub enum CaptureError {
    PlatformError(String),
    InvalidRegion,
    AlreadyRunning,
    NotRunning,
    SystemError(String),
}

impl std::fmt::Display for CaptureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CaptureError::PlatformError(msg) => write!(f, "Platform error: {}", msg),
            CaptureError::InvalidRegion => write!(f, "Invalid capture region"),
            CaptureError::AlreadyRunning => write!(f, "Capture already running"),
            CaptureError::NotRunning => write!(f, "Capture not running"),
            CaptureError::SystemError(msg) => write!(f, "System error: {}", msg),
        }
    }
}

impl std::error::Error for CaptureError {}

// Utility functions
pub fn quick_screenshot() -> Result<Image, CaptureError> {
    let mut capture = ScreenCapture::new(CaptureConfig::default());
    capture.capture_screen()
}

pub fn screenshot_region(x: i32, y: i32, width: u32, height: u32) -> Result<Image, CaptureError> {
    let config = CaptureConfig {
        capture_region: Some(CaptureRegion { x, y, width, height }),
        ..Default::default()
    };
    let mut capture = ScreenCapture::new(config);
    capture.capture_screen()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capture_config() {
        let config = CaptureConfig::default();
        assert_eq!(config.target_fps, 30);
        assert_eq!(config.compression_quality, 85);
        assert!(!config.capture_cursor);
        assert!(config.capture_region.is_none());
    }

    #[test]
    fn test_screen_capture_creation() {
        let config = CaptureConfig::default();
        let capture = ScreenCapture::new(config);
        assert!(capture.last_capture_time.is_none());
    }

    #[test]
    fn test_quick_screenshot() {
        let result = quick_screenshot();
        assert!(result.is_ok());
        
        let image = result.unwrap();
        assert!(image.width > 0);
        assert!(image.height > 0);
        assert_eq!(image.channels, 3);
    }

    #[test]
    fn test_screenshot_region() {
        let result = screenshot_region(0, 0, 100, 100);
        assert!(result.is_ok());
        
        let image = result.unwrap();
        assert!(image.width <= 100);
        assert!(image.height <= 100);
    }

    #[test]
    fn test_async_capture_lifecycle() {
        let mut async_capture = AsyncScreenCapture::new(CaptureConfig::default());
        
        // Should start successfully
        assert!(async_capture.start_capture().is_ok());
        
        // Should not start again
        assert!(matches!(async_capture.start_capture(), Err(CaptureError::AlreadyRunning)));
        
        // Should stop successfully
        async_capture.stop_capture();
        
        // Should be able to start again after stopping
        assert!(async_capture.start_capture().is_ok());
    }
}