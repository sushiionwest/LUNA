/*!
 * Luna Core - Simplified core functionality with minimal dependencies
 * 
 * Handles command processing and action execution using lightweight patterns
 */

use anyhow::Result;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use log::{info, debug, warn, error};

use crate::ai::AICoordinator;
use crate::input::{
    ActionType, BasicSafetyChecker, InputAction, InputController, MouseButton, ScrollDirection,
    Target,
};
use crate::utils::image_processing::Image;
use crate::vision::screen_capture::{CaptureConfig, ScreenCapture};

pub mod config;
pub mod error;
pub mod safety;

pub use error::LunaError;
pub use config::LunaConfig;

/// Screen analysis result
#[derive(Debug, Clone)]
pub struct ScreenAnalysis {
    pub elements: Vec<ScreenElement>,
    pub confidence: f32,
    pub processing_time_ms: u64,
    pub screen_size: (u32, u32),
}

/// Detected screen element
#[derive(Debug, Clone)]
pub struct ScreenElement {
    pub element_type: String,
    pub bounds: ElementBounds,
    pub confidence: f32,
    pub text: Option<String>,
    pub attributes: std::collections::HashMap<String, String>,
}

/// Element bounds rectangle
#[derive(Debug, Clone)]
pub struct ElementBounds {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// Action to be executed by Luna
#[derive(Debug, Clone)]
pub enum LunaAction {
    /// Click at specific coordinates
    Click { x: i32, y: i32 },
    /// Type text
    Type { text: String },
    /// Key combination
    KeyCombo { keys: Vec<String> },
    /// Scroll in direction
    Scroll { direction: String, amount: i32 },
    /// Wait for specified time
    Wait { milliseconds: u64 },
}

/// Luna event for coordination
#[derive(Debug, Clone)]
pub enum LunaEvent {
    /// Command received from user
    CommandReceived { command: String },
    /// Screen analysis completed
    AnalysisComplete { analysis: ScreenAnalysis },
    /// Actions planned
    ActionsPlanned { actions: Vec<LunaAction> },
    /// Action executed
    ActionExecuted { action: LunaAction, success: bool },
    /// Error occurred
    Error { error: String },
}

/// Main Luna coordinator
pub struct Luna {
    /// AI coordinator for screen analysis
    ai_coordinator: AICoordinator,
    /// Screen capture system
    screen_capture: ScreenCapture,
    /// Input system for executing actions
    input_system: InputController,
    /// Safety system for validating commands
    safety_system: Arc<safety::SafetySystem>,
    /// Configuration
    config: LunaConfig,
    /// Processing statistics
    stats: Arc<Mutex<ProcessingStats>>,
    /// Event subscribers
    event_subscribers: Arc<Mutex<Vec<Box<dyn Fn(LunaEvent) + Send + Sync>>>>,
}

/// Processing statistics
#[derive(Debug, Default, Clone)]
pub struct ProcessingStats {
    pub commands_processed: u64,
    pub actions_executed: u64,
    pub safety_blocks: u64,
    pub total_processing_time_ms: u64,
    pub average_processing_time_ms: f64,
}

impl Luna {
    /// Create a new Luna instance with the given configuration
    pub fn new(config: LunaConfig) -> Result<Self> {
        Ok(Self {
            ai_coordinator: AICoordinator::new(),
            screen_capture: ScreenCapture::new(CaptureConfig::default()),
            input_system: InputController::new(Box::new(BasicSafetyChecker::new())),
            safety_system: Arc::new(safety::SafetySystem::new(&config)),
            config,
            stats: Arc::new(Mutex::new(ProcessingStats::default())),
            event_subscribers: Arc::new(Mutex::new(Vec::new())),
        })
    }

    /// Process user command and execute actions
    pub fn process_command(&mut self, command: &str) -> Result<Vec<LunaAction>> {
        let start_time = Instant::now();
        
        info!("Processing command: '{}'", command);
        self.emit_event(LunaEvent::CommandReceived { 
            command: command.to_string() 
        });

        // Step 1: Safety check
        if !self.safety_system.is_command_safe(command) {
            warn!("Command blocked by safety system: '{}'", command);
            self.update_stats(|stats| stats.safety_blocks += 1);
            return Err(LunaError::UnsafeCommand(command.to_string()).into());
        }

        // Step 2: Capture current screen
        let screenshot = self.screen_capture.capture_screen()?;
        debug!("Screen captured: {}x{}", screenshot.width, screenshot.height);

        // Step 3: Analyze screen to understand current state
        let dynamic_image = to_dynamic_image(&screenshot)?;
        let analysis = self.ai_coordinator.analyze_screen(&dynamic_image)?;
        debug!("Screen analysis complete: {} elements detected", analysis.elements.len());
        
        self.emit_event(LunaEvent::AnalysisComplete { 
            analysis: analysis.clone() 
        });

        // Step 4: Plan actions based on command and screen state
        let actions = self.ai_coordinator.plan_actions(command, &analysis)?;
        debug!("Planned {} actions", actions.len());
        
        self.emit_event(LunaEvent::ActionsPlanned { 
            actions: actions.clone() 
        });

        // Step 5: Validate actions with safety system
        for action in &actions {
            if !self.safety_system.is_action_safe(action) {
                warn!("Action blocked by safety system: {:?}", action);
                self.update_stats(|stats| stats.safety_blocks += 1);
                return Err(LunaError::UnsafeAction(format!("{:?}", action)).into());
            }
        }

        // Step 6: Execute actions
        for action in &actions {
            match self.execute_single_action(action) {
                Ok(_) => {
                    debug!("Action executed successfully: {:?}", action);
                    self.emit_event(LunaEvent::ActionExecuted { 
                        action: action.clone(), 
                        success: true 
                    });
                }
                Err(e) => {
                    error!("Failed to execute action {:?}: {}", action, e);
                    self.emit_event(LunaEvent::ActionExecuted { 
                        action: action.clone(), 
                        success: false 
                    });
                    return Err(e);
                }
            }
            
            // Small delay between actions for stability
            std::thread::sleep(Duration::from_millis(50));
        }

        // Update statistics
        let processing_time = start_time.elapsed();
        let processing_time_ms = processing_time.as_millis() as u64;
        
        self.update_stats(|stats| {
            stats.commands_processed += 1;
            stats.actions_executed += actions.len() as u64;
            stats.total_processing_time_ms += processing_time_ms;
            stats.average_processing_time_ms = 
                stats.total_processing_time_ms as f64 / stats.commands_processed as f64;
        });

        info!("Command processed successfully in {}ms: {} actions executed", 
              processing_time_ms, actions.len());

        Ok(actions)
    }

    /// Get current screen analysis without executing actions
    pub fn analyze_current_screen(&mut self) -> Result<ScreenAnalysis> {
        let screenshot = self.screen_capture.capture_screen()?;
        let dynamic_image = to_dynamic_image(&screenshot)?;
        self.ai_coordinator.analyze_screen(&dynamic_image)
    }

    /// Execute one planned action through the guarded input layer
    fn execute_single_action(&mut self, action: &LunaAction) -> Result<()> {
        if let LunaAction::Wait { milliseconds } = action {
            std::thread::sleep(Duration::from_millis(*milliseconds));
            return Ok(());
        }

        let input_action = to_input_action(action)?;
        self.input_system.execute_action(input_action)?;
        Ok(())
    }

    /// Subscribe to Luna events
    pub fn subscribe_to_events<F>(&self, callback: F) 
    where 
        F: Fn(LunaEvent) + Send + Sync + 'static,
    {
        if let Ok(mut subscribers) = self.event_subscribers.lock() {
            subscribers.push(Box::new(callback));
        }
    }

    /// Get processing statistics
    pub fn get_stats(&self) -> ProcessingStats {
        self.stats
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    /// Get configuration
    pub fn get_config(&self) -> &LunaConfig {
        &self.config
    }

    /// Update configuration
    pub fn update_config(&mut self, config: LunaConfig) -> Result<()> {
        self.config = config.clone();
        self.safety_system = Arc::new(safety::SafetySystem::new(&config));
        Ok(())
    }

    /// Check if Luna is ready to process commands
    pub fn is_ready(&self) -> bool {
        // Simple readiness check
        true
    }

    /// Emit event to all subscribers
    fn emit_event(&self, event: LunaEvent) {
        if let Ok(subscribers) = self.event_subscribers.lock() {
            for callback in subscribers.iter() {
                callback(event.clone());
            }
        }
    }

    /// Update statistics with a closure
    fn update_stats<F>(&self, updater: F) 
    where 
        F: FnOnce(&mut ProcessingStats),
    {
        if let Ok(mut stats) = self.stats.lock() {
            updater(&mut *stats);
        }
    }
}

impl Default for Luna {
    fn default() -> Self {
        Self::new(LunaConfig::default()).expect("Failed to create default Luna")
    }
}

// Helper functions for common operations
impl Luna {
    /// Click at specific coordinates
    pub fn click(&mut self, x: i32, y: i32) -> Result<()> {
        let action = LunaAction::Click { x, y };
        if self.safety_system.is_action_safe(&action) {
            self.execute_single_action(&action)
        } else {
            Err(LunaError::UnsafeAction(format!("Click at ({}, {})", x, y)).into())
        }
    }

    /// Type text
    pub fn type_text(&mut self, text: &str) -> Result<()> {
        let action = LunaAction::Type { text: text.to_string() };
        if self.safety_system.is_action_safe(&action) {
            self.execute_single_action(&action)
        } else {
            Err(LunaError::UnsafeAction(format!("Type text: {}", text)).into())
        }
    }

    /// Send key combination
    pub fn send_keys(&mut self, keys: Vec<String>) -> Result<()> {
        let action = LunaAction::KeyCombo { keys };
        if self.safety_system.is_action_safe(&action) {
            self.execute_single_action(&action)
        } else {
            Err(LunaError::UnsafeAction("Key combination".to_string()).into())
        }
    }

    /// Scroll in direction
    pub fn scroll(&mut self, direction: &str, amount: i32) -> Result<()> {
        let action = LunaAction::Scroll {
            direction: direction.to_string(),
            amount
        };
        if self.safety_system.is_action_safe(&action) {
            self.execute_single_action(&action)
        } else {
            Err(LunaError::UnsafeAction(format!("Scroll {}", direction)).into())
        }
    }
}

/// Convert the internal image buffer to an `image::DynamicImage` for the CV pipeline
fn to_dynamic_image(image: &Image) -> Result<image::DynamicImage> {
    let width = image.width as u32;
    let height = image.height as u32;
    let data = image.data.clone();

    match image.channels {
        1 => image::GrayImage::from_raw(width, height, data)
            .map(image::DynamicImage::ImageLuma8),
        3 => image::RgbImage::from_raw(width, height, data)
            .map(image::DynamicImage::ImageRgb8),
        4 => image::RgbaImage::from_raw(width, height, data)
            .map(image::DynamicImage::ImageRgba8),
        _ => None,
    }
    .ok_or_else(|| {
        anyhow::anyhow!(
            "invalid image buffer: {}x{} with {} channels",
            image.width,
            image.height,
            image.channels
        )
    })
}

/// Convert a planned `LunaAction` into the input layer's `InputAction`.
///
/// `Wait` is handled by the coordinator directly and is rejected here.
fn to_input_action(action: &LunaAction) -> Result<InputAction> {
    let (action_type, target) = match action {
        LunaAction::Click { x, y } => (
            ActionType::Click { button: MouseButton::Left },
            Target { x: *x, y: *y, element_type: None },
        ),
        LunaAction::Type { text } => (
            ActionType::Type { text: text.clone() },
            Target { x: 0, y: 0, element_type: None },
        ),
        LunaAction::KeyCombo { keys } => (
            ActionType::Key { key: keys.join("+").to_lowercase() },
            Target { x: 0, y: 0, element_type: None },
        ),
        LunaAction::Scroll { direction, amount } => {
            let scroll_direction = match direction.to_lowercase().as_str() {
                "up" => ScrollDirection::Up,
                "left" => ScrollDirection::Left,
                "right" => ScrollDirection::Right,
                _ => ScrollDirection::Down,
            };
            (
                ActionType::Scroll { direction: scroll_direction, amount: *amount },
                Target { x: 0, y: 0, element_type: None },
            )
        }
        LunaAction::Wait { .. } => {
            return Err(anyhow::anyhow!("Wait actions are executed by the coordinator"));
        }
    };

    Ok(InputAction {
        action_type,
        target,
        timestamp: Instant::now(),
    })
}