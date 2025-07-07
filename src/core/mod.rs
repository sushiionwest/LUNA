/*!
 * Luna Core - The brain of Luna Visual AI
 * 
 * Handles command processing, AI coordination, and action execution
 */

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, debug, error};

pub mod config;
pub mod error;
pub mod events;
pub mod memory;
pub mod safety;

use crate::ai::AIVisionPipeline;
use crate::input::WindowsInputSystem;
use crate::overlay::VisualOverlay;
use crate::vision::ScreenCapture;

pub use error::LunaError;
pub use events::LunaEvent;
pub use config::LunaConfig;

/// Main Luna Core - coordinates all subsystems
pub struct LunaCore {
    /// AI vision processing pipeline
    ai_pipeline: Arc<AIVisionPipeline>,
    /// Screen capture system
    screen_capture: Arc<ScreenCapture>,
    /// Input system for mouse/keyboard
    input_system: Arc<WindowsInputSystem>,
    /// Visual feedback overlay
    overlay: Arc<RwLock<VisualOverlay>>,
    /// Configuration
    config: LunaConfig,
    /// Safety system
    safety: safety::SafetySystem,
    /// Memory manager
    memory: memory::MemoryManager,
}

impl LunaCore {
    /// Initialize new Luna Core instance
    pub async fn new() -> Result<Self> {
        info!("Initializing Luna Core...");
        
        // Load configuration
        let config = LunaConfig::load()?;
        debug!("Configuration loaded: {:?}", config);
        
        // Initialize AI pipeline
        let ai_pipeline = Arc::new(AIVisionPipeline::new().await?);
        info!("AI pipeline initialized");
        
        // Initialize screen capture
        let screen_capture = Arc::new(ScreenCapture::new()?);
        info!("Screen capture system ready");
        
        // Initialize input system
        let input_system = Arc::new(WindowsInputSystem::new()?);
        info!("Input system initialized");
        
        // Initialize visual overlay
        let overlay = Arc::new(RwLock::new(VisualOverlay::new()?));
        info!("Visual overlay ready");
        
        // Initialize safety system
        let safety = safety::SafetySystem::new(&config);
        info!("Safety system active");
        
        // Initialize memory manager
        let memory = memory::MemoryManager::new();
        info!("Memory manager initialized");
        
        info!("✅ Luna Core fully initialized and ready!");
        
        Ok(Self {
            ai_pipeline,
            screen_capture,
            input_system,
            overlay,
            config,
            safety,
            memory,
        })
    }
    
    /// Execute a natural language command
    pub async fn execute_command(&self, command: &str) -> Result<()> {
        info!("Executing command: '{}'", command);
        
        // Safety check
        if !self.safety.is_command_safe(command) {
            return Err(LunaError::UnsafeCommand(command.to_string()).into());
        }
        
        // Step 1: Take screenshot
        debug!("Step 1: Capturing screen...");
        let screenshot = self.screen_capture.capture_screen().await?;
        debug!("Screenshot captured: {}x{}", screenshot.width(), screenshot.height());
        
        // Step 2: Analyze with AI
        debug!("Step 2: Analyzing screen with AI...");
        let analysis = self.ai_pipeline.analyze_screen(&screenshot, command).await?;
        debug!("AI analysis complete: {} elements found", analysis.elements.len());
        
        // Step 3: Plan actions
        debug!("Step 3: Planning actions...");
        let actions = self.ai_pipeline.plan_actions(&analysis, command).await?;
        debug!("Action plan: {} actions planned", actions.len());
        
        // Step 4: Show visual preview
        debug!("Step 4: Showing visual preview...");
        {
            let mut overlay = self.overlay.write().await;
            overlay.show_action_preview(&actions, &screenshot).await?;
        }
        
        // Step 5: Wait for confirmation (3 second countdown)
        debug!("Step 5: Waiting for confirmation...");
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
        
        // Step 6: Execute actions
        debug!("Step 6: Executing actions...");
        for (i, action) in actions.iter().enumerate() {
            info!("Executing action {}/{}: {:?}", i + 1, actions.len(), action);
            self.input_system.execute_action(action).await?;
            
            // Small delay between actions
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
        
        // Hide overlay
        {
            let mut overlay = self.overlay.write().await;
            overlay.hide().await;
        }
        
        info!("✅ Command '{}' executed successfully!", command);
        Ok(())
    }
    
    /// Get current system status
    pub fn get_status(&self) -> Result<LunaStatus> {
        Ok(LunaStatus {
            ai_ready: self.ai_pipeline.is_ready(),
            memory_usage: self.memory.get_usage(),
            safety_enabled: self.safety.is_enabled(),
            uptime: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        })
    }
    
    /// Emergency stop all operations
    pub async fn emergency_stop(&self) -> Result<()> {
        info!("🛑 Emergency stop activated!");
        
        // Hide overlay immediately
        {
            let mut overlay = self.overlay.write().await;
            overlay.hide().await;
        }
        
        // Cancel any pending actions
        self.input_system.cancel_all().await?;
        
        info!("All operations stopped");
        Ok(())
    }
}

/// Luna system status
#[derive(Debug, Clone)]
pub struct LunaStatus {
    pub ai_ready: bool,
    pub memory_usage: u64,
    pub safety_enabled: bool,
    pub uptime: u64,
}

/// Action types that Luna can perform
#[derive(Debug, Clone)]
pub enum LunaAction {
    Click { x: i32, y: i32 },
    RightClick { x: i32, y: i32 },
    DoubleClick { x: i32, y: i32 },
    Type { text: String },
    KeyPress { key: String },
    KeyCombo { keys: Vec<String> },
    Scroll { x: i32, y: i32, direction: ScrollDirection },
    Drag { from_x: i32, from_y: i32, to_x: i32, to_y: i32 },
    Wait { milliseconds: u64 },
}

#[derive(Debug, Clone)]
pub enum ScrollDirection {
    Up,
    Down,
    Left,
    Right,
}

/// Screen element detected by AI
#[derive(Debug, Clone)]
pub struct ScreenElement {
    pub element_type: String,
    pub text: Option<String>,
    pub bounds: ElementBounds,
    pub confidence: f32,
    pub clickable: bool,
}

#[derive(Debug, Clone)]
pub struct ElementBounds {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// AI analysis result
#[derive(Debug, Clone)]
pub struct ScreenAnalysis {
    pub elements: Vec<ScreenElement>,
    pub text_content: String,
    pub screenshot_hash: String,
    pub analysis_time_ms: u64,
}