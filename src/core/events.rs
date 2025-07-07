/*!
 * Luna Visual AI Event System
 * 
 * High-performance event-driven communication system for Luna components:
 * - Type-safe event publishing and subscription
 * - Async event handling with backpressure control
 * - Event filtering and routing
 * - Memory-efficient event queue management
 * - Component lifecycle event coordination
 * - Real-time system status broadcasting
 */

use crate::core::error::{LunaError, Result};
use crossbeam::channel::{bounded, Receiver, Sender};
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::time;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Global event system instance
static EVENT_SYSTEM: once_cell::sync::Lazy<Arc<EventSystem>> =
    once_cell::sync::Lazy::new(|| Arc::new(EventSystem::new()));

/// Event system for inter-component communication
pub struct EventSystem {
    /// Event subscribers by event type
    subscribers: RwLock<HashMap<String, Vec<EventSubscriber>>>,
    
    /// Event queue for async processing
    event_queue: Mutex<Sender<LunaEvent>>,
    
    /// Event statistics
    stats: RwLock<EventStats>,
    
    /// System running state
    running: AtomicU64, // Using as boolean with timestamp
    
    /// Event processor handle
    processor_handle: Mutex<Option<tokio::task::JoinHandle<()>>>,
}

/// Event subscriber information
#[derive(Clone)]
struct EventSubscriber {
    id: Uuid,
    sender: Sender<LunaEvent>,
    filter: Option<EventFilter>,
    priority: EventPriority,
}

/// Event filtering criteria
#[derive(Debug, Clone)]
pub struct EventFilter {
    pub source_components: Option<Vec<String>>,
    pub event_types: Option<Vec<LunaEventType>>,
    pub min_priority: Option<EventPriority>,
}

/// Event priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum EventPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

/// Core event types in Luna Visual AI
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LunaEventType {
    // System events
    SystemStartup,
    SystemShutdown,
    SystemError,
    MemoryPressure,
    
    // AI events
    AiModelLoaded,
    AiModelUnloaded,
    AiInferenceStarted,
    AiInferenceCompleted,
    AiInferenceFailed,
    
    // Vision events
    ScreenCaptured,
    ElementDetected,
    ElementAnalyzed,
    
    // Input events
    VoiceCommandReceived,
    TextCommandReceived,
    HotkeyPressed,
    CommandParsed,
    CommandValidated,
    
    // Action events
    ActionPlanned,
    ActionPreview,
    ActionConfirmed,
    ActionCancelled,
    ActionExecuted,
    ActionFailed,
    
    // Overlay events
    OverlayShown,
    OverlayHidden,
    CountdownStarted,
    CountdownCancelled,
    UserInteraction,
    
    // Safety events
    SafetyCheckStarted,
    SafetyCheckPassed,
    SafetyCheckFailed,
    UnsafeActionBlocked,
    
    // Custom events for plugins
    Custom(String),
}

/// Luna event structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LunaEvent {
    /// Unique event identifier
    pub id: Uuid,
    
    /// Event type
    pub event_type: LunaEventType,
    
    /// Source component that generated the event
    pub source: String,
    
    /// Event priority
    pub priority: EventPriority,
    
    /// Event timestamp
    pub timestamp: u64,
    
    /// Event payload data
    pub data: serde_json::Value,
    
    /// Optional correlation ID for related events
    pub correlation_id: Option<Uuid>,
}

/// Event handling statistics
#[derive(Debug, Clone, Default)]
pub struct EventStats {
    pub events_published: u64,
    pub events_processed: u64,
    pub events_failed: u64,
    pub events_dropped: u64,
    pub subscribers_count: usize,
    pub queue_size: usize,
    pub average_processing_time_ms: f64,
}

/// Event subscription handle
pub struct EventSubscription {
    id: Uuid,
    receiver: Receiver<LunaEvent>,
}

impl EventSystem {
    fn new() -> Self {
        let (sender, _) = bounded(10000); // 10k event buffer
        
        Self {
            subscribers: RwLock::new(HashMap::new()),
            event_queue: Mutex::new(sender),
            stats: RwLock::new(EventStats::default()),
            running: AtomicU64::new(0),
            processor_handle: Mutex::new(None),
        }
    }

    /// Start the event processing system
    fn start(&self) -> Result<()> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)
            .unwrap().as_secs();
        self.running.store(now, Ordering::Relaxed);

        // Create event processing queue
        let (sender, receiver) = bounded(10000);
        *self.event_queue.lock() = sender;

        // Start event processor
        let system = Arc::clone(&EVENT_SYSTEM);
        let handle = tokio::spawn(async move {
            system.event_processor_loop(receiver).await;
        });

        *self.processor_handle.lock() = Some(handle);

        info!("Event system started");
        Ok(())
    }

    /// Main event processing loop
    async fn event_processor_loop(&self, receiver: Receiver<LunaEvent>) {
        info!("Event processor started");
        
        while self.running.load(Ordering::Relaxed) != 0 {
            match receiver.recv_timeout(Duration::from_millis(100)) {
                Ok(event) => {
                    if let Err(e) = self.process_event(event).await {
                        error!("Error processing event: {}", e);
                        self.stats.write().events_failed += 1;
                    }
                }
                Err(crossbeam::channel::RecvTimeoutError::Timeout) => {
                    // Normal timeout, continue
                    continue;
                }
                Err(crossbeam::channel::RecvTimeoutError::Disconnected) => {
                    info!("Event queue disconnected, stopping processor");
                    break;
                }
            }
        }
        
        info!("Event processor stopped");
    }

    /// Process a single event
    async fn process_event(&self, event: LunaEvent) -> Result<()> {
        let start_time = std::time::Instant::now();
        
        debug!("Processing event: {:?} from {}", event.event_type, event.source);
        
        // Get subscribers for this event type
        let subscribers = {
            let subs = self.subscribers.read();
            subs.get(&event.event_type.to_string())
                .map(|s| s.clone())
                .unwrap_or_default()
        };

        // Send event to all matching subscribers
        let mut delivered = 0;
        let mut failed = 0;

        for subscriber in subscribers {
            // Check filter
            if let Some(ref filter) = subscriber.filter {
                if !self.event_matches_filter(&event, filter) {
                    continue;
                }
            }

            // Send event to subscriber
            match subscriber.sender.try_send(event.clone()) {
                Ok(_) => delivered += 1,
                Err(_) => {
                    failed += 1;
                    warn!("Failed to deliver event to subscriber {}", subscriber.id);
                }
            }
        }

        // Update statistics
        let processing_time = start_time.elapsed().as_millis() as f64;
        {
            let mut stats = self.stats.write();
            stats.events_processed += 1;
            if failed > 0 {
                stats.events_failed += failed;
            }
            
            // Update average processing time (simple moving average)
            stats.average_processing_time_ms = 
                (stats.average_processing_time_ms * 0.9) + (processing_time * 0.1);
        }

        if delivered > 0 {
            debug!("Event delivered to {} subscribers", delivered);
        }

        Ok(())
    }

    /// Check if event matches filter criteria
    fn event_matches_filter(&self, event: &LunaEvent, filter: &EventFilter) -> bool {
        // Check source component filter
        if let Some(ref allowed_sources) = filter.source_components {
            if !allowed_sources.contains(&event.source) {
                return false;
            }
        }

        // Check event type filter
        if let Some(ref allowed_types) = filter.event_types {
            if !allowed_types.contains(&event.event_type) {
                return false;
            }
        }

        // Check priority filter
        if let Some(min_priority) = filter.min_priority {
            if event.priority < min_priority {
                return false;
            }
        }

        true
    }

    /// Subscribe to events
    fn subscribe(&self, filter: Option<EventFilter>) -> Result<EventSubscription> {
        let id = Uuid::new_v4();
        let (sender, receiver) = bounded(1000); // Per-subscriber buffer

        let subscriber = EventSubscriber {
            id,
            sender,
            filter,
            priority: EventPriority::Normal,
        };

        // Add subscriber to all relevant event types
        let event_types = if let Some(ref filter) = subscriber.filter {
            filter.event_types.clone().unwrap_or_else(|| {
                // Subscribe to all event types if none specified
                vec![
                    LunaEventType::SystemStartup,
                    LunaEventType::SystemShutdown,
                    LunaEventType::SystemError,
                    LunaEventType::AiInferenceCompleted,
                    LunaEventType::ActionExecuted,
                    // Add more as needed
                ]
            })
        } else {
            // Subscribe to all event types
            vec![
                LunaEventType::SystemStartup,
                LunaEventType::SystemShutdown,
                LunaEventType::SystemError,
                LunaEventType::MemoryPressure,
                LunaEventType::AiModelLoaded,
                LunaEventType::AiInferenceCompleted,
                LunaEventType::ScreenCaptured,
                LunaEventType::VoiceCommandReceived,
                LunaEventType::ActionExecuted,
                LunaEventType::SafetyCheckFailed,
            ]
        };

        {
            let mut subscribers = self.subscribers.write();
            for event_type in event_types {
                subscribers
                    .entry(event_type.to_string())
                    .or_insert_with(Vec::new)
                    .push(subscriber.clone());
            }
            
            // Update stats
            self.stats.write().subscribers_count = subscribers.len();
        }

        info!("New event subscription created: {}", id);

        Ok(EventSubscription { id, receiver })
    }

    /// Unsubscribe from events
    fn unsubscribe(&self, subscription_id: Uuid) -> Result<()> {
        let mut subscribers = self.subscribers.write();
        
        for (_, subs) in subscribers.iter_mut() {
            subs.retain(|s| s.id != subscription_id);
        }
        
        // Remove empty event type entries
        subscribers.retain(|_, subs| !subs.is_empty());
        
        // Update stats
        self.stats.write().subscribers_count = subscribers.len();
        
        info!("Event subscription removed: {}", subscription_id);
        Ok(())
    }

    /// Publish an event
    fn publish(&self, event: LunaEvent) -> Result<()> {
        let queue = self.event_queue.lock();
        
        match queue.try_send(event.clone()) {
            Ok(_) => {
                self.stats.write().events_published += 1;
                debug!("Event published: {:?}", event.event_type);
                Ok(())
            }
            Err(_) => {
                self.stats.write().events_dropped += 1;
                Err(LunaError::internal(
                    "Event queue is full, event dropped",
                    "events",
                ))
            }
        }
    }

    /// Get system statistics
    fn get_stats(&self) -> EventStats {
        self.stats.read().clone()
    }

    /// Stop the event system
    async fn stop(&self) -> Result<()> {
        info!("Stopping event system");
        
        self.running.store(0, Ordering::Relaxed);
        
        // Wait for processor to stop
        if let Some(handle) = self.processor_handle.lock().take() {
            if let Err(e) = handle.await {
                error!("Error waiting for event processor to stop: {}", e);
            }
        }
        
        // Clear all subscribers
        self.subscribers.write().clear();
        
        info!("Event system stopped");
        Ok(())
    }
}

impl LunaEvent {
    /// Create a new event
    pub fn new(
        event_type: LunaEventType,
        source: String,
        priority: EventPriority,
        data: serde_json::Value,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            event_type,
            source,
            priority,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            data,
            correlation_id: None,
        }
    }

    /// Create a new event with correlation ID
    pub fn with_correlation(
        event_type: LunaEventType,
        source: String,
        priority: EventPriority,
        data: serde_json::Value,
        correlation_id: Uuid,
    ) -> Self {
        let mut event = Self::new(event_type, source, priority, data);
        event.correlation_id = Some(correlation_id);
        event
    }

    /// Create a system event
    pub fn system(event_type: LunaEventType, data: serde_json::Value) -> Self {
        Self::new(event_type, "system".to_string(), EventPriority::High, data)
    }

    /// Create an error event
    pub fn error(source: String, error_message: String) -> Self {
        Self::new(
            LunaEventType::SystemError,
            source,
            EventPriority::Critical,
            serde_json::json!({
                "error": error_message,
                "timestamp": SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            }),
        )
    }
}

impl EventSubscription {
    /// Receive the next event (blocking)
    pub fn recv(&self) -> Result<LunaEvent> {
        self.receiver.recv()
            .map_err(|_| LunaError::internal("Event channel disconnected", "events"))
    }

    /// Receive the next event with timeout
    pub fn recv_timeout(&self, timeout: Duration) -> Result<LunaEvent> {
        self.receiver.recv_timeout(timeout)
            .map_err(|_| LunaError::internal("Event receive timeout or disconnected", "events"))
    }

    /// Try to receive an event without blocking
    pub fn try_recv(&self) -> Result<LunaEvent> {
        self.receiver.try_recv()
            .map_err(|_| LunaError::internal("No event available or channel disconnected", "events"))
    }

    /// Get subscription ID
    pub fn id(&self) -> Uuid {
        self.id
    }
}

impl ToString for LunaEventType {
    fn to_string(&self) -> String {
        match self {
            LunaEventType::SystemStartup => "system_startup".to_string(),
            LunaEventType::SystemShutdown => "system_shutdown".to_string(),
            LunaEventType::SystemError => "system_error".to_string(),
            LunaEventType::MemoryPressure => "memory_pressure".to_string(),
            LunaEventType::AiModelLoaded => "ai_model_loaded".to_string(),
            LunaEventType::AiModelUnloaded => "ai_model_unloaded".to_string(),
            LunaEventType::AiInferenceStarted => "ai_inference_started".to_string(),
            LunaEventType::AiInferenceCompleted => "ai_inference_completed".to_string(),
            LunaEventType::AiInferenceFailed => "ai_inference_failed".to_string(),
            LunaEventType::ScreenCaptured => "screen_captured".to_string(),
            LunaEventType::ElementDetected => "element_detected".to_string(),
            LunaEventType::ElementAnalyzed => "element_analyzed".to_string(),
            LunaEventType::VoiceCommandReceived => "voice_command_received".to_string(),
            LunaEventType::TextCommandReceived => "text_command_received".to_string(),
            LunaEventType::HotkeyPressed => "hotkey_pressed".to_string(),
            LunaEventType::CommandParsed => "command_parsed".to_string(),
            LunaEventType::CommandValidated => "command_validated".to_string(),
            LunaEventType::ActionPlanned => "action_planned".to_string(),
            LunaEventType::ActionPreview => "action_preview".to_string(),
            LunaEventType::ActionConfirmed => "action_confirmed".to_string(),
            LunaEventType::ActionCancelled => "action_cancelled".to_string(),
            LunaEventType::ActionExecuted => "action_executed".to_string(),
            LunaEventType::ActionFailed => "action_failed".to_string(),
            LunaEventType::OverlayShown => "overlay_shown".to_string(),
            LunaEventType::OverlayHidden => "overlay_hidden".to_string(),
            LunaEventType::CountdownStarted => "countdown_started".to_string(),
            LunaEventType::CountdownCancelled => "countdown_cancelled".to_string(),
            LunaEventType::UserInteraction => "user_interaction".to_string(),
            LunaEventType::SafetyCheckStarted => "safety_check_started".to_string(),
            LunaEventType::SafetyCheckPassed => "safety_check_passed".to_string(),
            LunaEventType::SafetyCheckFailed => "safety_check_failed".to_string(),
            LunaEventType::UnsafeActionBlocked => "unsafe_action_blocked".to_string(),
            LunaEventType::Custom(name) => format!("custom_{}", name),
        }
    }
}

// Public API functions

/// Initialize the event system
pub fn init() -> Result<()> {
    EVENT_SYSTEM.start()
}

/// Shutdown the event system
pub async fn shutdown() -> Result<()> {
    EVENT_SYSTEM.stop().await
}

/// Validate the event system is working
pub async fn validate_system() -> Result<()> {
    if EVENT_SYSTEM.running.load(Ordering::Relaxed) == 0 {
        return Err(LunaError::internal("Event system not running", "events"));
    }
    Ok(())
}

/// Subscribe to events with optional filtering
pub fn subscribe(filter: Option<EventFilter>) -> Result<EventSubscription> {
    EVENT_SYSTEM.subscribe(filter)
}

/// Unsubscribe from events
pub fn unsubscribe(subscription_id: Uuid) -> Result<()> {
    EVENT_SYSTEM.unsubscribe(subscription_id)
}

/// Publish an event
pub fn publish(event: LunaEvent) -> Result<()> {
    EVENT_SYSTEM.publish(event)
}

/// Get event system statistics
pub fn get_stats() -> EventStats {
    EVENT_SYSTEM.get_stats()
}

/// Helper function to publish a quick event
pub fn publish_event(
    event_type: LunaEventType,
    source: &str,
    priority: EventPriority,
    data: serde_json::Value,
) -> Result<()> {
    let event = LunaEvent::new(event_type, source.to_string(), priority, data);
    publish(event)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::timeout;

    #[tokio::test]
    async fn test_event_system_init() {
        let result = init();
        assert!(result.is_ok());
        
        let validation = validate_system().await;
        assert!(validation.is_ok());
        
        let shutdown_result = shutdown().await;
        assert!(shutdown_result.is_ok());
    }

    #[tokio::test]
    async fn test_event_subscription() {
        init().unwrap();
        
        let subscription = subscribe(None).unwrap();
        
        // Publish a test event
        let event = LunaEvent::system(
            LunaEventType::SystemStartup,
            serde_json::json!({"test": true}),
        );
        publish(event).unwrap();
        
        // Try to receive the event
        let received = timeout(Duration::from_secs(1), async {
            subscription.recv()
        }).await;
        
        assert!(received.is_ok());
        
        unsubscribe(subscription.id()).unwrap();
        shutdown().await.unwrap();
    }

    #[test]
    fn test_event_creation() {
        let event = LunaEvent::new(
            LunaEventType::SystemStartup,
            "test".to_string(),
            EventPriority::Normal,
            serde_json::json!({"data": "test"}),
        );
        
        assert_eq!(event.source, "test");
        assert_eq!(event.priority, EventPriority::Normal);
        assert!(event.id != Uuid::nil());
    }

    #[test]
    fn test_event_filter() {
        let filter = EventFilter {
            source_components: Some(vec!["ai".to_string()]),
            event_types: Some(vec![LunaEventType::AiInferenceCompleted]),
            min_priority: Some(EventPriority::High),
        };
        
        let event1 = LunaEvent::new(
            LunaEventType::AiInferenceCompleted,
            "ai".to_string(),
            EventPriority::High,
            serde_json::json!({}),
        );
        
        let event2 = LunaEvent::new(
            LunaEventType::SystemError,
            "system".to_string(),
            EventPriority::Critical,
            serde_json::json!({}),
        );
        
        let system = EventSystem::new();
        assert!(system.event_matches_filter(&event1, &filter));
        assert!(!system.event_matches_filter(&event2, &filter));
    }
}