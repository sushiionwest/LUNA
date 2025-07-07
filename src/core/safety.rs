/*!
 * Luna Safety System - Protects users from dangerous operations
 */

use crate::core::{LunaConfig, LunaAction};
use regex::Regex;
use std::collections::HashSet;
use tracing::{warn, debug};

/// Safety system to prevent dangerous operations
#[derive(Debug)]
pub struct SafetySystem {
    /// Whether safety checks are enabled
    enabled: bool,
    /// Blocked command patterns
    blocked_patterns: Vec<Regex>,
    /// Blocked keywords
    blocked_keywords: HashSet<String>,
    /// Maximum actions per command
    max_actions: usize,
    /// Safe application whitelist
    safe_apps: HashSet<String>,
    /// Dangerous file paths
    dangerous_paths: HashSet<String>,
}

impl SafetySystem {
    /// Create new safety system
    pub fn new(config: &LunaConfig) -> Self {
        let mut blocked_patterns = Vec::new();
        
        // Compile dangerous patterns
        let dangerous_patterns = [
            r"(?i)\bdelete\s+.*\bsystem\b",
            r"(?i)\bformat\s+[c-z]:",
            r"(?i)\brm\s+-rf\s+/",
            r"(?i)\bdel\s+/[qsf]\s+",
            r"(?i)\bshutdown\s+/[sir]",
            r"(?i)\brestart\s+/[fr]",
            r"(?i)\breg\s+delete\s+",
            r"(?i)\bregsvr32\s+",
            r"(?i)\bpowershell\s+.*\bremove\b",
            r"(?i)\bcmd\s+.*\b/c\b.*\bdel\b",
            r"(?i)\btaskkill\s+.*\b/f\b",
            r"(?i)\bnet\s+user\s+.*\bdelete\b",
        ];
        
        for pattern in &dangerous_patterns {
            if let Ok(regex) = Regex::new(pattern) {
                blocked_patterns.push(regex);
            }
        }
        
        // Blocked keywords
        let blocked_keywords: HashSet<String> = config.safety.blocked_keywords
            .iter()
            .map(|s| s.to_lowercase())
            .collect();
        
        // Safe applications
        let safe_apps: HashSet<String> = [
            "notepad.exe",
            "calculator.exe", 
            "mspaint.exe",
            "wordpad.exe",
            "firefox.exe",
            "chrome.exe",
            "msedge.exe",
            "explorer.exe",
            "winword.exe",
            "excel.exe",
            "powerpoint.exe",
            "teams.exe",
            "discord.exe",
            "spotify.exe",
        ].iter().map(|s| s.to_lowercase()).collect();
        
        // Dangerous paths
        let dangerous_paths: HashSet<String> = [
            "c:\\windows\\system32",
            "c:\\windows\\syswow64",
            "c:\\program files",
            "c:\\program files (x86)",
            "c:\\windows",
            "c:\\boot",
            "c:\\recovery",
        ].iter().map(|s| s.to_lowercase()).collect();
        
        Self {
            enabled: config.safety.enabled,
            blocked_patterns,
            blocked_keywords,
            max_actions: config.safety.max_actions_per_command,
            safe_apps,
            dangerous_paths,
        }
    }
    
    /// Check if a command is safe to execute
    pub fn is_command_safe(&self, command: &str) -> bool {
        if !self.enabled {
            debug!("Safety system disabled, allowing command: {}", command);
            return true;
        }
        
        let command_lower = command.to_lowercase();
        
        // Check blocked patterns
        for pattern in &self.blocked_patterns {
            if pattern.is_match(&command_lower) {
                warn!("Command blocked by pattern: {}", command);
                return false;
            }
        }
        
        // Check blocked keywords
        for keyword in &self.blocked_keywords {
            if command_lower.contains(keyword) {
                warn!("Command blocked by keyword '{}': {}", keyword, command);
                return false;
            }
        }
        
        // Additional safety checks
        if self.is_system_command(&command_lower) {
            warn!("System command blocked: {}", command);
            return false;
        }
        
        if self.is_file_deletion(&command_lower) {
            warn!("File deletion command blocked: {}", command);
            return false;
        }
        
        if self.is_registry_modification(&command_lower) {
            warn!("Registry modification blocked: {}", command);
            return false;
        }
        
        debug!("Command safety check passed: {}", command);
        true
    }
    
    /// Check if actions are safe to execute
    pub fn are_actions_safe(&self, actions: &[LunaAction]) -> bool {
        if !self.enabled {
            return true;
        }
        
        // Check action count
        if actions.len() > self.max_actions {
            warn!("Too many actions planned: {} (max: {})", actions.len(), self.max_actions);
            return false;
        }
        
        // Check individual actions
        for action in actions {
            if !self.is_action_safe(action) {
                return false;
            }
        }
        
        true
    }
    
    /// Check if a single action is safe
    pub fn is_action_safe(&self, action: &LunaAction) -> bool {
        if !self.enabled {
            return true;
        }
        
        match action {
            LunaAction::Type { text } => {
                // Check if typing dangerous commands
                if self.contains_dangerous_text(text) {
                    warn!("Dangerous text blocked: {}", text);
                    return false;
                }
            }
            LunaAction::KeyCombo { keys } => {
                // Block dangerous key combinations
                let combo = keys.join("+").to_lowercase();
                if self.is_dangerous_key_combo(&combo) {
                    warn!("Dangerous key combo blocked: {}", combo);
                    return false;
                }
            }
            LunaAction::Click { x, y } |
            LunaAction::RightClick { x, y } |
            LunaAction::DoubleClick { x, y } => {
                // Check click coordinates for dangerous areas
                if self.is_dangerous_click_area(*x, *y) {
                    warn!("Click in dangerous area blocked: ({}, {})", x, y);
                    return false;
                }
            }
            _ => {}
        }
        
        true
    }
    
    /// Check if text contains dangerous content
    fn contains_dangerous_text(&self, text: &str) -> bool {
        let text_lower = text.to_lowercase();
        
        // Check for dangerous commands
        let dangerous_commands = [
            "del /q /s",
            "format c:",
            "shutdown /s",
            "restart /f",
            "rm -rf",
            "reg delete",
            "taskkill /f",
        ];
        
        for cmd in &dangerous_commands {
            if text_lower.contains(cmd) {
                return true;
            }
        }
        
        // Check for paths to dangerous directories
        for path in &self.dangerous_paths {
            if text_lower.contains(path) {
                return true;
            }
        }
        
        false
    }
    
    /// Check if key combination is dangerous
    fn is_dangerous_key_combo(&self, combo: &str) -> bool {
        let dangerous_combos = [
            "ctrl+alt+del",
            "alt+f4", // Only when targeting system apps
            "win+r", // Run dialog can be dangerous
            "ctrl+shift+esc", // Task manager
        ];
        
        dangerous_combos.iter().any(|&dangerous| combo.contains(dangerous))
    }
    
    /// Check if click area is dangerous (approximation)
    fn is_dangerous_click_area(&self, x: i32, y: i32) -> bool {
        // This is a simplified check - in practice you'd need to analyze
        // what window/control is at these coordinates
        
        // Avoid taskbar area (bottom 40 pixels)
        let screen_height = 1080; // Would get actual screen height in real implementation
        if y > screen_height - 40 {
            return false; // Actually safer to avoid taskbar for now
        }
        
        // Avoid far corners (could be close/minimize buttons on critical apps)
        if (x < 50 && y < 50) || (x > 1920 - 50 && y < 50) {
            return true;
        }
        
        false
    }
    
    /// Check if command is a system command
    fn is_system_command(&self, command: &str) -> bool {
        let system_commands = [
            "shutdown", "restart", "reboot", "format", "fdisk",
            "diskpart", "bcdedit", "bcdboot", "sfc", "dism",
            "reg delete", "regsvr32", "gpedit", "regedit",
        ];
        
        system_commands.iter().any(|&cmd| command.contains(cmd))
    }
    
    /// Check if command involves file deletion
    fn is_file_deletion(&self, command: &str) -> bool {
        let deletion_patterns = [
            "delete", "remove", "del ", "erase", "rm ",
            "uninstall", "clean", "wipe", "purge",
        ];
        
        // Check if it's deletion AND involves system paths
        let has_deletion = deletion_patterns.iter().any(|&pattern| command.contains(pattern));
        let has_system_path = self.dangerous_paths.iter().any(|path| command.contains(path));
        
        has_deletion && (has_system_path || command.contains("program") || command.contains("windows"))
    }
    
    /// Check if command modifies registry
    fn is_registry_modification(&self, command: &str) -> bool {
        command.contains("reg ") || 
        command.contains("registry") || 
        command.contains("regedit") ||
        command.contains("hkey_")
    }
    
    /// Check if application is in safe list
    pub fn is_safe_application(&self, app_name: &str) -> bool {
        self.safe_apps.contains(&app_name.to_lowercase())
    }
    
    /// Get safety status
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
    
    /// Temporarily disable safety (for advanced users)
    pub fn disable(&mut self) {
        warn!("Safety system disabled!");
        self.enabled = false;
    }
    
    /// Re-enable safety
    pub fn enable(&mut self) {
        self.enabled = true;
        debug!("Safety system enabled");
    }
    
    /// Get safety statistics
    pub fn get_stats(&self) -> SafetyStats {
        SafetyStats {
            enabled: self.enabled,
            blocked_patterns_count: self.blocked_patterns.len(),
            blocked_keywords_count: self.blocked_keywords.len(),
            safe_apps_count: self.safe_apps.len(),
            max_actions: self.max_actions,
        }
    }
}

/// Safety system statistics
#[derive(Debug, Clone)]
pub struct SafetyStats {
    pub enabled: bool,
    pub blocked_patterns_count: usize,
    pub blocked_keywords_count: usize,
    pub safe_apps_count: usize,
    pub max_actions: usize,
}

/// Safety check result
#[derive(Debug, Clone)]
pub enum SafetyResult {
    Safe,
    Blocked { reason: String },
    Warning { message: String },
}

impl SafetyResult {
    pub fn is_safe(&self) -> bool {
        matches!(self, SafetyResult::Safe)
    }
    
    pub fn is_blocked(&self) -> bool {
        matches!(self, SafetyResult::Blocked { .. })
    }
    
    pub fn get_message(&self) -> Option<&str> {
        match self {
            SafetyResult::Safe => None,
            SafetyResult::Blocked { reason } => Some(reason),
            SafetyResult::Warning { message } => Some(message),
        }
    }
}