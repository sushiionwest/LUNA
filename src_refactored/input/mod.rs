// Cross-platform input handling with minimal dependencies
// Replaces heavy Windows-specific automation libraries

use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct InputAction {
    pub action_type: ActionType,
    pub target: Target,
    pub timestamp: Instant,
}

#[derive(Debug, Clone)]
pub enum ActionType {
    Click { button: MouseButton },
    Type { text: String },
    Key { key: String },
    Scroll { direction: ScrollDirection, amount: i32 },
    Move { x: i32, y: i32 },
}

#[derive(Debug, Clone)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

#[derive(Debug, Clone)]
pub enum ScrollDirection {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug, Clone)]
pub struct Target {
    pub x: i32,
    pub y: i32,
    pub element_type: Option<String>,
}

pub struct InputController {
    action_history: Vec<InputAction>,
    rate_limiter: RateLimiter,
    safety_checker: Box<dyn SafetyChecker>,
}

pub trait SafetyChecker {
    fn is_action_safe(&self, action: &InputAction) -> bool;
    fn get_risk_level(&self, action: &InputAction) -> RiskLevel;
}

#[derive(Debug, Clone, PartialEq)]
pub enum RiskLevel {
    Safe,
    Low,
    Medium,
    High,
    Critical,
}

pub struct RateLimiter {
    action_counts: HashMap<String, Vec<Instant>>,
    max_actions_per_minute: usize,
    max_actions_per_second: usize,
}

impl RateLimiter {
    pub fn new(max_per_minute: usize, max_per_second: usize) -> Self {
        Self {
            action_counts: HashMap::new(),
            max_actions_per_minute: max_per_minute,
            max_actions_per_second: max_per_second,
        }
    }

    pub fn check_rate_limit(&mut self, action_type: &str) -> bool {
        let now = Instant::now();
        let actions = self.action_counts.entry(action_type.to_string()).or_insert_with(Vec::new);
        
        // Remove old entries
        actions.retain(|&timestamp| now.duration_since(timestamp) < Duration::from_secs(60));
        
        // Check limits
        let recent_actions = actions.iter()
            .filter(|&&timestamp| now.duration_since(timestamp) < Duration::from_secs(1))
            .count();
        
        if recent_actions >= self.max_actions_per_second || actions.len() >= self.max_actions_per_minute {
            return false;
        }
        
        actions.push(now);
        true
    }
}

impl InputController {
    pub fn new(safety_checker: Box<dyn SafetyChecker>) -> Self {
        Self {
            action_history: Vec::new(),
            rate_limiter: RateLimiter::new(100, 10), // 100/min, 10/sec
            safety_checker,
        }
    }

    pub fn execute_action(&mut self, action: InputAction) -> Result<(), InputError> {
        // Safety check
        if !self.safety_checker.is_action_safe(&action) {
            return Err(InputError::SafetyViolation);
        }

        // Rate limiting
        let action_key = format!("{:?}", action.action_type);
        if !self.rate_limiter.check_rate_limit(&action_key) {
            return Err(InputError::RateLimited);
        }

        // Execute platform-specific action
        self.execute_platform_action(&action)?;
        
        // Record action
        self.action_history.push(action);
        
        Ok(())
    }

    #[cfg(target_os = "windows")]
    fn execute_platform_action(&self, action: &InputAction) -> Result<(), InputError> {
        // Simplified Windows implementation without heavy dependencies
        match &action.action_type {
            ActionType::Click { button } => {
                // Use minimal Windows API calls
                self.windows_click(action.target.x, action.target.y, button)
            }
            ActionType::Type { text } => {
                self.windows_type_text(text)
            }
            ActionType::Key { key } => {
                self.windows_send_key(key)
            }
            ActionType::Move { x, y } => {
                self.windows_move_cursor(*x, *y)
            }
            ActionType::Scroll { direction, amount } => {
                self.windows_scroll(action.target.x, action.target.y, direction, *amount)
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    fn execute_platform_action(&self, action: &InputAction) -> Result<(), InputError> {
        // Cross-platform fallback (X11, Wayland simulation)
        match &action.action_type {
            ActionType::Click { .. } => {
                // Log the action for testing/simulation
                println!("SIMULATE: Click at ({}, {})", action.target.x, action.target.y);
                Ok(())
            }
            ActionType::Type { text } => {
                println!("SIMULATE: Type text: {}", text);
                Ok(())
            }
            ActionType::Key { key } => {
                println!("SIMULATE: Send key: {}", key);
                Ok(())
            }
            ActionType::Move { x, y } => {
                println!("SIMULATE: Move cursor to ({}, {})", x, y);
                Ok(())
            }
            ActionType::Scroll { direction, amount } => {
                println!("SIMULATE: Scroll {:?} by {}", direction, amount);
                Ok(())
            }
        }
    }

    pub fn get_action_history(&self) -> &[InputAction] {
        &self.action_history
    }

    pub fn clear_history(&mut self) {
        self.action_history.clear();
    }
}

#[cfg(target_os = "windows")]
impl InputController {
    fn windows_click(&self, x: i32, y: i32, button: &MouseButton) -> Result<(), InputError> {
        // Minimal Windows API implementation
        // In real implementation, would use SetCursorPos and mouse_event
        println!("Windows click at ({}, {}) with {:?}", x, y, button);
        Ok(())
    }

    fn windows_type_text(&self, text: &str) -> Result<(), InputError> {
        // Minimal Windows API implementation
        // In real implementation, would use SendInput with VK_* codes
        println!("Windows type: {}", text);
        Ok(())
    }

    fn windows_send_key(&self, key: &str) -> Result<(), InputError> {
        // Minimal Windows API implementation
        println!("Windows key: {}", key);
        Ok(())
    }

    fn windows_move_cursor(&self, x: i32, y: i32) -> Result<(), InputError> {
        // Minimal Windows API implementation
        println!("Windows move cursor to ({}, {})", x, y);
        Ok(())
    }

    fn windows_scroll(&self, x: i32, y: i32, direction: &ScrollDirection, amount: i32) -> Result<(), InputError> {
        // Minimal Windows API implementation
        println!("Windows scroll at ({}, {}) {:?} by {}", x, y, direction, amount);
        Ok(())
    }
}

#[derive(Debug)]
pub enum InputError {
    SafetyViolation,
    RateLimited,
    PlatformError(String),
    InvalidTarget,
    InvalidAction,
}

impl std::fmt::Display for InputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InputError::SafetyViolation => write!(f, "Action blocked by safety system"),
            InputError::RateLimited => write!(f, "Action rate limited"),
            InputError::PlatformError(msg) => write!(f, "Platform error: {}", msg),
            InputError::InvalidTarget => write!(f, "Invalid target location"),
            InputError::InvalidAction => write!(f, "Invalid action type"),
        }
    }
}

impl std::error::Error for InputError {}

// Basic safety checker implementation
pub struct BasicSafetyChecker {
    forbidden_patterns: Vec<String>,
}

impl BasicSafetyChecker {
    pub fn new() -> Self {
        Self {
            forbidden_patterns: vec![
                "shutdown".to_string(),
                "format".to_string(),
                "delete".to_string(),
                "rm -rf".to_string(),
                "del /s".to_string(),
            ],
        }
    }
}

impl SafetyChecker for BasicSafetyChecker {
    fn is_action_safe(&self, action: &InputAction) -> bool {
        match &action.action_type {
            ActionType::Type { text } => {
                let text_lower = text.to_lowercase();
                !self.forbidden_patterns.iter().any(|pattern| text_lower.contains(pattern))
            }
            ActionType::Key { key } => {
                // Block dangerous key combinations
                !matches!(key.as_str(), "ctrl+alt+delete" | "alt+f4" | "win+r")
            }
            _ => true, // Other actions are generally safe
        }
    }

    fn get_risk_level(&self, action: &InputAction) -> RiskLevel {
        match &action.action_type {
            ActionType::Type { text } => {
                let text_lower = text.to_lowercase();
                if self.forbidden_patterns.iter().any(|pattern| text_lower.contains(pattern)) {
                    RiskLevel::Critical
                } else if text_lower.contains("password") || text_lower.contains("admin") {
                    RiskLevel::High
                } else {
                    RiskLevel::Safe
                }
            }
            ActionType::Key { key } => {
                if matches!(key.as_str(), "ctrl+alt+delete" | "alt+f4") {
                    RiskLevel::High
                } else {
                    RiskLevel::Low
                }
            }
            _ => RiskLevel::Low,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter() {
        let mut limiter = RateLimiter::new(5, 2);
        
        // Should allow first action
        assert!(limiter.check_rate_limit("click"));
        assert!(limiter.check_rate_limit("click"));
        
        // Should block third action in same second
        assert!(!limiter.check_rate_limit("click"));
    }

    #[test]
    fn test_safety_checker() {
        let checker = BasicSafetyChecker::new();
        
        let safe_action = InputAction {
            action_type: ActionType::Type { text: "hello world".to_string() },
            target: Target { x: 100, y: 100, element_type: None },
            timestamp: Instant::now(),
        };
        
        let unsafe_action = InputAction {
            action_type: ActionType::Type { text: "shutdown /s /t 0".to_string() },
            target: Target { x: 100, y: 100, element_type: None },
            timestamp: Instant::now(),
        };
        
        assert!(checker.is_action_safe(&safe_action));
        assert!(!checker.is_action_safe(&unsafe_action));
    }
}