/*!
 * Luna Input System - Windows input automation
 * 
 * Handles mouse clicks, keyboard input, and system interactions
 */

use anyhow::Result;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};
use windows::Win32::{
    Foundation::{POINT, LPARAM, WPARAM},
    UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_MOUSE, INPUT_KEYBOARD, 
        MOUSEINPUT, KEYBDINPUT,
        MOUSE_EVENT_FLAGS, KEYBD_EVENT_FLAGS,
        MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP,
        MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP,
        MOUSEEVENTF_MOVE, MOUSEEVENTF_ABSOLUTE,
        KEYEVENTF_KEYUP, VK_CONTROL, VK_SHIFT, VK_MENU,
    },
    UI::WindowsAndMessaging::{
        SetCursorPos, GetCursorPos, GetSystemMetrics,
        SM_CXSCREEN, SM_CYSCREEN,
    },
};

use crate::core::{LunaAction, ScrollDirection};

/// Windows input system for automation
pub struct WindowsInputSystem {
    /// Queue of pending actions
    action_queue: Arc<Mutex<VecDeque<LunaAction>>>,
    /// Current cursor position
    cursor_position: Arc<Mutex<(i32, i32)>>,
    /// Whether input is currently active
    active: Arc<std::sync::atomic::AtomicBool>,
    /// Screen dimensions
    screen_width: i32,
    screen_height: i32,
}

impl WindowsInputSystem {
    /// Create new Windows input system
    pub fn new() -> Result<Self> {
        // Get screen dimensions
        let screen_width = unsafe { GetSystemMetrics(SM_CXSCREEN) };
        let screen_height = unsafe { GetSystemMetrics(SM_CYSCREEN) };
        
        debug!("Windows input system initialized for {}x{} screen", screen_width, screen_height);
        
        Ok(Self {
            action_queue: Arc::new(Mutex::new(VecDeque::new())),
            cursor_position: Arc::new(Mutex::new((0, 0))),
            active: Arc::new(std::sync::atomic::AtomicBool::new(true)),
            screen_width,
            screen_height,
        })
    }
    
    /// Execute a single action
    pub async fn execute_action(&self, action: &LunaAction) -> Result<()> {
        if !self.active.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(anyhow::anyhow!("Input system is not active"));
        }
        
        debug!("Executing action: {:?}", action);
        
        match action {
            LunaAction::Click { x, y } => {
                self.click(*x, *y).await?;
            }
            LunaAction::RightClick { x, y } => {
                self.right_click(*x, *y).await?;
            }
            LunaAction::DoubleClick { x, y } => {
                self.double_click(*x, *y).await?;
            }
            LunaAction::Type { text } => {
                self.type_text(text).await?;
            }
            LunaAction::KeyPress { key } => {
                self.press_key(key).await?;
            }
            LunaAction::KeyCombo { keys } => {
                self.press_key_combo(keys).await?;
            }
            LunaAction::Scroll { x, y, direction } => {
                self.scroll(*x, *y, direction).await?;
            }
            LunaAction::Drag { from_x, from_y, to_x, to_y } => {
                self.drag(*from_x, *from_y, *to_x, *to_y).await?;
            }
            LunaAction::Wait { milliseconds } => {
                self.wait(*milliseconds).await;
            }
        }
        
        debug!("Action executed successfully");
        Ok(())
    }
    
    /// Move cursor and click at position
    async fn click(&self, x: i32, y: i32) -> Result<()> {
        self.move_cursor(x, y).await?;
        self.mouse_click(true).await?;
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        self.mouse_click(false).await?;
        Ok(())
    }
    
    /// Right-click at position
    async fn right_click(&self, x: i32, y: i32) -> Result<()> {
        self.move_cursor(x, y).await?;
        self.mouse_right_click(true).await?;
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        self.mouse_right_click(false).await?;
        Ok(())
    }
    
    /// Double-click at position
    async fn double_click(&self, x: i32, y: i32) -> Result<()> {
        self.click(x, y).await?;
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        self.click(x, y).await?;
        Ok(())
    }
    
    /// Type text character by character
    async fn type_text(&self, text: &str) -> Result<()> {
        debug!("Typing text: '{}'", text);
        
        for ch in text.chars() {
            self.type_character(ch).await?;
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }
        
        Ok(())
    }
    
    /// Type a single character
    async fn type_character(&self, ch: char) -> Result<()> {
        // Convert character to virtual key code (simplified)
        let vk_code = match ch {
            'a'..='z' => (ch as u8 - b'a' + 0x41) as u16,
            'A'..='Z' => {
                // Shift + letter
                self.key_down(VK_SHIFT.0).await?;
                let code = (ch as u8 - b'A' + 0x41) as u16;
                self.key_down(code).await?;
                self.key_up(code).await?;
                self.key_up(VK_SHIFT.0).await?;
                return Ok(());
            }
            '0'..='9' => (ch as u8 - b'0' + 0x30) as u16,
            ' ' => 0x20, // Space
            '\n' => 0x0D, // Enter
            '\t' => 0x09, // Tab
            _ => {
                warn!("Unsupported character for typing: '{}'", ch);
                return Ok(());
            }
        };
        
        self.key_down(vk_code).await?;
        self.key_up(vk_code).await?;
        
        Ok(())
    }
    
    /// Press a single key
    async fn press_key(&self, key: &str) -> Result<()> {
        let vk_code = self.get_virtual_key_code(key)?;
        self.key_down(vk_code).await?;
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        self.key_up(vk_code).await?;
        Ok(())
    }
    
    /// Press a key combination
    async fn press_key_combo(&self, keys: &[String]) -> Result<()> {
        debug!("Pressing key combo: {:?}", keys);
        
        // Press all keys down
        let mut vk_codes = Vec::new();
        for key in keys {
            let vk_code = self.get_virtual_key_code(key)?;
            self.key_down(vk_code).await?;
            vk_codes.push(vk_code);
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }
        
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        // Release all keys in reverse order
        for vk_code in vk_codes.iter().rev() {
            self.key_up(*vk_code).await?;
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }
        
        Ok(())
    }
    
    /// Scroll at position
    async fn scroll(&self, x: i32, y: i32, direction: &ScrollDirection) -> Result<()> {
        self.move_cursor(x, y).await?;
        
        // Simulate scroll wheel (simplified)
        let scroll_amount = match direction {
            ScrollDirection::Up => 3,
            ScrollDirection::Down => -3,
            ScrollDirection::Left => 0, // Horizontal scroll not implemented in this simplified version
            ScrollDirection::Right => 0,
        };
        
        if scroll_amount != 0 {
            debug!("Scrolling {} units at ({}, {})", scroll_amount, x, y);
            // In a real implementation, would use MOUSEEVENTF_WHEEL
            // For now, we'll simulate with arrow keys
            for _ in 0..scroll_amount.abs() {
                if scroll_amount > 0 {
                    self.press_key("Up").await?;
                } else {
                    self.press_key("Down").await?;
                }
                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
            }
        }
        
        Ok(())
    }
    
    /// Drag from one position to another
    async fn drag(&self, from_x: i32, from_y: i32, to_x: i32, to_y: i32) -> Result<()> {
        debug!("Dragging from ({}, {}) to ({}, {})", from_x, from_y, to_x, to_y);
        
        // Move to start position
        self.move_cursor(from_x, from_y).await?;
        
        // Press mouse down
        self.mouse_click(true).await?;
        
        // Move to end position while holding mouse
        self.move_cursor(to_x, to_y).await?;
        
        // Release mouse
        self.mouse_click(false).await?;
        
        Ok(())
    }
    
    /// Wait for specified time
    async fn wait(&self, milliseconds: u64) {
        tokio::time::sleep(tokio::time::Duration::from_millis(milliseconds)).await;
    }
    
    /// Move cursor to position
    async fn move_cursor(&self, x: i32, y: i32) -> Result<()> {
        // Clamp coordinates to screen bounds
        let x = x.clamp(0, self.screen_width - 1);
        let y = y.clamp(0, self.screen_height - 1);
        
        debug!("Moving cursor to ({}, {})", x, y);
        
        unsafe {
            SetCursorPos(x, y).map_err(|e| anyhow::anyhow!("Failed to set cursor position: {:?}", e))?;
        }
        
        // Update internal position
        {
            let mut pos = self.cursor_position.lock().await;
            *pos = (x, y);
        }
        
        // Small delay to ensure cursor movement is registered
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        
        Ok(())
    }
    
    /// Send mouse click event
    async fn mouse_click(&self, down: bool) -> Result<()> {
        let flags = if down { MOUSEEVENTF_LEFTDOWN } else { MOUSEEVENTF_LEFTUP };
        
        let input = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: windows::Win32::UI::Input::KeyboardAndMouse::INPUT_0 {
                mi: MOUSEINPUT {
                    dx: 0,
                    dy: 0,
                    mouseData: 0,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };
        
        unsafe {
            SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
        }
        
        Ok(())
    }
    
    /// Send right mouse click event
    async fn mouse_right_click(&self, down: bool) -> Result<()> {
        let flags = if down { MOUSEEVENTF_RIGHTDOWN } else { MOUSEEVENTF_RIGHTUP };
        
        let input = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: windows::Win32::UI::Input::KeyboardAndMouse::INPUT_0 {
                mi: MOUSEINPUT {
                    dx: 0,
                    dy: 0,
                    mouseData: 0,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };
        
        unsafe {
            SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
        }
        
        Ok(())
    }
    
    /// Send key down event
    async fn key_down(&self, vk_code: u16) -> Result<()> {
        let input = INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: windows::Win32::UI::Input::KeyboardAndMouse::INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY(vk_code),
                    wScan: 0,
                    dwFlags: KEYBD_EVENT_FLAGS(0),
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };
        
        unsafe {
            SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
        }
        
        Ok(())
    }
    
    /// Send key up event
    async fn key_up(&self, vk_code: u16) -> Result<()> {
        let input = INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: windows::Win32::UI::Input::KeyboardAndMouse::INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY(vk_code),
                    wScan: 0,
                    dwFlags: KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };
        
        unsafe {
            SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
        }
        
        Ok(())
    }
    
    /// Convert key name to virtual key code
    fn get_virtual_key_code(&self, key: &str) -> Result<u16> {
        let code = match key.to_lowercase().as_str() {
            "ctrl" | "control" => VK_CONTROL.0,
            "shift" => VK_SHIFT.0,
            "alt" => VK_MENU.0,
            "enter" | "return" => 0x0D,
            "space" => 0x20,
            "tab" => 0x09,
            "escape" | "esc" => 0x1B,
            "backspace" => 0x08,
            "delete" | "del" => 0x2E,
            "home" => 0x24,
            "end" => 0x23,
            "pageup" => 0x21,
            "pagedown" => 0x22,
            "up" => 0x26,
            "down" => 0x28,
            "left" => 0x25,
            "right" => 0x27,
            "f1" => 0x70,
            "f2" => 0x71,
            "f3" => 0x72,
            "f4" => 0x73,
            "f5" => 0x74,
            "f6" => 0x75,
            "f7" => 0x76,
            "f8" => 0x77,
            "f9" => 0x78,
            "f10" => 0x79,
            "f11" => 0x7A,
            "f12" => 0x7B,
            // Single characters
            key if key.len() == 1 => {
                let ch = key.chars().next().unwrap();
                match ch {
                    'a'..='z' => (ch as u8 - b'a' + 0x41) as u16,
                    '0'..='9' => (ch as u8 - b'0' + 0x30) as u16,
                    _ => return Err(anyhow::anyhow!("Unsupported key: {}", key)),
                }
            }
            _ => return Err(anyhow::anyhow!("Unknown key: {}", key)),
        };
        
        Ok(code)
    }
    
    /// Get current cursor position
    pub async fn get_cursor_position(&self) -> (i32, i32) {
        let pos = self.cursor_position.lock().await;
        *pos
    }
    
    /// Cancel all pending actions
    pub async fn cancel_all(&self) -> Result<()> {
        info!("Cancelling all pending input actions");
        let mut queue = self.action_queue.lock().await;
        queue.clear();
        Ok(())
    }
    
    /// Disable input system
    pub fn disable(&self) {
        warn!("Input system disabled");
        self.active.store(false, std::sync::atomic::Ordering::SeqCst);
    }
    
    /// Enable input system
    pub fn enable(&self) {
        info!("Input system enabled");
        self.active.store(true, std::sync::atomic::Ordering::SeqCst);
    }
    
    /// Check if input system is active
    pub fn is_active(&self) -> bool {
        self.active.load(std::sync::atomic::Ordering::SeqCst)
    }
}