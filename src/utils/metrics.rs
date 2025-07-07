/*!
 * Luna Metrics Collection - Performance and usage metrics
 */

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

/// Metrics collector for Luna performance tracking
pub struct MetricsCollector {
    /// Performance metrics
    performance: Arc<RwLock<PerformanceMetrics>>,
    /// Usage metrics
    usage: Arc<RwLock<UsageMetrics>>,
    /// Error metrics
    errors: Arc<RwLock<ErrorMetrics>>,
    /// Custom counters
    counters: Arc<RwLock<HashMap<String, u64>>>,
    /// Custom timers
    timers: Arc<RwLock<HashMap<String, Vec<Duration>>>>,
    /// Start time for uptime calculation
    start_time: Instant,
}

/// Performance metrics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub cpu_usage_percent: f64,
    pub memory_usage_mb: u64,
    pub screen_capture_time_ms: Vec<u64>,
    pub ai_processing_time_ms: Vec<u64>,
    pub action_execution_time_ms: Vec<u64>,
    pub total_processing_time_ms: u64,
    pub frames_per_second: f64,
}

/// Usage metrics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct UsageMetrics {
    pub commands_executed: u64,
    pub commands_successful: u64,
    pub commands_failed: u64,
    pub clicks_performed: u64,
    pub keys_pressed: u64,
    pub screenshots_taken: u64,
    pub ai_inferences: u64,
    pub session_duration_seconds: u64,
    pub most_used_commands: HashMap<String, u64>,
}

/// Error metrics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ErrorMetrics {
    pub total_errors: u64,
    pub ai_errors: u64,
    pub input_errors: u64,
    pub capture_errors: u64,
    pub safety_blocks: u64,
    pub error_types: HashMap<String, u64>,
    pub recent_errors: Vec<ErrorEntry>,
}

/// Error entry for tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorEntry {
    pub timestamp: u64,
    pub error_type: String,
    pub message: String,
    pub operation: String,
}

impl MetricsCollector {
    /// Create new metrics collector
    pub fn new() -> Self {
        debug!("Initializing metrics collector");
        
        Self {
            performance: Arc::new(RwLock::new(PerformanceMetrics::default())),
            usage: Arc::new(RwLock::new(UsageMetrics::default())),
            errors: Arc::new(RwLock::new(ErrorMetrics::default())),
            counters: Arc::new(RwLock::new(HashMap::new())),
            timers: Arc::new(RwLock::new(HashMap::new())),
            start_time: Instant::now(),
        }
    }
    
    /// Record command execution
    pub fn record_command(&self, command: &str, success: bool, duration_ms: u64) {
        let mut usage = self.usage.write();
        usage.commands_executed += 1;
        
        if success {
            usage.commands_successful += 1;
        } else {
            usage.commands_failed += 1;
        }
        
        // Track most used commands
        *usage.most_used_commands.entry(command.to_string()).or_insert(0) += 1;
        
        // Record processing time
        let mut performance = self.performance.write();
        performance.total_processing_time_ms += duration_ms;
        
        debug!("Recorded command: {} ({}ms, success: {})", command, duration_ms, success);
    }
    
    /// Record screen capture time
    pub fn record_screen_capture(&self, duration_ms: u64) {
        let mut performance = self.performance.write();
        performance.screen_capture_time_ms.push(duration_ms);
        
        // Keep only last 100 measurements
        if performance.screen_capture_time_ms.len() > 100 {
            performance.screen_capture_time_ms.remove(0);
        }
        
        let mut usage = self.usage.write();
        usage.screenshots_taken += 1;
    }
    
    /// Record AI processing time
    pub fn record_ai_processing(&self, model: &str, duration_ms: u64, success: bool) {
        let mut performance = self.performance.write();
        performance.ai_processing_time_ms.push(duration_ms);
        
        // Keep only last 100 measurements
        if performance.ai_processing_time_ms.len() > 100 {
            performance.ai_processing_time_ms.remove(0);
        }
        
        let mut usage = self.usage.write();
        usage.ai_inferences += 1;
        
        if !success {
            let mut errors = self.errors.write();
            errors.ai_errors += 1;
        }
        
        debug!("Recorded AI processing: {} ({}ms, success: {})", model, duration_ms, success);
    }
    
    /// Record action execution
    pub fn record_action(&self, action_type: &str, duration_ms: u64) {
        let mut performance = self.performance.write();
        performance.action_execution_time_ms.push(duration_ms);
        
        // Keep only last 100 measurements
        if performance.action_execution_time_ms.len() > 100 {
            performance.action_execution_time_ms.remove(0);
        }
        
        let mut usage = self.usage.write();
        
        // Count specific action types
        match action_type {
            "click" | "right_click" | "double_click" => {
                usage.clicks_performed += 1;
            }
            "key_press" | "key_combo" | "type" => {
                usage.keys_pressed += 1;
            }
            _ => {}
        }
    }
    
    /// Record error
    pub fn record_error(&self, error_type: &str, message: &str, operation: &str) {
        let mut errors = self.errors.write();
        errors.total_errors += 1;
        
        // Count by type
        *errors.error_types.entry(error_type.to_string()).or_insert(0) += 1;
        
        // Specific error categories
        match error_type {
            "ai_error" => errors.ai_errors += 1,
            "input_error" => errors.input_errors += 1,
            "capture_error" => errors.capture_errors += 1,
            "safety_block" => errors.safety_blocks += 1,
            _ => {}
        }
        
        // Add to recent errors
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        errors.recent_errors.push(ErrorEntry {
            timestamp,
            error_type: error_type.to_string(),
            message: message.to_string(),
            operation: operation.to_string(),
        });
        
        // Keep only last 50 errors
        if errors.recent_errors.len() > 50 {
            errors.recent_errors.remove(0);
        }
        
        warn!("Recorded error: {} in {}: {}", error_type, operation, message);
    }
    
    /// Update memory usage
    pub fn update_memory_usage(&self, memory_mb: u64) {
        let mut performance = self.performance.write();
        performance.memory_usage_mb = memory_mb;
        
        // Warn if memory usage is high
        if memory_mb > 400 {
            warn!("High memory usage: {}MB", memory_mb);
        }
    }
    
    /// Update CPU usage
    pub fn update_cpu_usage(&self, cpu_percent: f64) {
        let mut performance = self.performance.write();
        performance.cpu_usage_percent = cpu_percent;
        
        // Warn if CPU usage is high
        if cpu_percent > 80.0 {
            warn!("High CPU usage: {:.1}%", cpu_percent);
        }
    }
    
    /// Increment a custom counter
    pub fn increment_counter(&self, name: &str) {
        let mut counters = self.counters.write();
        *counters.entry(name.to_string()).or_insert(0) += 1;
    }
    
    /// Add a custom timer measurement
    pub fn record_timer(&self, name: &str, duration: Duration) {
        let mut timers = self.timers.write();
        timers.entry(name.to_string()).or_insert_with(Vec::new).push(duration);
    }
    
    /// Start a timer and return a guard that will record the time when dropped
    pub fn start_timer(&self, name: String) -> TimerGuard {
        TimerGuard {
            name,
            start_time: Instant::now(),
            collector: Arc::downgrade(&Arc::new(self.clone())),
        }
    }
    
    /// Get current performance metrics
    pub fn get_performance_metrics(&self) -> PerformanceMetrics {
        let performance = self.performance.read();
        performance.clone()
    }
    
    /// Get current usage metrics
    pub fn get_usage_metrics(&self) -> UsageMetrics {
        let mut usage = self.usage.write();
        usage.session_duration_seconds = self.start_time.elapsed().as_secs();
        usage.clone()
    }
    
    /// Get current error metrics
    pub fn get_error_metrics(&self) -> ErrorMetrics {
        let errors = self.errors.read();
        errors.clone()
    }
    
    /// Get all metrics as summary
    pub fn get_metrics_summary(&self) -> MetricsSummary {
        let performance = self.get_performance_metrics();
        let usage = self.get_usage_metrics();
        let errors = self.get_error_metrics();
        let counters = self.counters.read().clone();
        
        MetricsSummary {
            uptime_seconds: self.start_time.elapsed().as_secs(),
            performance,
            usage,
            errors,
            custom_counters: counters,
        }
    }
    
    /// Calculate average processing times
    pub fn get_average_times(&self) -> AverageTimings {
        let performance = self.performance.read();
        
        let avg_capture = if !performance.screen_capture_time_ms.is_empty() {
            performance.screen_capture_time_ms.iter().sum::<u64>() as f64 / performance.screen_capture_time_ms.len() as f64
        } else {
            0.0
        };
        
        let avg_ai = if !performance.ai_processing_time_ms.is_empty() {
            performance.ai_processing_time_ms.iter().sum::<u64>() as f64 / performance.ai_processing_time_ms.len() as f64
        } else {
            0.0
        };
        
        let avg_action = if !performance.action_execution_time_ms.is_empty() {
            performance.action_execution_time_ms.iter().sum::<u64>() as f64 / performance.action_execution_time_ms.len() as f64
        } else {
            0.0
        };
        
        AverageTimings {
            screen_capture_ms: avg_capture,
            ai_processing_ms: avg_ai,
            action_execution_ms: avg_action,
        }
    }
    
    /// Reset all metrics
    pub fn reset(&self) {
        debug!("Resetting all metrics");
        
        *self.performance.write() = PerformanceMetrics::default();
        *self.usage.write() = UsageMetrics::default();
        *self.errors.write() = ErrorMetrics::default();
        self.counters.write().clear();
        self.timers.write().clear();
    }
    
    /// Export metrics to JSON
    pub fn export_json(&self) -> Result<String, serde_json::Error> {
        let summary = self.get_metrics_summary();
        serde_json::to_string_pretty(&summary)
    }
}

// Implement Clone for MetricsCollector
impl Clone for MetricsCollector {
    fn clone(&self) -> Self {
        Self {
            performance: self.performance.clone(),
            usage: self.usage.clone(),
            errors: self.errors.clone(),
            counters: self.counters.clone(),
            timers: self.timers.clone(),
            start_time: self.start_time,
        }
    }
}

/// Timer guard that records elapsed time when dropped
pub struct TimerGuard {
    name: String,
    start_time: Instant,
    collector: std::sync::Weak<MetricsCollector>,
}

impl Drop for TimerGuard {
    fn drop(&mut self) {
        let duration = self.start_time.elapsed();
        if let Some(collector) = self.collector.upgrade() {
            collector.record_timer(&self.name, duration);
        }
    }
}

/// Complete metrics summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSummary {
    pub uptime_seconds: u64,
    pub performance: PerformanceMetrics,
    pub usage: UsageMetrics,
    pub errors: ErrorMetrics,
    pub custom_counters: HashMap<String, u64>,
}

/// Average timing statistics
#[derive(Debug, Clone)]
pub struct AverageTimings {
    pub screen_capture_ms: f64,
    pub ai_processing_ms: f64,
    pub action_execution_ms: f64,
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Global metrics instance (for convenience)
static GLOBAL_METRICS: once_cell::sync::OnceCell<MetricsCollector> = once_cell::sync::OnceCell::new();

/// Get global metrics instance
pub fn global_metrics() -> &'static MetricsCollector {
    GLOBAL_METRICS.get_or_init(MetricsCollector::new)
}

/// Initialize global metrics
pub fn init_global_metrics() -> &'static MetricsCollector {
    global_metrics()
}