/*!
 * Luna Visual AI Safety System
 * 
 * Comprehensive safety and security framework to protect users and systems:
 * - Dangerous action detection and prevention
 * - User confirmation workflows for risky operations
 * - Sandbox mode for testing and development
 * - Application whitelisting and blacklisting
 * - Command validation and pattern matching
 * - Emergency stop mechanisms
 * - Audit logging for security compliance
 */

use crate::core::{
    config::SafetyConfig,
    error::{LunaError, Result, RiskLevel},
    events::{self, LunaEvent, LunaEventType, EventPriority},
};
use parking_lot::RwLock;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Global safety system instance
static SAFETY_SYSTEM: once_cell::sync::Lazy<Arc<SafetySystem>> =
    once_cell::sync::Lazy::new(|| Arc::new(SafetySystem::new()));

/// Safety validation and protection system
pub struct SafetySystem {
    /// Safety configuration
    config: RwLock<SafetyConfig>,
    
    /// Compiled dangerous patterns for fast matching
    dangerous_patterns: RwLock<Vec<Regex>>,
    
    /// Pending confirmations awaiting user response
    pending_confirmations: RwLock<HashMap<Uuid, PendingConfirmation>>,
    
    /// Application whitelist and blacklist
    app_permissions: RwLock<AppPermissions>,
    
    /// Action rate limiting
    rate_limiter: RwLock<RateLimiter>,
    
    /// Safety statistics
    stats: RwLock<SafetyStats>,
    
    /// Emergency stop state
    emergency_stop: AtomicU64, // Using as boolean with timestamp
    
    /// Sandbox mode state
    sandbox_mode: AtomicU64, // Using as boolean
}

/// Pending confirmation for dangerous actions
#[derive(Debug, Clone)]
struct PendingConfirmation {
    action: ActionRequest,
    risk_level: RiskLevel,
    created_at: Instant,
    timeout: Duration,
    user_response: Option<bool>,
}

/// Action request for safety validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionRequest {
    pub id: Uuid,
    pub command: String,
    pub action_type: ActionType,
    pub target_application: Option<String>,
    pub target_elements: Vec<TargetElement>,
    pub parameters: HashMap<String, serde_json::Value>,
}

/// Types of actions Luna can perform
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionType {
    Click,
    Type,
    KeyPress,
    Drag,
    Scroll,
    ApplicationLaunch,
    FileOperation,
    SystemCommand,
    Custom(String),
}

/// Target UI element for actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetElement {
    pub element_type: String,
    pub coordinates: (i32, i32),
    pub confidence: f32,
    pub text_content: Option<String>,
    pub application: Option<String>,
}

/// Application permission management
#[derive(Debug, Clone)]
struct AppPermissions {
    allowed_apps: HashSet<String>,
    blocked_apps: HashSet<String>,
    temporary_permissions: HashMap<String, Instant>,
}

/// Rate limiting for actions
#[derive(Debug)]
struct RateLimiter {
    action_timestamps: Vec<Instant>,
    max_actions_per_minute: u32,
}

/// Safety system statistics
#[derive(Debug, Clone, Default)]
pub struct SafetyStats {
    pub actions_validated: u64,
    pub actions_approved: u64,
    pub actions_blocked: u64,
    pub confirmations_requested: u64,
    pub confirmations_approved: u64,
    pub confirmations_denied: u64,
    pub emergency_stops: u64,
    pub rate_limit_hits: u64,
}

/// Safety validation result
#[derive(Debug, Clone)]
pub struct SafetyResult {
    pub allowed: bool,
    pub risk_level: RiskLevel,
    pub reason: String,
    pub requires_confirmation: bool,
    pub confirmation_id: Option<Uuid>,
    pub auto_approve_timeout: Option<Duration>,
}

impl SafetySystem {
    fn new() -> Self {
        Self {
            config: RwLock::new(SafetyConfig::default()),
            dangerous_patterns: RwLock::new(Vec::new()),
            pending_confirmations: RwLock::new(HashMap::new()),
            app_permissions: RwLock::new(AppPermissions::new()),
            rate_limiter: RwLock::new(RateLimiter::new()),
            stats: RwLock::new(SafetyStats::default()),
            emergency_stop: AtomicU64::new(0),
            sandbox_mode: AtomicU64::new(0),
        }
    }

    /// Initialize safety system with configuration
    fn init(&self, config: SafetyConfig) -> Result<()> {
        info!("Initializing Luna Visual AI safety system");
        
        // Store configuration
        *self.config.write() = config.clone();
        
        // Set sandbox mode
        if config.sandbox_mode {
            self.sandbox_mode.store(1, Ordering::Relaxed);
            warn!("🏖️ Sandbox mode enabled - actions will be simulated only");
        }
        
        // Compile dangerous patterns
        self.compile_dangerous_patterns(&config.dangerous_patterns)?;
        
        // Initialize application permissions
        self.init_app_permissions(&config)?;
        
        // Configure rate limiter
        self.rate_limiter.write().max_actions_per_minute = 
            config.validation.rate_limit;
        
        info!("✅ Safety system initialized");
        info!("Dangerous patterns: {}", config.dangerous_patterns.len());
        info!("Allowed apps: {}", config.allowed_apps.len());
        info!("Blocked apps: {}", config.blocked_apps.len());
        info!("Rate limit: {} actions/minute", config.validation.rate_limit);
        
        Ok(())
    }

    /// Compile dangerous patterns into regex objects
    fn compile_dangerous_patterns(&self, patterns: &[String]) -> Result<()> {
        let mut compiled_patterns = Vec::new();
        
        for pattern in patterns {
            match Regex::new(pattern) {
                Ok(regex) => compiled_patterns.push(regex),
                Err(e) => {
                    error!("Failed to compile dangerous pattern '{}': {}", pattern, e);
                    return Err(LunaError::safety(
                        format!("Invalid regex pattern: {}", e),
                        RiskLevel::Medium,
                        false,
                    ));
                }
            }
        }
        
        *self.dangerous_patterns.write() = compiled_patterns;
        Ok(())
    }

    /// Initialize application permissions
    fn init_app_permissions(&self, config: &SafetyConfig) -> Result<()> {
        let mut permissions = self.app_permissions.write();
        
        permissions.allowed_apps = config.allowed_apps.iter().cloned().collect();
        permissions.blocked_apps = config.blocked_apps.iter().cloned().collect();
        
        // Default blocked applications for security
        let default_blocked = vec![
            "cmd.exe".to_string(),
            "powershell.exe".to_string(),
            "regedit.exe".to_string(),
            "msconfig.exe".to_string(),
            "services.msc".to_string(),
            "gpedit.msc".to_string(),
        ];
        
        for app in default_blocked {
            permissions.blocked_apps.insert(app);
        }
        
        Ok(())
    }

    /// Validate an action request
    async fn validate_action(&self, action: &ActionRequest) -> Result<SafetyResult> {
        debug!("Validating action: {:?}", action.action_type);
        
        // Update statistics
        self.stats.write().actions_validated += 1;
        
        // Check emergency stop
        if self.emergency_stop.load(Ordering::Relaxed) != 0 {
            return Ok(SafetyResult {
                allowed: false,
                risk_level: RiskLevel::Critical,
                reason: "Emergency stop is active".to_string(),
                requires_confirmation: false,
                confirmation_id: None,
                auto_approve_timeout: None,
            });
        }
        
        // Check rate limiting
        if let Err(e) = self.check_rate_limit().await {
            self.stats.write().rate_limit_hits += 1;
            return Ok(SafetyResult {
                allowed: false,
                risk_level: RiskLevel::Medium,
                reason: e.to_string(),
                requires_confirmation: false,
                confirmation_id: None,
                auto_approve_timeout: None,
            });
        }
        
        // Check if safety is disabled
        let config = self.config.read();
        if !config.enabled {
            return Ok(SafetyResult {
                allowed: true,
                risk_level: RiskLevel::Low,
                reason: "Safety checks disabled".to_string(),
                requires_confirmation: false,
                confirmation_id: None,
                auto_approve_timeout: None,
            });
        }
        
        // Check application permissions
        if let Some(ref app) = action.target_application {
            if let Err(risk) = self.check_app_permissions(app) {
                self.stats.write().actions_blocked += 1;
                return Ok(SafetyResult {
                    allowed: false,
                    risk_level: risk,
                    reason: format!("Application '{}' is not permitted", app),
                    requires_confirmation: false,
                    confirmation_id: None,
                    auto_approve_timeout: None,
                });
            }
        }
        
        // Check dangerous patterns
        let risk_level = self.check_dangerous_patterns(&action.command);
        
        // Determine if confirmation is required
        let requires_confirmation = match risk_level {
            RiskLevel::Low => false,
            RiskLevel::Medium => config.confirm_dangerous,
            RiskLevel::High | RiskLevel::Critical => true,
        };
        
        if requires_confirmation {
            // Create pending confirmation
            let confirmation_id = Uuid::new_v4();
            let timeout = Duration::from_secs(config.confirmation_timeout as u64);
            
            let pending = PendingConfirmation {
                action: action.clone(),
                risk_level,
                created_at: Instant::now(),
                timeout,
                user_response: None,
            };
            
            self.pending_confirmations.write().insert(confirmation_id, pending);
            self.stats.write().confirmations_requested += 1;
            
            // Publish confirmation request event
            let event = LunaEvent::new(
                LunaEventType::SafetyCheckStarted,
                "safety".to_string(),
                EventPriority::High,
                serde_json::json!({
                    "confirmation_id": confirmation_id,
                    "action": action,
                    "risk_level": risk_level,
                    "timeout_seconds": config.confirmation_timeout
                }),
            );
            
            if let Err(e) = events::publish(event) {
                warn!("Failed to publish confirmation request event: {}", e);
            }
            
            return Ok(SafetyResult {
                allowed: false,
                risk_level,
                reason: "Action requires user confirmation".to_string(),
                requires_confirmation: true,
                confirmation_id: Some(confirmation_id),
                auto_approve_timeout: Some(timeout),
            });
        }
        
        // Action is allowed without confirmation
        self.stats.write().actions_approved += 1;
        
        Ok(SafetyResult {
            allowed: true,
            risk_level,
            reason: "Action validated and approved".to_string(),
            requires_confirmation: false,
            confirmation_id: None,
            auto_approve_timeout: None,
        })
    }

    /// Check rate limiting
    async fn check_rate_limit(&self) -> Result<()> {
        let mut limiter = self.rate_limiter.write();
        let now = Instant::now();
        let one_minute_ago = now - Duration::from_secs(60);
        
        // Remove old timestamps
        limiter.action_timestamps.retain(|&ts| ts > one_minute_ago);
        
        // Check if we're over the limit
        if limiter.action_timestamps.len() >= limiter.max_actions_per_minute as usize {
            return Err(LunaError::safety(
                "Rate limit exceeded",
                RiskLevel::Medium,
                false,
            ));
        }
        
        // Add current timestamp
        limiter.action_timestamps.push(now);
        
        Ok(())
    }

    /// Check application permissions
    fn check_app_permissions(&self, app: &str) -> Result<(), RiskLevel> {
        let permissions = self.app_permissions.read();
        
        // Check if explicitly blocked
        if permissions.blocked_apps.contains(app) {
            return Err(RiskLevel::High);
        }
        
        // If whitelist is not empty, check if app is allowed
        if !permissions.allowed_apps.is_empty() && !permissions.allowed_apps.contains(app) {
            return Err(RiskLevel::Medium);
        }
        
        Ok(())
    }

    /// Check dangerous patterns in command
    fn check_dangerous_patterns(&self, command: &str) -> RiskLevel {
        let patterns = self.dangerous_patterns.read();
        
        for pattern in patterns.iter() {
            if pattern.is_match(command) {
                debug!("Dangerous pattern matched: {}", pattern.as_str());
                
                // Classify risk level based on pattern
                let pattern_str = pattern.as_str();
                if pattern_str.contains("delete") || pattern_str.contains("format") || 
                   pattern_str.contains("shutdown") {
                    return RiskLevel::Critical;
                } else if pattern_str.contains("install") || pattern_str.contains("uninstall") {
                    return RiskLevel::High;
                } else {
                    return RiskLevel::Medium;
                }
            }
        }
        
        RiskLevel::Low
    }

    /// Process user confirmation response
    async fn process_confirmation(&self, confirmation_id: Uuid, approved: bool) -> Result<()> {
        let mut confirmations = self.pending_confirmations.write();
        
        if let Some(mut confirmation) = confirmations.remove(&confirmation_id) {
            confirmation.user_response = Some(approved);
            
            if approved {
                self.stats.write().confirmations_approved += 1;
                self.stats.write().actions_approved += 1;
                
                // Publish approval event
                let event = LunaEvent::new(
                    LunaEventType::SafetyCheckPassed,
                    "safety".to_string(),
                    EventPriority::Normal,
                    serde_json::json!({
                        "confirmation_id": confirmation_id,
                        "action": confirmation.action
                    }),
                );
                
                events::publish(event)?;
                
                info!("Action approved by user: {}", confirmation.action.command);
            } else {
                self.stats.write().confirmations_denied += 1;
                self.stats.write().actions_blocked += 1;
                
                // Publish denial event
                let event = LunaEvent::new(
                    LunaEventType::SafetyCheckFailed,
                    "safety".to_string(),
                    EventPriority::Normal,
                    serde_json::json!({
                        "confirmation_id": confirmation_id,
                        "action": confirmation.action,
                        "reason": "User denied confirmation"
                    }),
                );
                
                events::publish(event)?;
                
                info!("Action denied by user: {}", confirmation.action.command);
            }
        } else {
            warn!("Confirmation ID not found or already processed: {}", confirmation_id);
        }
        
        Ok(())
    }

    /// Trigger emergency stop
    async fn emergency_stop(&self) -> Result<()> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)
            .unwrap().as_secs();
        self.emergency_stop.store(now, Ordering::Relaxed);
        self.stats.write().emergency_stops += 1;
        
        error!("🚨 EMERGENCY STOP ACTIVATED");
        
        // Clear all pending confirmations
        self.pending_confirmations.write().clear();
        
        // Publish emergency stop event
        let event = LunaEvent::new(
            LunaEventType::UnsafeActionBlocked,
            "safety".to_string(),
            EventPriority::Critical,
            serde_json::json!({
                "reason": "Emergency stop activated",
                "timestamp": now
            }),
        );
        
        events::publish(event)?;
        
        Ok(())
    }

    /// Clear emergency stop
    async fn clear_emergency_stop(&self) -> Result<()> {
        self.emergency_stop.store(0, Ordering::Relaxed);
        info!("Emergency stop cleared");
        Ok(())
    }

    /// Check if sandbox mode is enabled
    fn is_sandbox_mode(&self) -> bool {
        self.sandbox_mode.load(Ordering::Relaxed) != 0
    }

    /// Get safety statistics
    fn get_stats(&self) -> SafetyStats {
        self.stats.read().clone()
    }

    /// Clean up expired confirmations
    async fn cleanup_expired_confirmations(&self) -> Result<()> {
        let mut confirmations = self.pending_confirmations.write();
        let now = Instant::now();
        
        let before_count = confirmations.len();
        confirmations.retain(|_, confirmation| {
            now.duration_since(confirmation.created_at) < confirmation.timeout
        });
        let after_count = confirmations.len();
        
        if before_count != after_count {
            debug!("Cleaned up {} expired confirmations", before_count - after_count);
        }
        
        Ok(())
    }
}

impl AppPermissions {
    fn new() -> Self {
        Self {
            allowed_apps: HashSet::new(),
            blocked_apps: HashSet::new(),
            temporary_permissions: HashMap::new(),
        }
    }
}

impl RateLimiter {
    fn new() -> Self {
        Self {
            action_timestamps: Vec::new(),
            max_actions_per_minute: 60,
        }
    }
}

impl ActionRequest {
    /// Create a new action request
    pub fn new(
        command: String,
        action_type: ActionType,
        target_application: Option<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            command,
            action_type,
            target_application,
            target_elements: Vec::new(),
            parameters: HashMap::new(),
        }
    }

    /// Add a target element
    pub fn add_target(&mut self, element: TargetElement) {
        self.target_elements.push(element);
    }

    /// Add a parameter
    pub fn add_parameter(&mut self, key: String, value: serde_json::Value) {
        self.parameters.insert(key, value);
    }
}

// Public API functions

/// Initialize the safety system
pub fn init(config: SafetyConfig) -> Result<()> {
    SAFETY_SYSTEM.init(config)
}

/// Shutdown the safety system
pub async fn shutdown() -> Result<()> {
    info!("Shutting down safety system");
    
    // Clear emergency stop
    SAFETY_SYSTEM.clear_emergency_stop().await?;
    
    // Clear all pending confirmations
    SAFETY_SYSTEM.pending_confirmations.write().clear();
    
    info!("✅ Safety system shut down");
    Ok(())
}

/// Validate system health
pub async fn validate_system() -> Result<()> {
    // Check if patterns are compiled
    let patterns = SAFETY_SYSTEM.dangerous_patterns.read();
    if patterns.is_empty() {
        warn!("No dangerous patterns loaded");
    }
    
    // Clean up expired confirmations
    SAFETY_SYSTEM.cleanup_expired_confirmations().await?;
    
    Ok(())
}

/// Validate an action for safety
pub async fn validate_action(action: &ActionRequest) -> Result<SafetyResult> {
    SAFETY_SYSTEM.validate_action(action).await
}

/// Process user confirmation response
pub async fn process_confirmation(confirmation_id: Uuid, approved: bool) -> Result<()> {
    SAFETY_SYSTEM.process_confirmation(confirmation_id, approved).await
}

/// Trigger emergency stop
pub async fn emergency_stop() -> Result<()> {
    SAFETY_SYSTEM.emergency_stop().await
}

/// Clear emergency stop
pub async fn clear_emergency_stop() -> Result<()> {
    SAFETY_SYSTEM.clear_emergency_stop().await
}

/// Check if sandbox mode is enabled
pub fn is_sandbox_mode() -> bool {
    SAFETY_SYSTEM.is_sandbox_mode()
}

/// Get safety statistics
pub fn get_stats() -> SafetyStats {
    SAFETY_SYSTEM.get_stats()
}

/// Quick validation for simple commands
pub async fn quick_validate(command: &str, action_type: ActionType) -> Result<bool> {
    let action = ActionRequest::new(command.to_string(), action_type, None);
    let result = validate_action(&action).await?;
    Ok(result.allowed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_safety_init() {
        let config = SafetyConfig::default();
        let result = init(config);
        assert!(result.is_ok());
        
        let validation = validate_system().await;
        assert!(validation.is_ok());
        
        shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_dangerous_pattern_detection() {
        let mut config = SafetyConfig::default();
        config.dangerous_patterns = vec![r"delete.*".to_string()];
        
        init(config).unwrap();
        
        let action = ActionRequest::new(
            "delete important file".to_string(),
            ActionType::FileOperation,
            None,
        );
        
        let result = validate_action(&action).await.unwrap();
        assert!(!result.allowed || result.requires_confirmation);
        
        shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_emergency_stop() {
        let config = SafetyConfig::default();
        init(config).unwrap();
        
        emergency_stop().await.unwrap();
        
        let action = ActionRequest::new(
            "safe command".to_string(),
            ActionType::Click,
            None,
        );
        
        let result = validate_action(&action).await.unwrap();
        assert!(!result.allowed);
        assert_eq!(result.risk_level, RiskLevel::Critical);
        
        clear_emergency_stop().await.unwrap();
        shutdown().await.unwrap();
    }

    #[test]
    fn test_action_request_creation() {
        let mut action = ActionRequest::new(
            "test command".to_string(),
            ActionType::Click,
            Some("notepad.exe".to_string()),
        );
        
        action.add_target(TargetElement {
            element_type: "button".to_string(),
            coordinates: (100, 200),
            confidence: 0.95,
            text_content: Some("OK".to_string()),
            application: Some("notepad.exe".to_string()),
        });
        
        action.add_parameter("test".to_string(), serde_json::json!("value"));
        
        assert_eq!(action.command, "test command");
        assert_eq!(action.target_elements.len(), 1);
        assert_eq!(action.parameters.len(), 1);
    }
}