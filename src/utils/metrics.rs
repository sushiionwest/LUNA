/*!
 * Luna Visual AI Performance Metrics System
 * 
 * Comprehensive performance monitoring and metrics collection:
 * - Real-time performance tracking for all components
 * - Memory usage monitoring and alerting
 * - AI model inference time measurement
 * - System resource utilization tracking
 * - Performance bottleneck identification
 * - Historical metrics storage and analysis
 * - Automated performance reporting
 */

use crate::core::error::{LunaError, Result};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, VecDeque},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tracing::{debug, info, warn};

/// Global metrics collector instance
static METRICS_COLLECTOR: once_cell::sync::Lazy<Arc<MetricsCollector>> =
    once_cell::sync::Lazy::new(|| Arc::new(MetricsCollector::new()));

/// Performance metrics collection and analysis system
pub struct MetricsCollector {
    /// System performance metrics
    system_metrics: RwLock<SystemMetrics>,
    
    /// Component performance metrics
    component_metrics: RwLock<HashMap<String, ComponentMetrics>>,
    
    /// AI model performance metrics
    ai_metrics: RwLock<HashMap<String, AiModelMetrics>>,
    
    /// Historical data points (last 1 hour)
    historical_data: RwLock<VecDeque<MetricsSnapshot>>,
    
    /// Performance alerts
    alerts: RwLock<Vec<PerformanceAlert>>,
    
    /// Metrics collection state
    collection_enabled: AtomicU64, // Using as boolean
    
    /// Last collection timestamp
    last_collection: AtomicU64,
}

/// System-wide performance metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub cpu_usage_percent: f64,
    pub memory_usage_bytes: u64,
    pub memory_available_bytes: u64,
    pub disk_usage_bytes: u64,
    pub network_bytes_sent: u64,
    pub network_bytes_received: u64,
    pub uptime_seconds: u64,
    pub last_updated: u64,
}

/// Component-specific performance metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ComponentMetrics {
    pub component_name: String,
    pub operations_count: u64,
    pub total_execution_time_ms: u64,
    pub average_execution_time_ms: f64,
    pub min_execution_time_ms: u64,
    pub max_execution_time_ms: u64,
    pub error_count: u64,
    pub error_rate_percent: f64,
    pub memory_usage_bytes: u64,
    pub last_operation: Option<u64>,
}

/// AI model performance metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AiModelMetrics {
    pub model_name: String,
    pub inference_count: u64,
    pub total_inference_time_ms: u64,
    pub average_inference_time_ms: f64,
    pub min_inference_time_ms: u64,
    pub max_inference_time_ms: u64,
    pub inference_errors: u64,
    pub confidence_average: f64,
    pub memory_usage_bytes: u64,
    pub gpu_utilization_percent: f64,
    pub last_inference: Option<u64>,
}

/// Point-in-time metrics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub timestamp: u64,
    pub system: SystemMetrics,
    pub components: HashMap<String, ComponentMetrics>,
    pub ai_models: HashMap<String, AiModelMetrics>,
}

/// Performance alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceAlert {
    pub id: String,
    pub alert_type: AlertType,
    pub severity: AlertSeverity,
    pub message: String,
    pub component: Option<String>,
    pub metric_value: f64,
    pub threshold: f64,
    pub timestamp: u64,
    pub acknowledged: bool,
}

/// Types of performance alerts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertType {
    HighCpuUsage,
    HighMemoryUsage,
    SlowInference,
    HighErrorRate,
    SystemOverload,
    ModelTimeout,
    MemoryLeak,
    DiskSpaceLow,
}

/// Alert severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// Performance measurement timer
pub struct PerformanceTimer {
    component: String,
    operation: String,
    start_time: Instant,
    memory_start: u64,
}

impl MetricsCollector {
    fn new() -> Self {
        Self {
            system_metrics: RwLock::new(SystemMetrics::default()),
            component_metrics: RwLock::new(HashMap::new()),
            ai_metrics: RwLock::new(HashMap::new()),
            historical_data: RwLock::new(VecDeque::new()),
            alerts: RwLock::new(Vec::new()),
            collection_enabled: AtomicU64::new(1),
            last_collection: AtomicU64::new(0),
        }
    }

    /// Start metrics collection
    pub fn start_collection(&self) -> Result<()> {
        self.collection_enabled.store(1, Ordering::Relaxed);
        info!("📊 Performance metrics collection started");
        Ok(())
    }

    /// Stop metrics collection
    pub fn stop_collection(&self) {
        self.collection_enabled.store(0, Ordering::Relaxed);
        info!("Performance metrics collection stopped");
    }

    /// Record system metrics
    pub fn record_system_metrics(&self) -> Result<()> {
        if self.collection_enabled.load(Ordering::Relaxed) == 0 {
            return Ok(());
        }

        let now = SystemTime::now().duration_since(UNIX_EPOCH)
            .unwrap().as_secs();

        let mut system_metrics = self.system_metrics.write();
        
        // Update system metrics (placeholder implementations)
        system_metrics.cpu_usage_percent = self.get_cpu_usage()?;
        system_metrics.memory_usage_bytes = self.get_memory_usage()?;
        system_metrics.memory_available_bytes = self.get_available_memory()?;
        system_metrics.uptime_seconds = self.get_uptime()?;
        system_metrics.last_updated = now;

        self.last_collection.store(now, Ordering::Relaxed);

        // Check for alerts
        self.check_system_alerts(&system_metrics)?;

        debug!("System metrics updated: CPU {:.1}%, Memory {} MB",
               system_metrics.cpu_usage_percent,
               system_metrics.memory_usage_bytes / 1_000_000);

        Ok(())
    }

    /// Record component operation
    pub fn record_component_operation(
        &self,
        component: &str,
        execution_time_ms: u64,
        success: bool,
        memory_used: u64,
    ) -> Result<()> {
        if self.collection_enabled.load(Ordering::Relaxed) == 0 {
            return Ok(());
        }

        let now = SystemTime::now().duration_since(UNIX_EPOCH)
            .unwrap().as_secs();

        let mut components = self.component_metrics.write();
        let metrics = components.entry(component.to_string())
            .or_insert_with(|| ComponentMetrics {
                component_name: component.to_string(),
                min_execution_time_ms: u64::MAX,
                ..Default::default()
            });

        // Update metrics
        metrics.operations_count += 1;
        metrics.total_execution_time_ms += execution_time_ms;
        metrics.average_execution_time_ms = 
            metrics.total_execution_time_ms as f64 / metrics.operations_count as f64;
        
        metrics.min_execution_time_ms = metrics.min_execution_time_ms.min(execution_time_ms);
        metrics.max_execution_time_ms = metrics.max_execution_time_ms.max(execution_time_ms);
        
        if !success {
            metrics.error_count += 1;
        }
        metrics.error_rate_percent = 
            (metrics.error_count as f64 / metrics.operations_count as f64) * 100.0;
        
        metrics.memory_usage_bytes = memory_used;
        metrics.last_operation = Some(now);

        // Check for component-specific alerts
        self.check_component_alerts(component, metrics)?;

        debug!("Component operation recorded: {} ({}ms, success: {})",
               component, execution_time_ms, success);

        Ok(())
    }

    /// Record AI model inference
    pub fn record_ai_inference(
        &self,
        model_name: &str,
        inference_time_ms: u64,
        confidence: f32,
        success: bool,
        memory_used: u64,
        gpu_utilization: f64,
    ) -> Result<()> {
        if self.collection_enabled.load(Ordering::Relaxed) == 0 {
            return Ok(());
        }

        let now = SystemTime::now().duration_since(UNIX_EPOCH)
            .unwrap().as_secs();

        let mut ai_metrics = self.ai_metrics.write();
        let metrics = ai_metrics.entry(model_name.to_string())
            .or_insert_with(|| AiModelMetrics {
                model_name: model_name.to_string(),
                min_inference_time_ms: u64::MAX,
                ..Default::default()
            });

        // Update AI metrics
        metrics.inference_count += 1;
        metrics.total_inference_time_ms += inference_time_ms;
        metrics.average_inference_time_ms = 
            metrics.total_inference_time_ms as f64 / metrics.inference_count as f64;
        
        metrics.min_inference_time_ms = metrics.min_inference_time_ms.min(inference_time_ms);
        metrics.max_inference_time_ms = metrics.max_inference_time_ms.max(inference_time_ms);
        
        if !success {
            metrics.inference_errors += 1;
        }
        
        // Update confidence average (exponential moving average)
        let alpha = 0.1;
        metrics.confidence_average = 
            alpha * confidence as f64 + (1.0 - alpha) * metrics.confidence_average;
        
        metrics.memory_usage_bytes = memory_used;
        metrics.gpu_utilization_percent = gpu_utilization;
        metrics.last_inference = Some(now);

        // Check for AI model alerts
        self.check_ai_model_alerts(model_name, metrics)?;

        debug!("AI inference recorded: {} ({}ms, confidence: {:.2}, success: {})",
               model_name, inference_time_ms, confidence, success);

        Ok(())
    }

    /// Take a snapshot of current metrics
    pub fn take_snapshot(&self) -> MetricsSnapshot {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)
            .unwrap().as_secs();

        MetricsSnapshot {
            timestamp: now,
            system: self.system_metrics.read().clone(),
            components: self.component_metrics.read().clone(),
            ai_models: self.ai_metrics.read().clone(),
        }
    }

    /// Store snapshot in historical data
    pub fn store_snapshot(&self, snapshot: MetricsSnapshot) {
        let mut historical = self.historical_data.write();
        
        historical.push_back(snapshot);
        
        // Keep only last hour of data (assuming 1 snapshot per minute)
        while historical.len() > 60 {
            historical.pop_front();
        }
    }

    /// Get historical data
    pub fn get_historical_data(&self) -> Vec<MetricsSnapshot> {
        self.historical_data.read().iter().cloned().collect()
    }

    /// Get current alerts
    pub fn get_alerts(&self) -> Vec<PerformanceAlert> {
        self.alerts.read().clone()
    }

    /// Acknowledge an alert
    pub fn acknowledge_alert(&self, alert_id: &str) -> Result<()> {
        let mut alerts = self.alerts.write();
        
        if let Some(alert) = alerts.iter_mut().find(|a| a.id == alert_id) {
            alert.acknowledged = true;
            info!("Alert acknowledged: {}", alert_id);
        }
        
        Ok(())
    }

    /// Clear old alerts
    pub fn clear_old_alerts(&self) {
        let cutoff = SystemTime::now().duration_since(UNIX_EPOCH)
            .unwrap().as_secs() - 3600; // 1 hour ago
        
        let mut alerts = self.alerts.write();
        alerts.retain(|alert| alert.timestamp > cutoff);
    }

    /// Check for system-level alerts
    fn check_system_alerts(&self, metrics: &SystemMetrics) -> Result<()> {
        // High CPU usage alert
        if metrics.cpu_usage_percent > 90.0 {
            self.create_alert(
                AlertType::HighCpuUsage,
                AlertSeverity::Warning,
                format!("High CPU usage: {:.1}%", metrics.cpu_usage_percent),
                None,
                metrics.cpu_usage_percent,
                90.0,
            )?;
        }

        // High memory usage alert
        let memory_usage_percent = if metrics.memory_available_bytes > 0 {
            (metrics.memory_usage_bytes as f64 / 
             (metrics.memory_usage_bytes + metrics.memory_available_bytes) as f64) * 100.0
        } else {
            0.0
        };

        if memory_usage_percent > 85.0 {
            self.create_alert(
                AlertType::HighMemoryUsage,
                AlertSeverity::Warning,
                format!("High memory usage: {:.1}%", memory_usage_percent),
                None,
                memory_usage_percent,
                85.0,
            )?;
        }

        Ok(())
    }

    /// Check for component-specific alerts
    fn check_component_alerts(&self, component: &str, metrics: &ComponentMetrics) -> Result<()> {
        // High error rate alert
        if metrics.error_rate_percent > 10.0 && metrics.operations_count > 10 {
            self.create_alert(
                AlertType::HighErrorRate,
                AlertSeverity::Error,
                format!("High error rate in {}: {:.1}%", component, metrics.error_rate_percent),
                Some(component.to_string()),
                metrics.error_rate_percent,
                10.0,
            )?;
        }

        // Slow operations alert
        if metrics.average_execution_time_ms > 5000.0 { // 5 seconds
            self.create_alert(
                AlertType::SystemOverload,
                AlertSeverity::Warning,
                format!("Slow operations in {}: {:.1}ms average", 
                       component, metrics.average_execution_time_ms),
                Some(component.to_string()),
                metrics.average_execution_time_ms,
                5000.0,
            )?;
        }

        Ok(())
    }

    /// Check for AI model alerts
    fn check_ai_model_alerts(&self, model_name: &str, metrics: &AiModelMetrics) -> Result<()> {
        // Slow inference alert
        if metrics.average_inference_time_ms > 2000.0 { // 2 seconds
            self.create_alert(
                AlertType::SlowInference,
                AlertSeverity::Warning,
                format!("Slow inference in {}: {:.1}ms average", 
                       model_name, metrics.average_inference_time_ms),
                Some(model_name.to_string()),
                metrics.average_inference_time_ms,
                2000.0,
            )?;
        }

        // Low confidence alert
        if metrics.confidence_average < 0.7 && metrics.inference_count > 10 {
            self.create_alert(
                AlertType::ModelTimeout,
                AlertSeverity::Warning,
                format!("Low confidence in {}: {:.2}", model_name, metrics.confidence_average),
                Some(model_name.to_string()),
                metrics.confidence_average,
                0.7,
            )?;
        }

        Ok(())
    }

    /// Create a new performance alert
    fn create_alert(
        &self,
        alert_type: AlertType,
        severity: AlertSeverity,
        message: String,
        component: Option<String>,
        metric_value: f64,
        threshold: f64,
    ) -> Result<()> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)
            .unwrap().as_secs();

        let alert = PerformanceAlert {
            id: format!("alert_{}", now),
            alert_type,
            severity,
            message: message.clone(),
            component,
            metric_value,
            threshold,
            timestamp: now,
            acknowledged: false,
        };

        self.alerts.write().push(alert);

        match severity {
            AlertSeverity::Critical => warn!("🚨 CRITICAL ALERT: {}", message),
            AlertSeverity::Error => warn!("❌ ERROR ALERT: {}", message),
            AlertSeverity::Warning => warn!("⚠️  WARNING ALERT: {}", message),
            AlertSeverity::Info => info!("ℹ️  INFO ALERT: {}", message),
        }

        Ok(())
    }

    /// Get CPU usage percentage (placeholder implementation)
    fn get_cpu_usage(&self) -> Result<f64> {
        // In a real implementation, this would read from Windows performance counters
        // or use a system monitoring library
        Ok(25.0) // Placeholder value
    }

    /// Get memory usage in bytes
    fn get_memory_usage(&self) -> Result<u64> {
        crate::core::memory::get_current_usage()
    }

    /// Get available memory in bytes
    fn get_available_memory(&self) -> Result<u64> {
        crate::core::memory::get_available_memory()
    }

    /// Get system uptime in seconds
    fn get_uptime(&self) -> Result<u64> {
        // Placeholder implementation
        Ok(3600) // 1 hour
    }
}

impl PerformanceTimer {
    /// Start a new performance timer
    pub fn start(component: &str, operation: &str) -> Self {
        Self {
            component: component.to_string(),
            operation: operation.to_string(),
            start_time: Instant::now(),
            memory_start: crate::core::memory::get_current_usage(),
        }
    }

    /// Finish the timer and record metrics
    pub fn finish(self, success: bool) -> u64 {
        let elapsed_ms = self.start_time.elapsed().as_millis() as u64;
        let memory_end = crate::core::memory::get_current_usage();
        let memory_used = memory_end.saturating_sub(self.memory_start);

        if let Err(e) = METRICS_COLLECTOR.record_component_operation(
            &self.component,
            elapsed_ms,
            success,
            memory_used,
        ) {
            warn!("Failed to record performance metrics: {}", e);
        }

        elapsed_ms
    }
}

// Public API functions

/// Start metrics collection
pub fn start_collection() -> Result<()> {
    METRICS_COLLECTOR.start_collection()
}

/// Stop metrics collection
pub fn stop_collection() {
    METRICS_COLLECTOR.stop_collection()
}

/// Record system metrics
pub fn record_system_metrics() -> Result<()> {
    METRICS_COLLECTOR.record_system_metrics()
}

/// Record component operation
pub fn record_component_operation(
    component: &str,
    execution_time_ms: u64,
    success: bool,
    memory_used: u64,
) -> Result<()> {
    METRICS_COLLECTOR.record_component_operation(component, execution_time_ms, success, memory_used)
}

/// Record AI inference
pub fn record_ai_inference(
    model_name: &str,
    inference_time_ms: u64,
    confidence: f32,
    success: bool,
    memory_used: u64,
    gpu_utilization: f64,
) -> Result<()> {
    METRICS_COLLECTOR.record_ai_inference(
        model_name,
        inference_time_ms,
        confidence,
        success,
        memory_used,
        gpu_utilization,
    )
}

/// Take a snapshot of current metrics
pub fn take_snapshot() -> MetricsSnapshot {
    METRICS_COLLECTOR.take_snapshot()
}

/// Store snapshot in historical data
pub fn store_snapshot(snapshot: MetricsSnapshot) {
    METRICS_COLLECTOR.store_snapshot(snapshot)
}

/// Get historical data
pub fn get_historical_data() -> Vec<MetricsSnapshot> {
    METRICS_COLLECTOR.get_historical_data()
}

/// Get current alerts
pub fn get_alerts() -> Vec<PerformanceAlert> {
    METRICS_COLLECTOR.get_alerts()
}

/// Acknowledge an alert
pub fn acknowledge_alert(alert_id: &str) -> Result<()> {
    METRICS_COLLECTOR.acknowledge_alert(alert_id)
}

/// Clear old alerts
pub fn clear_old_alerts() {
    METRICS_COLLECTOR.clear_old_alerts()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_performance_timer() {
        let timer = PerformanceTimer::start("test", "operation");
        std::thread::sleep(Duration::from_millis(10));
        let elapsed = timer.finish(true);
        assert!(elapsed >= 10);
    }

    #[test]
    fn test_metrics_snapshot() {
        start_collection().unwrap();
        
        record_component_operation("test", 100, true, 1000).unwrap();
        
        let snapshot = take_snapshot();
        assert!(snapshot.components.contains_key("test"));
    }

    #[test]
    fn test_alert_creation() {
        start_collection().unwrap();
        
        // Simulate high error rate
        for _ in 0..15 {
            record_component_operation("failing_component", 100, false, 1000).unwrap();
        }
        
        let alerts = get_alerts();
        assert!(!alerts.is_empty());
    }
}