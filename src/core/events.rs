/*!
 * Luna Event System - Simple event handling for the portable app
 */

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use parking_lot::RwLock;
use chrono::{DateTime, Utc};

/// Luna event types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LunaEvent {
    /// System events
    SystemStarted,
    SystemStopping,
    SystemError { error: String },
    
    /// Command events
    CommandReceived { command: String },
    CommandStarted { command: String },
    CommandCompleted { command: String, success: bool },
    CommandCancelled { command: String },
    
    /// AI events
    ScreenshotTaken { width: u32, height: u32 },
    AiAnalysisStarted,
    AiAnalysisCompleted { elements_found: usize },
    AiAnalysisFailed { error: String },
    
    /// Action events
    ActionPlanned { action_type: String, target: Option<String> },
    ActionExecuted { action_type: String, success: bool },
    ActionFailed { action_type: String, error: String },
    
    /// Safety events
    UnsafeCommandBlocked { command: String, reason: String },
    SafetyCheckPassed { command: String },
    EmergencyStop,
    
    /// UI events
    OverlayShown,
    OverlayHidden,
    UserConfirmation { confirmed: bool },
    
    /// Performance events
    MemoryUsageHigh { usage_mb: u64 },
    PerformanceWarning { message: String },
}

/// Event with metadata
#[derive(Debug, Clone)]
pub struct EventEntry {
    pub event: LunaEvent,
    pub timestamp: DateTime<Utc>,
    pub id: u64,
}

/// Simple event manager for the portable app
#[derive(Debug)]
pub struct EventManager {
    events: Arc<RwLock<VecDeque<EventEntry>>>,
    next_id: Arc<parking_lot::Mutex<u64>>,
    max_events: usize,
}

impl EventManager {
    /// Create new event manager
    pub fn new(max_events: usize) -> Self {
        Self {
            events: Arc::new(RwLock::new(VecDeque::with_capacity(max_events))),
            next_id: Arc::new(parking_lot::Mutex::new(1)),
            max_events,
        }
    }
    
    /// Emit an event
    pub fn emit(&self, event: LunaEvent) {
        let id = {
            let mut next_id = self.next_id.lock();
            let id = *next_id;
            *next_id = id.wrapping_add(1);
            id
        };
        
        let entry = EventEntry {
            event,
            timestamp: Utc::now(),
            id,
        };
        
        let mut events = self.events.write();
        
        // Add new event
        events.push_back(entry);
        
        // Remove old events if we exceed max
        while events.len() > self.max_events {
            events.pop_front();
        }
        
        // Log important events
        if let Some(latest) = events.back() {
            match &latest.event {
                LunaEvent::SystemError { error } => {
                    tracing::error!("System error: {}", error);
                }
                LunaEvent::UnsafeCommandBlocked { command, reason } => {
                    tracing::warn!("Blocked unsafe command '{}': {}", command, reason);
                }
                LunaEvent::CommandCompleted { command, success } => {
                    if *success {
                        tracing::info!("Command completed: '{}'", command);
                    } else {
                        tracing::warn!("Command failed: '{}'", command);
                    }
                }
                LunaEvent::EmergencyStop => {
                    tracing::warn!("Emergency stop activated");
                }
                _ => {
                    tracing::debug!("Event: {:?}", latest.event);
                }
            }
        }
    }
    
    /// Get recent events
    pub fn get_recent(&self, count: usize) -> Vec<EventEntry> {
        let events = self.events.read();
        events.iter()
            .rev()
            .take(count)
            .cloned()
            .collect()
    }
    
    /// Get events by type
    pub fn get_by_type(&self, event_filter: impl Fn(&LunaEvent) -> bool) -> Vec<EventEntry> {
        let events = self.events.read();
        events.iter()
            .filter(|entry| event_filter(&entry.event))
            .cloned()
            .collect()
    }
    
    /// Get event count
    pub fn count(&self) -> usize {
        let events = self.events.read();
        events.len()
    }
    
    /// Clear all events
    pub fn clear(&self) {
        let mut events = self.events.write();
        events.clear();
    }
    
    /// Get system statistics
    pub fn get_stats(&self) -> EventStats {
        let events = self.events.read();
        
        let mut stats = EventStats::default();
        
        for entry in events.iter() {
            match &entry.event {
                LunaEvent::CommandCompleted { success, .. } => {
                    if *success {
                        stats.commands_succeeded += 1;
                    } else {
                        stats.commands_failed += 1;
                    }
                }
                LunaEvent::CommandCancelled { .. } => {
                    stats.commands_cancelled += 1;
                }
                LunaEvent::UnsafeCommandBlocked { .. } => {
                    stats.unsafe_commands_blocked += 1;
                }
                LunaEvent::SystemError { .. } => {
                    stats.system_errors += 1;
                }
                LunaEvent::ActionExecuted { success, .. } => {
                    if *success {
                        stats.actions_executed += 1;
                    } else {
                        stats.actions_failed += 1;
                    }
                }
                _ => {}
            }
        }
        
        stats.total_events = events.len();
        stats
    }
}

/// Event statistics
#[derive(Debug, Default, Clone)]
pub struct EventStats {
    pub total_events: usize,
    pub commands_succeeded: usize,
    pub commands_failed: usize,
    pub commands_cancelled: usize,
    pub unsafe_commands_blocked: usize,
    pub system_errors: usize,
    pub actions_executed: usize,
    pub actions_failed: usize,
}

impl LunaEvent {
    /// Get event category
    pub fn category(&self) -> &'static str {
        match self {
            LunaEvent::SystemStarted | 
            LunaEvent::SystemStopping | 
            LunaEvent::SystemError { .. } => "System",
            
            LunaEvent::CommandReceived { .. } | 
            LunaEvent::CommandStarted { .. } | 
            LunaEvent::CommandCompleted { .. } | 
            LunaEvent::CommandCancelled { .. } => "Command",
            
            LunaEvent::ScreenshotTaken { .. } | 
            LunaEvent::AiAnalysisStarted | 
            LunaEvent::AiAnalysisCompleted { .. } | 
            LunaEvent::AiAnalysisFailed { .. } => "AI",
            
            LunaEvent::ActionPlanned { .. } | 
            LunaEvent::ActionExecuted { .. } | 
            LunaEvent::ActionFailed { .. } => "Action",
            
            LunaEvent::UnsafeCommandBlocked { .. } | 
            LunaEvent::SafetyCheckPassed { .. } | 
            LunaEvent::EmergencyStop => "Safety",
            
            LunaEvent::OverlayShown | 
            LunaEvent::OverlayHidden | 
            LunaEvent::UserConfirmation { .. } => "UI",
            
            LunaEvent::MemoryUsageHigh { .. } | 
            LunaEvent::PerformanceWarning { .. } => "Performance",
        }
    }
    
    /// Get event severity
    pub fn severity(&self) -> EventSeverity {
        match self {
            LunaEvent::SystemError { .. } | LunaEvent::EmergencyStop => EventSeverity::Critical,
            LunaEvent::UnsafeCommandBlocked { .. } | LunaEvent::MemoryUsageHigh { .. } => EventSeverity::Warning,
            LunaEvent::SystemStopping | LunaEvent::CommandCancelled { .. } => EventSeverity::Warning,
            LunaEvent::CommandCompleted { success: false, .. } | LunaEvent::ActionFailed { .. } => EventSeverity::Warning,
            LunaEvent::AiAnalysisFailed { .. } | LunaEvent::PerformanceWarning { .. } => EventSeverity::Warning,
            _ => EventSeverity::Info,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventSeverity {
    Critical,
    Warning,
    Info,
}