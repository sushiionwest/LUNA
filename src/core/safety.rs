// Safety system: validates commands and planned actions before execution.
//
// This is intentionally conservative pattern matching, not intelligence.
// It blocks obviously destructive text commands and rejects actions with
// out-of-range parameters. The input layer applies its own per-action
// safety check and rate limiting on top of this (see crate::input).

use super::config::LunaConfig;
use super::LunaAction;
use regex::RegexSet;

/// Maximum length of a text command or typed string the agent will accept.
const MAX_TEXT_LENGTH: usize = 1000;

/// Maximum single scroll magnitude.
const MAX_SCROLL_AMOUNT: i32 = 100;

/// Maximum wait a planned action may request (milliseconds).
const MAX_WAIT_MS: u64 = 60_000;

pub struct SafetySystem {
    enabled: bool,
    blocked_patterns: RegexSet,
}

impl SafetySystem {
    pub fn new(config: &LunaConfig) -> Self {
        let patterns = [
            r"(?i)format\s+[a-z]:",
            r"(?i)rm\s+-rf",
            r"(?i)del\s+/[fqs]",
            r"(?i)rd\s+/s",
            r"(?i)shutdown",
            r"(?i)diskpart",
            r"(?i)reg\s+delete",
            r"(?i)mkfs",
        ];

        Self {
            enabled: config.safety.enabled,
            blocked_patterns: RegexSet::new(patterns)
                .expect("static safety patterns must compile"),
        }
    }

    /// Check whether a raw user command is safe to process at all.
    pub fn is_command_safe(&self, command: &str) -> bool {
        if !self.enabled {
            return true;
        }
        if command.len() > MAX_TEXT_LENGTH {
            return false;
        }
        !self.blocked_patterns.is_match(command)
    }

    /// Check whether a planned action is safe to execute.
    pub fn is_action_safe(&self, action: &LunaAction) -> bool {
        if !self.enabled {
            return true;
        }
        match action {
            LunaAction::Click { x, y } => *x >= 0 && *y >= 0,
            LunaAction::Type { text } => {
                text.len() <= MAX_TEXT_LENGTH && !self.blocked_patterns.is_match(text)
            }
            LunaAction::KeyCombo { keys } => !keys.is_empty() && keys.len() <= 5,
            LunaAction::Scroll { amount, .. } => amount.abs() <= MAX_SCROLL_AMOUNT,
            LunaAction::Wait { milliseconds } => *milliseconds <= MAX_WAIT_MS,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn system() -> SafetySystem {
        SafetySystem::new(&LunaConfig::default())
    }

    #[test]
    fn blocks_destructive_commands() {
        let s = system();
        assert!(!s.is_command_safe("please format c: for me"));
        assert!(!s.is_command_safe("rm -rf /"));
        assert!(!s.is_command_safe("shutdown the machine"));
    }

    #[test]
    fn allows_normal_commands() {
        let s = system();
        assert!(s.is_command_safe("click the save button"));
        assert!(s.is_command_safe("type \"hello world\""));
    }

    #[test]
    fn rejects_out_of_range_actions() {
        let s = system();
        assert!(!s.is_action_safe(&LunaAction::Click { x: -5, y: 10 }));
        assert!(!s.is_action_safe(&LunaAction::Scroll {
            direction: "down".to_string(),
            amount: 10_000,
        }));
        assert!(s.is_action_safe(&LunaAction::Click { x: 100, y: 100 }));
    }
}
