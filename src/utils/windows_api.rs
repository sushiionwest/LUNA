/*!
 * Windows API Helper - Utilities for Windows system integration
 */

use anyhow::Result;
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use tracing::{debug, warn, error};
use windows::Win32::{
    Foundation::{HWND, LPARAM, WPARAM, RECT, POINT, BOOL},
    System::{
        Diagnostics::Debug::GetLastError,
        ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS},
        Threading::{GetCurrentProcess, OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ},
    },
    UI::WindowsAndMessaging::{
        FindWindowW, GetWindowTextW, GetWindowRect, IsWindowVisible,
        EnumWindows, GetWindowThreadProcessId, ShowWindow, SW_MINIMIZE, SW_RESTORE,
        GetForegroundWindow, SetForegroundWindow, GetDesktopWindow,
    },
};

/// Windows API helper for system operations
pub struct WindowsApiHelper;

impl WindowsApiHelper {
    /// Get current memory usage in MB
    pub fn get_memory_usage() -> Result<u64> {
        unsafe {
            let process = GetCurrentProcess();
            let mut counters = PROCESS_MEMORY_COUNTERS::default();
            
            if GetProcessMemoryInfo(process, &mut counters, std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32).as_bool() {
                let working_set_mb = counters.WorkingSetSize / (1024 * 1024);
                Ok(working_set_mb as u64)
            } else {
                Err(anyhow::anyhow!("Failed to get memory usage"))
            }
        }
    }
    
    /// Get CPU usage percentage (simplified)
    pub fn get_cpu_usage() -> Result<f64> {
        // Simplified CPU usage - in real implementation would use performance counters
        // For now, return a mock value based on system load
        Ok(15.0) // Mock 15% CPU usage
    }
    
    /// Find window by title
    pub fn find_window_by_title(title: &str) -> Option<HWND> {
        unsafe {
            let wide_title: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
            let hwnd = FindWindowW(None, windows::core::PCWSTR(wide_title.as_ptr()));
            
            if hwnd.0 != 0 {
                Some(hwnd)
            } else {
                None
            }
        }
    }
    
    /// Get window title
    pub fn get_window_title(hwnd: HWND) -> Result<String> {
        unsafe {
            let mut buffer = [0u16; 256];
            let len = GetWindowTextW(hwnd, &mut buffer);
            
            if len > 0 {
                let title = OsString::from_wide(&buffer[..len as usize])
                    .to_string_lossy()
                    .to_string();
                Ok(title)
            } else {
                Err(anyhow::anyhow!("Failed to get window title"))
            }
        }
    }
    
    /// Get window bounds
    pub fn get_window_bounds(hwnd: HWND) -> Result<(i32, i32, i32, i32)> {
        unsafe {
            let mut rect = RECT::default();
            if GetWindowRect(hwnd, &mut rect).as_bool() {
                Ok((rect.left, rect.top, rect.right - rect.left, rect.bottom - rect.top))
            } else {
                Err(anyhow::anyhow!("Failed to get window bounds"))
            }
        }
    }
    
    /// Check if window is visible
    pub fn is_window_visible(hwnd: HWND) -> bool {
        unsafe { IsWindowVisible(hwnd).as_bool() }
    }
    
    /// Get foreground window
    pub fn get_foreground_window() -> HWND {
        unsafe { GetForegroundWindow() }
    }
    
    /// Set foreground window
    pub fn set_foreground_window(hwnd: HWND) -> Result<()> {
        unsafe {
            if SetForegroundWindow(hwnd).as_bool() {
                Ok(())
            } else {
                Err(anyhow::anyhow!("Failed to set foreground window"))
            }
        }
    }
    
    /// Minimize window
    pub fn minimize_window(hwnd: HWND) -> Result<()> {
        unsafe {
            if ShowWindow(hwnd, SW_MINIMIZE).as_bool() {
                Ok(())
            } else {
                Err(anyhow::anyhow!("Failed to minimize window"))
            }
        }
    }
    
    /// Restore window
    pub fn restore_window(hwnd: HWND) -> Result<()> {
        unsafe {
            if ShowWindow(hwnd, SW_RESTORE).as_bool() {
                Ok(())
            } else {
                Err(anyhow::anyhow!("Failed to restore window"))
            }
        }
    }
    
    /// Enumerate all visible windows
    pub fn enumerate_windows() -> Result<Vec<WindowInfo>> {
        let mut windows = Vec::new();
        
        unsafe {
            let mut windows_ptr = &mut windows as *mut Vec<WindowInfo>;
            
            EnumWindows(
                Some(enum_windows_proc),
                LPARAM(windows_ptr as isize),
            );
        }
        
        Ok(windows)
    }
    
    /// Get system information
    pub fn get_system_info() -> SystemInfo {
        SystemInfo {
            memory_usage_mb: Self::get_memory_usage().unwrap_or(0),
            cpu_usage_percent: Self::get_cpu_usage().unwrap_or(0.0),
            foreground_window: Self::get_foreground_window(),
            window_count: Self::enumerate_windows().map(|w| w.len()).unwrap_or(0),
        }
    }
    
    /// Check if Luna has required permissions
    pub fn check_permissions() -> PermissionStatus {
        let mut status = PermissionStatus {
            screen_capture: false,
            input_simulation: false,
            window_management: false,
            memory_access: false,
        };
        
        // Test screen capture
        status.screen_capture = Self::test_screen_capture();
        
        // Test input simulation (simplified check)
        status.input_simulation = true; // Assume we have input permissions
        
        // Test window management
        status.window_management = Self::get_foreground_window().0 != 0;
        
        // Test memory access
        status.memory_access = Self::get_memory_usage().is_ok();
        
        status
    }
    
    /// Test if screen capture is available
    fn test_screen_capture() -> bool {
        // Try to get desktop window
        unsafe {
            let desktop = GetDesktopWindow();
            desktop.0 != 0
        }
    }
    
    /// Get last Windows error
    pub fn get_last_error() -> u32 {
        unsafe { GetLastError().0 }
    }
    
    /// Convert Windows error to string
    pub fn error_to_string(error_code: u32) -> String {
        // Simplified error conversion
        match error_code {
            0 => "Success".to_string(),
            5 => "Access denied".to_string(),
            6 => "Invalid handle".to_string(),
            87 => "Invalid parameter".to_string(),
            _ => format!("Windows error code: {}", error_code),
        }
    }
}

/// Window information
#[derive(Debug, Clone)]
pub struct WindowInfo {
    pub hwnd: HWND,
    pub title: String,
    pub visible: bool,
    pub bounds: (i32, i32, i32, i32), // x, y, width, height
    pub process_id: u32,
}

/// System information
#[derive(Debug, Clone)]
pub struct SystemInfo {
    pub memory_usage_mb: u64,
    pub cpu_usage_percent: f64,
    pub foreground_window: HWND,
    pub window_count: usize,
}

/// Permission status
#[derive(Debug, Clone)]
pub struct PermissionStatus {
    pub screen_capture: bool,
    pub input_simulation: bool,
    pub window_management: bool,
    pub memory_access: bool,
}

impl PermissionStatus {
    pub fn all_granted(&self) -> bool {
        self.screen_capture && self.input_simulation && self.window_management && self.memory_access
    }
    
    pub fn get_missing_permissions(&self) -> Vec<String> {
        let mut missing = Vec::new();
        
        if !self.screen_capture {
            missing.push("Screen Capture".to_string());
        }
        if !self.input_simulation {
            missing.push("Input Simulation".to_string());
        }
        if !self.window_management {
            missing.push("Window Management".to_string());
        }
        if !self.memory_access {
            missing.push("Memory Access".to_string());
        }
        
        missing
    }
}

/// Callback for enumerating windows
unsafe extern "system" fn enum_windows_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let windows_ptr = lparam.0 as *mut Vec<WindowInfo>;
    let windows = &mut *windows_ptr;
    
    // Only include visible windows with titles
    if IsWindowVisible(hwnd).as_bool() {
        if let Ok(title) = WindowsApiHelper::get_window_title(hwnd) {
            if !title.is_empty() {
                let bounds = WindowsApiHelper::get_window_bounds(hwnd).unwrap_or((0, 0, 0, 0));
                
                let mut process_id = 0;
                GetWindowThreadProcessId(hwnd, Some(&mut process_id));
                
                windows.push(WindowInfo {
                    hwnd,
                    title,
                    visible: true,
                    bounds,
                    process_id,
                });
            }
        }
    }
    
    BOOL::from(true) // Continue enumeration
}

/// Windows API utilities for Luna
pub struct WinApiUtils;

impl WinApiUtils {
    /// Check if running as administrator
    pub fn is_admin() -> bool {
        // Simplified admin check - in real implementation would use tokens
        std::env::var("USERNAME").unwrap_or_default().to_lowercase().contains("admin")
    }
    
    /// Get Windows version
    pub fn get_windows_version() -> String {
        // Simplified version detection
        "Windows 10/11".to_string()
    }
    
    /// Get screen DPI scaling
    pub fn get_dpi_scaling() -> f32 {
        // Simplified DPI detection - would use GetDpiForMonitor in real implementation
        1.0 // Assume 100% scaling
    }
    
    /// Check if dark mode is enabled
    pub fn is_dark_mode() -> bool {
        // Simplified dark mode detection
        false
    }
    
    /// Get system uptime in seconds
    pub fn get_system_uptime() -> u64 {
        // Simplified uptime - would use GetTickCount64 in real implementation
        3600 // Mock 1 hour uptime
    }
}

/// Create a Windows API error from the last error
pub fn windows_error(operation: &str) -> anyhow::Error {
    let error_code = WindowsApiHelper::get_last_error();
    let error_message = WindowsApiHelper::error_to_string(error_code);
    anyhow::anyhow!("{} failed: {} ({})", operation, error_message, error_code)
}

/// Macro for Windows API error handling
#[macro_export]
macro_rules! win_try {
    ($expr:expr, $op:expr) => {
        if !$expr.as_bool() {
            return Err(crate::utils::windows_api::windows_error($op));
        }
    };
}

/// Safe wrapper for Windows string conversion
pub fn to_wide_string(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Safe wrapper for Windows string from wide
pub fn from_wide_string(wide: &[u16]) -> String {
    let end = wide.iter().position(|&c| c == 0).unwrap_or(wide.len());
    OsString::from_wide(&wide[..end]).to_string_lossy().to_string()
}