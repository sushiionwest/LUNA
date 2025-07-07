/*!
 * Luna Visual AI Windows API Utilities
 * 
 * Windows-specific system integration and API wrappers:
 * - Safe Win32 API wrappers with error handling
 * - Window management and enumeration
 * - Input simulation (mouse and keyboard)
 * - Process and application management
 * - System information and monitoring
 * - Registry operations (safe and validated)
 * - Performance counter access
 * - Hardware information gathering
 */

use crate::core::error::{LunaError, Result};
use std::{
    ffi::{c_void, OsStr},
    mem,
    os::windows::ffi::OsStrExt,
    ptr,
};
use tracing::{debug, error, warn};
use windows::{
    core::{PCWSTR, PWSTR},
    Win32::{
        Foundation::{BOOL, HWND, LPARAM, POINT, RECT, TRUE},
        Graphics::Gdi::{
            GetDC, GetDeviceCaps, ReleaseDC, HORZRES, VERTRES,
        },
        System::{
            Diagnostics::Debug::GetLastError,
            Memory::{
                GlobalMemoryStatusEx, MEMORYSTATUSEX,
            },
            Performance::{
                QueryPerformanceCounter, QueryPerformanceFrequency, LARGE_INTEGER,
            },
            ProcessStatus::{
                GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS,
            },
            Threading::{
                GetCurrentProcess, OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ,
            },
        },
        UI::{
            Input::KeyboardAndMouse::{
                GetCursorPos, SetCursorPos, SendInput, INPUT, INPUT_KEYBOARD, INPUT_MOUSE,
                KEYBDINPUT, KEYEVENTF_KEYUP, MOUSEINPUT, WHEEL_DELTA,
            },
            WindowsAndMessaging::{
                EnumWindows, FindWindowW, GetForegroundWindow, GetWindow, GetWindowRect,
                GetWindowTextW, GetWindowThreadProcessId, IsWindowVisible, SetForegroundWindow,
                ShowWindow, GW_CHILD, GW_HWNDNEXT, SW_HIDE, SW_RESTORE, SW_SHOW,
            },
        },
    },
};

/// Window information structure
#[derive(Debug, Clone)]
pub struct WindowInfo {
    pub hwnd: isize,
    pub title: String,
    pub class_name: String,
    pub process_id: u32,
    pub thread_id: u32,
    pub rect: WindowRect,
    pub is_visible: bool,
    pub is_foreground: bool,
}

/// Window rectangle
#[derive(Debug, Clone, Copy)]
pub struct WindowRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
    pub width: i32,
    pub height: i32,
}

/// System memory information
#[derive(Debug, Clone)]
pub struct MemoryInfo {
    pub total_physical: u64,
    pub available_physical: u64,
    pub total_virtual: u64,
    pub available_virtual: u64,
    pub memory_load: u32,
}

/// Process memory information
#[derive(Debug, Clone)]
pub struct ProcessMemoryInfo {
    pub working_set_size: u64,
    pub peak_working_set_size: u64,
    pub page_file_usage: u64,
    pub peak_page_file_usage: u64,
}

/// Display information
#[derive(Debug, Clone)]
pub struct DisplayInfo {
    pub width: i32,
    pub height: i32,
    pub dpi_x: i32,
    pub dpi_y: i32,
    pub color_depth: i32,
}

/// Mouse button types
#[derive(Debug, Clone, Copy)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// Key state for input simulation
#[derive(Debug, Clone, Copy)]
pub enum KeyState {
    Down,
    Up,
}

/// Get system memory information
pub fn get_memory_info() -> Result<MemoryInfo> {
    unsafe {
        let mut mem_status = MEMORYSTATUSEX {
            dwLength: mem::size_of::<MEMORYSTATUSEX>() as u32,
            ..Default::default()
        };

        match GlobalMemoryStatusEx(&mut mem_status) {
            Ok(_) => Ok(MemoryInfo {
                total_physical: mem_status.ullTotalPhys,
                available_physical: mem_status.ullAvailPhys,
                total_virtual: mem_status.ullTotalVirtual,
                available_virtual: mem_status.ullAvailVirtual,
                memory_load: mem_status.dwMemoryLoad,
            }),
            Err(e) => Err(LunaError::windows_api(
                format!("Failed to get memory info: {}", e),
                GetLastError().0,
                "GlobalMemoryStatusEx",
            )),
        }
    }
}

/// Get process memory information
pub fn get_process_memory_info(process_id: u32) -> Result<ProcessMemoryInfo> {
    unsafe {
        let process = OpenProcess(
            PROCESS_QUERY_INFORMATION | PROCESS_VM_READ,
            BOOL(0),
            process_id,
        )
        .map_err(|e| LunaError::windows_api(
            format!("Failed to open process {}: {}", process_id, e),
            GetLastError().0,
            "OpenProcess",
        ))?;

        let mut mem_counters = PROCESS_MEMORY_COUNTERS::default();
        mem_counters.cb = mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;

        match GetProcessMemoryInfo(process, &mut mem_counters, mem_counters.cb) {
            Ok(_) => Ok(ProcessMemoryInfo {
                working_set_size: mem_counters.WorkingSetSize,
                peak_working_set_size: mem_counters.PeakWorkingSetSize,
                page_file_usage: mem_counters.PagefileUsage,
                peak_page_file_usage: mem_counters.PeakPagefileUsage,
            }),
            Err(e) => Err(LunaError::windows_api(
                format!("Failed to get process memory info: {}", e),
                GetLastError().0,
                "GetProcessMemoryInfo",
            )),
        }
    }
}

/// Get display information
pub fn get_display_info() -> Result<DisplayInfo> {
    unsafe {
        let dc = GetDC(HWND(0));
        if dc.is_invalid() {
            return Err(LunaError::windows_api(
                "Failed to get device context".to_string(),
                GetLastError().0,
                "GetDC",
            ));
        }

        let width = GetDeviceCaps(dc, HORZRES);
        let height = GetDeviceCaps(dc, VERTRES);
        let dpi_x = GetDeviceCaps(dc, 88); // LOGPIXELSX
        let dpi_y = GetDeviceCaps(dc, 90); // LOGPIXELSY
        let color_depth = GetDeviceCaps(dc, 12); // BITSPIXEL

        ReleaseDC(HWND(0), dc);

        Ok(DisplayInfo {
            width,
            height,
            dpi_x,
            dpi_y,
            color_depth,
        })
    }
}

/// Get current cursor position
pub fn get_cursor_position() -> Result<(i32, i32)> {
    unsafe {
        let mut point = POINT::default();
        match GetCursorPos(&mut point) {
            Ok(_) => Ok((point.x, point.y)),
            Err(e) => Err(LunaError::windows_api(
                format!("Failed to get cursor position: {}", e),
                GetLastError().0,
                "GetCursorPos",
            )),
        }
    }
}

/// Set cursor position
pub fn set_cursor_position(x: i32, y: i32) -> Result<()> {
    unsafe {
        match SetCursorPos(x, y) {
            Ok(_) => {
                debug!("Cursor position set to ({}, {})", x, y);
                Ok(())
            }
            Err(e) => Err(LunaError::windows_api(
                format!("Failed to set cursor position: {}", e),
                GetLastError().0,
                "SetCursorPos",
            )),
        }
    }
}

/// Simulate mouse click
pub fn simulate_mouse_click(x: i32, y: i32, button: MouseButton) -> Result<()> {
    // Move cursor to position first
    set_cursor_position(x, y)?;

    // Wait a moment for cursor to settle
    std::thread::sleep(std::time::Duration::from_millis(10));

    unsafe {
        let (down_flag, up_flag) = match button {
            MouseButton::Left => (0x0002, 0x0004), // MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP
            MouseButton::Right => (0x0008, 0x0010), // MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP
            MouseButton::Middle => (0x0020, 0x0040), // MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP
        };

        // Mouse down
        let input_down = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: windows::Win32::UI::Input::KeyboardAndMouse::INPUT_0 {
                mi: MOUSEINPUT {
                    dx: 0,
                    dy: 0,
                    mouseData: 0,
                    dwFlags: down_flag,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };

        // Mouse up
        let input_up = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: windows::Win32::UI::Input::KeyboardAndMouse::INPUT_0 {
                mi: MOUSEINPUT {
                    dx: 0,
                    dy: 0,
                    mouseData: 0,
                    dwFlags: up_flag,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };

        let inputs = [input_down, input_up];
        let sent = SendInput(&inputs, mem::size_of::<INPUT>() as i32);

        if sent != 2 {
            Err(LunaError::windows_api(
                format!("Failed to send mouse input, sent {} of 2 events", sent),
                GetLastError().0,
                "SendInput",
            ))
        } else {
            debug!("Mouse {:?} click simulated at ({}, {})", button, x, y);
            Ok(())
        }
    }
}

/// Simulate key press
pub fn simulate_key_press(virtual_key: u16, state: KeyState) -> Result<()> {
    unsafe {
        let flags = match state {
            KeyState::Down => 0,
            KeyState::Up => KEYEVENTF_KEYUP.0,
        };

        let input = INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: windows::Win32::UI::Input::KeyboardAndMouse::INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: virtual_key,
                    wScan: 0,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };

        let sent = SendInput(&[input], mem::size_of::<INPUT>() as i32);

        if sent != 1 {
            Err(LunaError::windows_api(
                format!("Failed to send keyboard input"),
                GetLastError().0,
                "SendInput",
            ))
        } else {
            debug!("Key {} {:?} simulated", virtual_key, state);
            Ok(())
        }
    }
}

/// Type text string
pub fn type_text(text: &str) -> Result<()> {
    for ch in text.chars() {
        // Convert character to virtual key code (simplified)
        let vk = char_to_virtual_key(ch);
        
        if let Some(key_code) = vk {
            simulate_key_press(key_code, KeyState::Down)?;
            std::thread::sleep(std::time::Duration::from_millis(10));
            simulate_key_press(key_code, KeyState::Up)?;
            std::thread::sleep(std::time::Duration::from_millis(10));
        } else {
            warn!("Cannot convert character '{}' to virtual key", ch);
        }
    }
    
    debug!("Text typed: '{}'", text);
    Ok(())
}

/// Convert character to virtual key code (simplified mapping)
fn char_to_virtual_key(ch: char) -> Option<u16> {
    match ch {
        'a'..='z' => Some((ch as u8 - b'a' + 0x41) as u16), // A-Z keys
        'A'..='Z' => Some((ch as u8 - b'A' + 0x41) as u16), // A-Z keys
        '0'..='9' => Some((ch as u8 - b'0' + 0x30) as u16), // 0-9 keys
        ' ' => Some(0x20), // Space
        '\n' => Some(0x0D), // Enter
        '\t' => Some(0x09), // Tab
        _ => None, // Unsupported character
    }
}

/// Get foreground window
pub fn get_foreground_window() -> Result<WindowInfo> {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0 == 0 {
            return Err(LunaError::windows_api(
                "No foreground window".to_string(),
                0,
                "GetForegroundWindow",
            ));
        }

        get_window_info(hwnd)
    }
}

/// Get window information
pub fn get_window_info(hwnd: HWND) -> Result<WindowInfo> {
    unsafe {
        // Get window title
        let mut title_buffer = [0u16; 256];
        let title_len = GetWindowTextW(hwnd, &mut title_buffer);
        let title = String::from_utf16_lossy(&title_buffer[..title_len as usize]);

        // Get process and thread IDs
        let mut process_id = 0u32;
        let thread_id = GetWindowThreadProcessId(hwnd, Some(&mut process_id));

        // Get window rectangle
        let mut rect = RECT::default();
        GetWindowRect(hwnd, &mut rect)
            .map_err(|e| LunaError::windows_api(
                format!("Failed to get window rect: {}", e),
                GetLastError().0,
                "GetWindowRect",
            ))?;

        let window_rect = WindowRect {
            left: rect.left,
            top: rect.top,
            right: rect.right,
            bottom: rect.bottom,
            width: rect.right - rect.left,
            height: rect.bottom - rect.top,
        };

        // Check if visible and foreground
        let is_visible = IsWindowVisible(hwnd).as_bool();
        let is_foreground = GetForegroundWindow() == hwnd;

        Ok(WindowInfo {
            hwnd: hwnd.0,
            title,
            class_name: String::new(), // Would need GetClassName for this
            process_id,
            thread_id,
            rect: window_rect,
            is_visible,
            is_foreground,
        })
    }
}

/// Enumerate all windows
pub fn enumerate_windows() -> Result<Vec<WindowInfo>> {
    unsafe {
        let mut windows = Vec::new();
        let windows_ptr = &mut windows as *mut Vec<WindowInfo>;

        let enum_proc = Some(enum_windows_proc);
        
        match EnumWindows(enum_proc, LPARAM(windows_ptr as isize)) {
            Ok(_) => Ok(windows),
            Err(e) => Err(LunaError::windows_api(
                format!("Failed to enumerate windows: {}", e),
                GetLastError().0,
                "EnumWindows",
            )),
        }
    }
}

/// Window enumeration callback
unsafe extern "system" fn enum_windows_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let windows_ptr = lparam.0 as *mut Vec<WindowInfo>;
    let windows = &mut *windows_ptr;

    // Skip invisible windows
    if !IsWindowVisible(hwnd).as_bool() {
        return TRUE;
    }

    // Get window info
    if let Ok(window_info) = get_window_info(hwnd) {
        // Skip windows with empty titles
        if !window_info.title.is_empty() {
            windows.push(window_info);
        }
    }

    TRUE
}

/// Find window by title
pub fn find_window_by_title(title: &str) -> Result<Option<WindowInfo>> {
    let windows = enumerate_windows()?;
    
    for window in windows {
        if window.title.contains(title) {
            return Ok(Some(window));
        }
    }
    
    Ok(None)
}

/// Bring window to foreground
pub fn bring_window_to_foreground(hwnd: isize) -> Result<()> {
    unsafe {
        let hwnd = HWND(hwnd);
        
        match SetForegroundWindow(hwnd) {
            Ok(_) => {
                debug!("Window brought to foreground: {}", hwnd.0);
                Ok(())
            }
            Err(e) => Err(LunaError::windows_api(
                format!("Failed to bring window to foreground: {}", e),
                GetLastError().0,
                "SetForegroundWindow",
            )),
        }
    }
}

/// Show or hide window
pub fn show_window(hwnd: isize, show: bool) -> Result<()> {
    unsafe {
        let hwnd = HWND(hwnd);
        let cmd = if show { SW_SHOW } else { SW_HIDE };
        
        ShowWindow(hwnd, cmd);
        debug!("Window {} {}", hwnd.0, if show { "shown" } else { "hidden" });
        Ok(())
    }
}

/// Get high-resolution timestamp
pub fn get_high_res_timestamp() -> Result<u64> {
    unsafe {
        let mut counter = LARGE_INTEGER::default();
        let mut frequency = LARGE_INTEGER::default();

        QueryPerformanceFrequency(&mut frequency)
            .map_err(|e| LunaError::windows_api(
                format!("Failed to get performance frequency: {}", e),
                GetLastError().0,
                "QueryPerformanceFrequency",
            ))?;

        QueryPerformanceCounter(&mut counter)
            .map_err(|e| LunaError::windows_api(
                format!("Failed to get performance counter: {}", e),
                GetLastError().0,
                "QueryPerformanceCounter",
            ))?;

        // Convert to microseconds
        let timestamp = (counter.QuadPart() * 1_000_000) / frequency.QuadPart();
        Ok(timestamp as u64)
    }
}

/// Convert string to wide string for Windows APIs
pub fn to_wide_string(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
}

/// Convert wide string to regular string
pub fn from_wide_string(wide: &[u16]) -> String {
    // Find null terminator
    let len = wide.iter().position(|&c| c == 0).unwrap_or(wide.len());
    String::from_utf16_lossy(&wide[..len])
}

/// Check if current user has administrator privileges
pub fn is_administrator() -> bool {
    // Simplified check - in a real implementation you'd use proper privilege checks
    // This is a placeholder that assumes non-admin for safety
    false
}

/// Validate Windows API accessibility
pub fn validate_apis() -> Result<()> {
    // Test basic API calls to ensure they work
    let _ = get_display_info()?;
    let _ = get_cursor_position()?;
    let _ = get_memory_info()?;
    
    debug!("Windows APIs validated successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_info() {
        let memory_info = get_memory_info().unwrap();
        assert!(memory_info.total_physical > 0);
        assert!(memory_info.available_physical > 0);
    }

    #[test]
    fn test_display_info() {
        let display_info = get_display_info().unwrap();
        assert!(display_info.width > 0);
        assert!(display_info.height > 0);
    }

    #[test]
    fn test_cursor_position() {
        let pos = get_cursor_position().unwrap();
        // Position should be valid screen coordinates
        assert!(pos.0 >= 0);
        assert!(pos.1 >= 0);
    }

    #[test]
    fn test_string_conversion() {
        let test_str = "Hello, World!";
        let wide = to_wide_string(test_str);
        let converted = from_wide_string(&wide);
        assert_eq!(test_str, converted);
    }

    #[test]
    fn test_char_to_virtual_key() {
        assert_eq!(char_to_virtual_key('A'), Some(0x41));
        assert_eq!(char_to_virtual_key('a'), Some(0x41));
        assert_eq!(char_to_virtual_key('0'), Some(0x30));
        assert_eq!(char_to_virtual_key(' '), Some(0x20));
    }

    #[test]
    fn test_api_validation() {
        let result = validate_apis();
        assert!(result.is_ok());
    }
}