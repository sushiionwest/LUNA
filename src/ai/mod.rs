/*!
 * Luna AI Pipeline - Computer Vision and Natural Language Processing
 * 
 * Handles screen analysis, element detection, and action planning
 */

use anyhow::Result;
use image::{DynamicImage, RgbaImage};
use std::sync::Arc;
use tracing::{info, debug, error};

pub mod pipeline;
pub mod model_manager;
pub mod clip;
pub mod florence;
pub mod sam;
pub mod trocr;

use crate::core::{ScreenAnalysis, ScreenElement, LunaAction, ElementBounds};

pub use pipeline::AIVisionPipeline;

/// AI processing result
#[derive(Debug, Clone)]
pub struct AIResult {
    pub elements: Vec<ScreenElement>,
    pub confidence: f32,
    pub processing_time_ms: u64,
}

/// Element detection result
#[derive(Debug, Clone)]
pub struct ElementDetection {
    pub element_type: String,
    pub bounds: ElementBounds,
    pub confidence: f32,
    pub text: Option<String>,
    pub attributes: std::collections::HashMap<String, String>,
}

/// Action planning result
#[derive(Debug, Clone)]
pub struct ActionPlan {
    pub actions: Vec<LunaAction>,
    pub confidence: f32,
    pub explanation: String,
}

/// AI model interface
pub trait AIModel {
    fn name(&self) -> &str;
    fn is_loaded(&self) -> bool;
    fn load(&mut self) -> Result<()>;
    fn unload(&mut self);
}

/// Vision model interface for screen analysis
pub trait VisionModel: AIModel {
    fn analyze_screen(&self, image: &DynamicImage) -> Result<Vec<ElementDetection>>;
    fn detect_text(&self, image: &DynamicImage) -> Result<String>;
    fn find_elements(&self, image: &DynamicImage, query: &str) -> Result<Vec<ElementDetection>>;
}

/// Language model interface for command understanding
pub trait LanguageModel: AIModel {
    fn understand_command(&self, command: &str, context: &ScreenAnalysis) -> Result<ActionPlan>;
    fn match_elements(&self, command: &str, elements: &[ScreenElement]) -> Result<Vec<usize>>;
}

/// Main AI coordinator
pub struct AICoordinator {
    vision_models: Vec<Box<dyn VisionModel + Send + Sync>>,
    language_models: Vec<Box<dyn LanguageModel + Send + Sync>>,
    model_manager: Arc<model_manager::ModelManager>,
}

impl AICoordinator {
    pub async fn new() -> Result<Self> {
        info!("Initializing AI coordinator...");
        
        let model_manager = Arc::new(model_manager::ModelManager::new().await?);
        
        let mut coordinator = Self {
            vision_models: Vec::new(),
            language_models: Vec::new(),
            model_manager,
        };
        
        // Initialize vision models
        coordinator.init_vision_models().await?;
        
        // Initialize language models
        coordinator.init_language_models().await?;
        
        info!("AI coordinator initialized with {} vision models and {} language models", 
              coordinator.vision_models.len(), 
              coordinator.language_models.len());
        
        Ok(coordinator)
    }
    
    async fn init_vision_models(&mut self) -> Result<()> {
        // Add CLIP model for general vision understanding
        let clip_model = Box::new(clip::CLIPModel::new(&self.model_manager).await?);
        self.vision_models.push(clip_model);
        
        // Add Florence-2 for detailed scene understanding  
        let florence_model = Box::new(florence::FlorenceModel::new(&self.model_manager).await?);
        self.vision_models.push(florence_model);
        
        // Add TrOCR for text recognition
        let trocr_model = Box::new(trocr::TrOCRModel::new(&self.model_manager).await?);
        self.vision_models.push(trocr_model);
        
        Ok(())
    }
    
    async fn init_language_models(&mut self) -> Result<()> {
        // For now, we'll use a simple rule-based language model
        // In the future, we could add a proper LLM here
        Ok(())
    }
    
    /// Analyze screen image and extract elements
    pub async fn analyze_screen(&self, image: &DynamicImage, command: &str) -> Result<ScreenAnalysis> {
        let start_time = std::time::Instant::now();
        debug!("Starting screen analysis for command: '{}'", command);
        
        let mut all_elements = Vec::new();
        let mut text_content = String::new();
        
        // Run all vision models in parallel
        let mut tasks = Vec::new();
        
        for model in &self.vision_models {
            let image_clone = image.clone();
            let model_name = model.name().to_string();
            
            // Run each model (in a real async implementation, these would be futures)
            match model.analyze_screen(&image_clone) {
                Ok(detections) => {
                    debug!("Model '{}' found {} elements", model_name, detections.len());
                    
                    // Convert detections to screen elements
                    for detection in detections {
                        let element = ScreenElement {
                            element_type: detection.element_type,
                            text: detection.text.clone(),
                            bounds: detection.bounds,
                            confidence: detection.confidence,
                            clickable: self.is_clickable(&detection.element_type),
                        };
                        all_elements.push(element);
                        
                        // Collect text content
                        if let Some(text) = detection.text {
                            text_content.push_str(&text);
                            text_content.push(' ');
                        }
                    }
                }
                Err(e) => {
                    error!("Model '{}' failed: {}", model_name, e);
                }
            }
        }
        
        // Deduplicate and merge similar elements
        all_elements = self.deduplicate_elements(all_elements);
        
        let processing_time = start_time.elapsed().as_millis() as u64;
        
        debug!("Screen analysis completed in {}ms, found {} unique elements", 
               processing_time, all_elements.len());
        
        Ok(ScreenAnalysis {
            elements: all_elements,
            text_content: text_content.trim().to_string(),
            screenshot_hash: self.calculate_image_hash(image),
            analysis_time_ms: processing_time,
        })
    }
    
    /// Plan actions based on analysis and command
    pub async fn plan_actions(&self, analysis: &ScreenAnalysis, command: &str) -> Result<Vec<LunaAction>> {
        debug!("Planning actions for command: '{}'", command);
        
        let actions = self.parse_command_to_actions(command, analysis)?;
        
        debug!("Planned {} actions", actions.len());
        Ok(actions)
    }
    
    /// Simple command parsing (can be enhanced with LLM)
    fn parse_command_to_actions(&self, command: &str, analysis: &ScreenAnalysis) -> Result<Vec<LunaAction>> {
        let command_lower = command.to_lowercase();
        let mut actions = Vec::new();
        
        // Close all tabs
        if command_lower.contains("close") && command_lower.contains("tab") {
            let close_buttons = self.find_close_buttons(&analysis.elements);
            for button in close_buttons {
                actions.push(LunaAction::Click {
                    x: button.bounds.x + button.bounds.width / 2,
                    y: button.bounds.y + button.bounds.height / 2,
                });
                actions.push(LunaAction::Wait { milliseconds: 100 });
            }
        }
        
        // Click specific element
        else if command_lower.starts_with("click") {
            let target = command_lower.strip_prefix("click").unwrap_or("").trim();
            if let Some(element) = self.find_element_by_text(&analysis.elements, target) {
                actions.push(LunaAction::Click {
                    x: element.bounds.x + element.bounds.width / 2,
                    y: element.bounds.y + element.bounds.height / 2,
                });
            }
        }
        
        // Type text
        else if command_lower.starts_with("type") {
            let text = command.strip_prefix("type").unwrap_or("").trim();
            if !text.is_empty() {
                actions.push(LunaAction::Type {
                    text: text.to_string(),
                });
            }
        }
        
        // Key combinations
        else if command_lower.contains("ctrl") || command_lower.contains("alt") {
            let keys = self.parse_key_combination(&command_lower);
            if !keys.is_empty() {
                actions.push(LunaAction::KeyCombo { keys });
            }
        }
        
        // Scroll
        else if command_lower.contains("scroll") {
            let direction = if command_lower.contains("down") {
                crate::core::ScrollDirection::Down
            } else if command_lower.contains("up") {
                crate::core::ScrollDirection::Up
            } else {
                crate::core::ScrollDirection::Down
            };
            
            actions.push(LunaAction::Scroll {
                x: 500, // Center of typical screen
                y: 400,
                direction,
            });
        }
        
        // Default: try to find and click the mentioned element
        else {
            if let Some(element) = self.find_best_match(&analysis.elements, &command_lower) {
                actions.push(LunaAction::Click {
                    x: element.bounds.x + element.bounds.width / 2,
                    y: element.bounds.y + element.bounds.height / 2,
                });
            }
        }
        
        if actions.is_empty() {
            return Err(anyhow::anyhow!("Could not understand command: {}", command));
        }
        
        Ok(actions)
    }
    
    fn find_close_buttons(&self, elements: &[ScreenElement]) -> Vec<&ScreenElement> {
        elements.iter()
            .filter(|e| {
                e.element_type == "button" && 
                e.text.as_ref().map_or(false, |t| t.contains("×") || t.contains("close"))
            })
            .collect()
    }
    
    fn find_element_by_text(&self, elements: &[ScreenElement], target: &str) -> Option<&ScreenElement> {
        elements.iter()
            .find(|e| {
                e.text.as_ref().map_or(false, |t| 
                    t.to_lowercase().contains(target) || 
                    target.contains(&t.to_lowercase())
                )
            })
    }
    
    fn find_best_match(&self, elements: &[ScreenElement], command: &str) -> Option<&ScreenElement> {
        let mut best_match = None;
        let mut best_score = 0.0;
        
        for element in elements {
            if !element.clickable {
                continue;
            }
            
            let score = self.calculate_match_score(element, command);
            if score > best_score {
                best_score = score;
                best_match = Some(element);
            }
        }
        
        if best_score > 0.3 { // Minimum confidence threshold
            best_match
        } else {
            None
        }
    }
    
    fn calculate_match_score(&self, element: &ScreenElement, command: &str) -> f32 {
        let mut score = 0.0;
        
        // Check element type match
        if command.contains(&element.element_type) {
            score += 0.3;
        }
        
        // Check text content match
        if let Some(text) = &element.text {
            let text_lower = text.to_lowercase();
            let words: Vec<&str> = command.split_whitespace().collect();
            
            for word in words {
                if text_lower.contains(word) {
                    score += 0.4 / words.len() as f32;
                }
            }
        }
        
        // Boost score for buttons and links
        if element.element_type == "button" || element.element_type == "link" {
            score += 0.1;
        }
        
        // Factor in AI confidence
        score *= element.confidence;
        
        score
    }
    
    fn parse_key_combination(&self, command: &str) -> Vec<String> {
        let mut keys = Vec::new();
        
        if command.contains("ctrl") {
            keys.push("ctrl".to_string());
        }
        if command.contains("alt") {
            keys.push("alt".to_string());
        }
        if command.contains("shift") {
            keys.push("shift".to_string());
        }
        
        // Extract the main key
        if command.contains("ctrl+c") || command.contains("copy") {
            keys.push("c".to_string());
        } else if command.contains("ctrl+v") || command.contains("paste") {
            keys.push("v".to_string());
        } else if command.contains("ctrl+s") || command.contains("save") {
            keys.push("s".to_string());
        }
        
        keys
    }
    
    fn is_clickable(&self, element_type: &str) -> bool {
        matches!(element_type, "button" | "link" | "checkbox" | "radio" | "tab" | "menu" | "icon")
    }
    
    fn deduplicate_elements(&self, mut elements: Vec<ScreenElement>) -> Vec<ScreenElement> {
        // Simple deduplication based on position and type
        elements.sort_by(|a, b| {
            a.bounds.x.cmp(&b.bounds.x)
                .then(a.bounds.y.cmp(&b.bounds.y))
                .then(a.element_type.cmp(&b.element_type))
        });
        
        let mut deduplicated = Vec::new();
        let mut last_element: Option<&ScreenElement> = None;
        
        for element in &elements {
            let should_keep = match last_element {
                None => true,
                Some(last) => {
                    // Keep if significantly different position or type
                    (element.bounds.x - last.bounds.x).abs() > 10 ||
                    (element.bounds.y - last.bounds.y).abs() > 10 ||
                    element.element_type != last.element_type
                }
            };
            
            if should_keep {
                deduplicated.push(element.clone());
                last_element = Some(element);
            }
        }
        
        deduplicated
    }
    
    fn calculate_image_hash(&self, image: &DynamicImage) -> String {
        // Simple hash based on image dimensions and a few pixel samples
        let width = image.width();
        let height = image.height();
        
        format!("{}x{}", width, height) // Simplified hash
    }
}

/// Get available AI models info
pub fn get_model_info() -> Vec<ModelInfo> {
    vec![
        ModelInfo {
            name: "CLIP".to_string(),
            description: "General computer vision and image understanding".to_string(),
            size_mb: 150,
            capabilities: vec!["object_detection".to_string(), "scene_understanding".to_string()],
        },
        ModelInfo {
            name: "Florence-2".to_string(),
            description: "Advanced scene analysis and element detection".to_string(),
            size_mb: 280,
            capabilities: vec!["detailed_analysis".to_string(), "element_detection".to_string()],
        },
        ModelInfo {
            name: "TrOCR".to_string(),
            description: "Optical character recognition for text extraction".to_string(),
            size_mb: 120,
            capabilities: vec!["text_recognition".to_string(), "ocr".to_string()],
        },
    ]
}

#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub name: String,
    pub description: String,
    pub size_mb: u64,
    pub capabilities: Vec<String>,
}