/*!
 * Luna AI Vision Pipeline - Main orchestrator for computer vision tasks
 */

use anyhow::Result;
use image::DynamicImage;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, debug, warn};

use crate::core::{ScreenAnalysis, LunaAction};
use super::{AICoordinator, ModelInfo};

/// Main AI vision pipeline that coordinates all AI models
pub struct AIVisionPipeline {
    coordinator: Arc<RwLock<Option<AICoordinator>>>,
    is_ready: Arc<std::sync::atomic::AtomicBool>,
    stats: Arc<RwLock<PipelineStats>>,
}

#[derive(Debug, Default, Clone)]
pub struct PipelineStats {
    pub total_analyses: u64,
    pub successful_analyses: u64,
    pub failed_analyses: u64,
    pub total_processing_time_ms: u64,
    pub average_processing_time_ms: f64,
    pub elements_detected: u64,
    pub actions_planned: u64,
}

impl AIVisionPipeline {
    /// Create new AI vision pipeline
    pub async fn new() -> Result<Self> {
        info!("Initializing AI Vision Pipeline...");
        
        let pipeline = Self {
            coordinator: Arc::new(RwLock::new(None)),
            is_ready: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            stats: Arc::new(RwLock::new(PipelineStats::default())),
        };
        
        // Initialize in background
        pipeline.initialize_coordinator().await?;
        
        Ok(pipeline)
    }
    
    async fn initialize_coordinator(&self) -> Result<()> {
        debug!("Loading AI models...");
        
        match AICoordinator::new().await {
            Ok(coordinator) => {
                *self.coordinator.write().await = Some(coordinator);
                self.is_ready.store(true, std::sync::atomic::Ordering::SeqCst);
                info!("✅ AI Vision Pipeline ready!");
                Ok(())
            }
            Err(e) => {
                warn!("Failed to initialize AI coordinator: {}", e);
                // Continue with limited functionality
                self.is_ready.store(false, std::sync::atomic::Ordering::SeqCst);
                Err(e)
            }
        }
    }
    
    /// Analyze screen image and extract elements
    pub async fn analyze_screen(&self, image: &DynamicImage, command: &str) -> Result<ScreenAnalysis> {
        let start_time = std::time::Instant::now();
        
        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.total_analyses += 1;
        }
        
        // Check if coordinator is ready
        let coordinator_guard = self.coordinator.read().await;
        let coordinator = match coordinator_guard.as_ref() {
            Some(coord) => coord,
            None => {
                warn!("AI coordinator not ready, falling back to basic analysis");
                return self.fallback_analysis(image, command).await;
            }
        };
        
        // Perform analysis
        match coordinator.analyze_screen(image, command).await {
            Ok(analysis) => {
                let processing_time = start_time.elapsed().as_millis() as u64;
                
                // Update stats
                {
                    let mut stats = self.stats.write().await;
                    stats.successful_analyses += 1;
                    stats.total_processing_time_ms += processing_time;
                    stats.average_processing_time_ms = 
                        stats.total_processing_time_ms as f64 / stats.successful_analyses as f64;
                    stats.elements_detected += analysis.elements.len() as u64;
                }
                
                debug!("Analysis completed successfully in {}ms", processing_time);
                Ok(analysis)
            }
            Err(e) => {
                // Update stats
                {
                    let mut stats = self.stats.write().await;
                    stats.failed_analyses += 1;
                }
                
                warn!("AI analysis failed: {}, falling back to basic analysis", e);
                self.fallback_analysis(image, command).await
            }
        }
    }
    
    /// Plan actions based on analysis
    pub async fn plan_actions(&self, analysis: &ScreenAnalysis, command: &str) -> Result<Vec<LunaAction>> {
        // Check if coordinator is ready
        let coordinator_guard = self.coordinator.read().await;
        let coordinator = match coordinator_guard.as_ref() {
            Some(coord) => coord,
            None => {
                warn!("AI coordinator not ready, falling back to basic planning");
                return self.fallback_planning(analysis, command);
            }
        };
        
        // Plan actions
        match coordinator.plan_actions(analysis, command).await {
            Ok(actions) => {
                // Update stats
                {
                    let mut stats = self.stats.write().await;
                    stats.actions_planned += actions.len() as u64;
                }
                
                debug!("Planned {} actions for command: '{}'", actions.len(), command);
                Ok(actions)
            }
            Err(e) => {
                warn!("AI action planning failed: {}, falling back to basic planning", e);
                self.fallback_planning(analysis, command)
            }
        }
    }
    
    /// Fallback analysis when AI models are not available
    async fn fallback_analysis(&self, image: &DynamicImage, _command: &str) -> Result<ScreenAnalysis> {
        debug!("Using fallback analysis");
        
        // Simple fallback: create mock analysis
        let analysis = ScreenAnalysis {
            elements: vec![
                // Mock some common UI elements
                crate::core::ScreenElement {
                    element_type: "button".to_string(),
                    text: Some("Button".to_string()),
                    bounds: crate::core::ElementBounds {
                        x: 100,
                        y: 100,
                        width: 80,
                        height: 30,
                    },
                    confidence: 0.5,
                    clickable: true,
                },
            ],
            text_content: "Fallback analysis - limited functionality".to_string(),
            screenshot_hash: format!("fallback_{}", image.width()),
            analysis_time_ms: 10,
        };
        
        Ok(analysis)
    }
    
    /// Fallback action planning when AI models are not available
    fn fallback_planning(&self, analysis: &ScreenAnalysis, command: &str) -> Result<Vec<LunaAction>> {
        debug!("Using fallback action planning");
        
        let command_lower = command.to_lowercase();
        let mut actions = Vec::new();
        
        // Very basic command parsing
        if command_lower.contains("click") {
            // Click somewhere in the middle of the screen
            actions.push(LunaAction::Click { x: 500, y: 400 });
        } else if command_lower.contains("type") {
            // Extract text to type
            if let Some(text_start) = command.find("type") {
                let text = command[text_start + 4..].trim();
                if !text.is_empty() {
                    actions.push(LunaAction::Type {
                        text: text.to_string(),
                    });
                }
            }
        } else if command_lower.contains("ctrl") && command_lower.contains("c") {
            actions.push(LunaAction::KeyCombo {
                keys: vec!["ctrl".to_string(), "c".to_string()],
            });
        } else if command_lower.contains("ctrl") && command_lower.contains("v") {
            actions.push(LunaAction::KeyCombo {
                keys: vec!["ctrl".to_string(), "v".to_string()],
            });
        } else {
            // Default: click the first clickable element if any
            if let Some(element) = analysis.elements.iter().find(|e| e.clickable) {
                actions.push(LunaAction::Click {
                    x: element.bounds.x + element.bounds.width / 2,
                    y: element.bounds.y + element.bounds.height / 2,
                });
            }
        }
        
        if actions.is_empty() {
            return Err(anyhow::anyhow!("Could not plan actions for command: {}", command));
        }
        
        Ok(actions)
    }
    
    /// Check if pipeline is ready
    pub fn is_ready(&self) -> bool {
        self.is_ready.load(std::sync::atomic::Ordering::SeqCst)
    }
    
    /// Get pipeline statistics
    pub async fn get_stats(&self) -> PipelineStats {
        let stats = self.stats.read().await;
        stats.clone()
    }
    
    /// Reset statistics
    pub async fn reset_stats(&self) {
        let mut stats = self.stats.write().await;
        *stats = PipelineStats::default();
    }
    
    /// Get available models information
    pub fn get_available_models(&self) -> Vec<ModelInfo> {
        super::get_model_info()
    }
    
    /// Reload AI coordinator (useful for model updates)
    pub async fn reload(&self) -> Result<()> {
        info!("Reloading AI coordinator...");
        
        // Mark as not ready during reload
        self.is_ready.store(false, std::sync::atomic::Ordering::SeqCst);
        
        // Clear current coordinator
        {
            let mut coordinator = self.coordinator.write().await;
            *coordinator = None;
        }
        
        // Reinitialize
        self.initialize_coordinator().await
    }
    
    /// Shutdown pipeline and cleanup resources
    pub async fn shutdown(&self) {
        info!("Shutting down AI pipeline...");
        
        self.is_ready.store(false, std::sync::atomic::Ordering::SeqCst);
        
        let mut coordinator = self.coordinator.write().await;
        *coordinator = None;
        
        info!("AI pipeline shutdown complete");
    }
    
    /// Get detailed status information
    pub async fn get_status(&self) -> PipelineStatus {
        let is_ready = self.is_ready();
        let stats = self.get_stats().await;
        let models = self.get_available_models();
        
        PipelineStatus {
            ready: is_ready,
            models_loaded: if is_ready { models.len() } else { 0 },
            total_models: models.len(),
            stats,
            models,
        }
    }
}

/// Detailed pipeline status
#[derive(Debug, Clone)]
pub struct PipelineStatus {
    pub ready: bool,
    pub models_loaded: usize,
    pub total_models: usize,
    pub stats: PipelineStats,
    pub models: Vec<ModelInfo>,
}

impl PipelineStatus {
    pub fn readiness_percentage(&self) -> f32 {
        if self.total_models == 0 {
            0.0
        } else {
            (self.models_loaded as f32 / self.total_models as f32) * 100.0
        }
    }
    
    pub fn success_rate(&self) -> f32 {
        if self.stats.total_analyses == 0 {
            0.0
        } else {
            (self.stats.successful_analyses as f32 / self.stats.total_analyses as f32) * 100.0
        }
    }
}