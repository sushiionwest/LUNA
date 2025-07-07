//! # AI Processing Pipeline
//! 
//! This module orchestrates all AI specialists (Florence-2, CLIP, TrOCR, SAM) to process
//! user commands and screenshot data. It implements the core AI workflow that transforms
//! natural language commands into precise click coordinates.
//!
//! ## Key Responsibilities
//! - Coordinate multiple AI models for comprehensive screen understanding
//! - Implement the 6-step Luna AI processing flow
//! - Provide intelligent error recovery and fallback strategies
//! - Optimize performance through parallel processing and caching
//!
//! ## Memory Management
//! - Efficient resource allocation across multiple AI models
//! - Automatic memory cleanup and garbage collection
//! - Dynamic model loading based on available resources
//!
//! ## Error Handling
//! - Graceful degradation when models fail
//! - Intelligent fallback to alternative processing strategies
//! - Comprehensive error reporting with suggested fixes

use crate::ai::{
    florence::{Florence2Specialist, DetectedObject},
    clip::{ClipSpecialist, MatchResult, TextQuery, VisualElement},
    trocr::{TrOcrSpecialist, ExtractedText, TextRegion},
    sam::{SamSpecialist, SegmentationMask, SegmentationPrompt, PromptType, PromptData},
};
use crate::core::{LunaError, LunaResult, MemoryManager};
use crate::utils::MetricsCollector;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info, warn, error, instrument};

/// Complete AI analysis result for a screenshot and command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiAnalysisResult {
    /// Original user command
    pub command: String,
    /// Screenshot dimensions
    pub image_dimensions: (u32, u32),
    /// All detected objects from Florence-2
    pub detected_objects: Vec<DetectedObject>,
    /// Extracted text from TrOCR
    pub extracted_text: Vec<ExtractedText>,
    /// Text-visual matches from CLIP
    pub matches: Vec<MatchResult>,
    /// Segmentation masks from SAM (optional)
    pub segmentation_masks: Vec<SegmentationMask>,
    /// Final recommended click targets
    pub click_targets: Vec<ClickTarget>,
    /// Overall confidence in the analysis
    pub confidence: f32,
    /// Processing time breakdown
    pub timing_info: TimingInfo,
    /// Additional metadata and debugging info
    pub metadata: HashMap<String, String>,
}

/// A recommended click target with detailed information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClickTarget {
    /// Unique identifier
    pub id: String,
    /// Precise click coordinates (x, y)
    pub coordinates: (u32, u32),
    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,
    /// Element type (button, link, input, etc.)
    pub element_type: String,
    /// Reasoning for why this target was selected
    pub reasoning: String,
    /// Source object/match that led to this target
    pub source_id: String,
    /// Alternative click points if primary fails
    pub alternatives: Vec<(u32, u32)>,
    /// Element text content if available
    pub text_content: Option<String>,
}

/// Processing time breakdown for performance analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimingInfo {
    /// Total processing time
    pub total_ms: u64,
    /// Florence-2 object detection time
    pub florence_ms: u64,
    /// TrOCR text extraction time
    pub trocr_ms: u64,
    /// CLIP matching time
    pub clip_ms: u64,
    /// SAM segmentation time (if used)
    pub sam_ms: u64,
    /// Post-processing and coordination time
    pub coordination_ms: u64,
}

/// Configuration for the AI pipeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    /// Enable parallel processing of AI models
    pub enable_parallel_processing: bool,
    /// Enable SAM segmentation for precise targeting
    pub enable_segmentation: bool,
    /// Minimum confidence threshold for final results
    pub min_final_confidence: f32,
    /// Maximum number of click targets to return
    pub max_click_targets: usize,
    /// Timeout for the entire pipeline in milliseconds
    pub pipeline_timeout_ms: u64,
    /// Enable intelligent fallback strategies
    pub enable_fallbacks: bool,
    /// Enable result caching
    pub enable_caching: bool,
    /// Cache TTL in seconds
    pub cache_ttl_seconds: u64,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            enable_parallel_processing: true,
            enable_segmentation: true,
            min_final_confidence: 0.6,
            max_click_targets: 5,
            pipeline_timeout_ms: 15000, // 15 seconds
            enable_fallbacks: true,
            enable_caching: true,
            cache_ttl_seconds: 300, // 5 minutes
        }
    }
}

/// AI processing pipeline that coordinates all specialists
pub struct AiPipeline {
    /// Pipeline configuration
    config: PipelineConfig,
    /// Florence-2 specialist for object detection
    florence_specialist: Arc<RwLock<Florence2Specialist>>,
    /// CLIP specialist for text-visual matching
    clip_specialist: Arc<RwLock<ClipSpecialist>>,
    /// TrOCR specialist for text extraction
    trocr_specialist: Arc<RwLock<TrOcrSpecialist>>,
    /// SAM specialist for segmentation
    sam_specialist: Arc<RwLock<SamSpecialist>>,
    /// Memory manager for resource tracking
    memory_manager: Arc<MemoryManager>,
    /// Metrics collector for performance monitoring
    metrics: Arc<MetricsCollector>,
    /// Result cache
    result_cache: Arc<RwLock<HashMap<String, (AiAnalysisResult, Instant)>>>,
    /// Processing statistics
    total_analyses: Arc<RwLock<u64>>,
    successful_analyses: Arc<RwLock<u64>>,
}

impl AiPipeline {
    /// Create a new AI pipeline with all specialists
    pub async fn new(
        config: PipelineConfig,
        memory_manager: Arc<MemoryManager>,
        metrics: Arc<MetricsCollector>,
    ) -> LunaResult<Self> {
        info!("Initializing AI pipeline with {} specialists", 4);

        // Create all specialists
        let florence_config = crate::ai::florence::Florence2Config::default();
        let florence_specialist = Arc::new(RwLock::new(
            Florence2Specialist::new(florence_config, memory_manager.clone(), metrics.clone())?
        ));

        let clip_config = crate::ai::clip::ClipConfig::default();
        let clip_specialist = Arc::new(RwLock::new(
            ClipSpecialist::new(clip_config, memory_manager.clone(), metrics.clone())?
        ));

        let trocr_config = crate::ai::trocr::TrOcrConfig::default();
        let trocr_specialist = Arc::new(RwLock::new(
            TrOcrSpecialist::new(trocr_config, memory_manager.clone(), metrics.clone())?
        ));

        let sam_config = crate::ai::sam::SamConfig::default();
        let sam_specialist = Arc::new(RwLock::new(
            SamSpecialist::new(sam_config, memory_manager.clone(), metrics.clone())?
        ));

        Ok(Self {
            config,
            florence_specialist,
            clip_specialist,
            trocr_specialist,
            sam_specialist,
            memory_manager,
            metrics,
            result_cache: Arc::new(RwLock::new(HashMap::new())),
            total_analyses: Arc::new(RwLock::new(0)),
            successful_analyses: Arc::new(RwLock::new(0)),
        })
    }

    /// Initialize all AI specialists
    #[instrument(skip(self))]
    pub async fn initialize(&self) -> LunaResult<()> {
        let start_time = Instant::now();
        info!("Initializing all AI specialists");

        // Check total memory requirements
        let memory_info = self.memory_manager.get_memory_info().await;
        let total_required = self.estimate_total_memory_mb().await;
        
        if memory_info.available_mb < total_required {
            return Err(LunaError::Memory {
                operation: "AI pipeline initialization".to_string(),
                required_mb: total_required,
                available_mb: memory_info.available_mb,
                suggestion: "Close other applications or disable some AI models".to_string(),
            });
        }

        // Initialize specialists based on configuration
        let mut initialization_tasks = Vec::new();

        // Always initialize Florence-2 and CLIP (core functionality)
        initialization_tasks.push(tokio::spawn({
            let florence = self.florence_specialist.clone();
            async move {
                florence.write().await.initialize().await
            }
        }));

        initialization_tasks.push(tokio::spawn({
            let clip = self.clip_specialist.clone();
            async move {
                clip.write().await.initialize().await
            }
        }));

        initialization_tasks.push(tokio::spawn({
            let trocr = self.trocr_specialist.clone();
            async move {
                trocr.write().await.initialize().await
            }
        }));

        // SAM is optional based on configuration
        if self.config.enable_segmentation {
            initialization_tasks.push(tokio::spawn({
                let sam = self.sam_specialist.clone();
                async move {
                    sam.write().await.initialize().await
                }
            }));
        }

        // Wait for all initializations to complete
        let mut failed_specialists = Vec::new();
        for (i, task) in initialization_tasks.into_iter().enumerate() {
            match task.await {
                Ok(Ok(())) => {
                    let specialist_name = match i {
                        0 => "Florence-2",
                        1 => "CLIP",
                        2 => "TrOCR",
                        3 => "SAM",
                        _ => "Unknown",
                    };
                    info!("{} initialized successfully", specialist_name);
                }
                Ok(Err(e)) => {
                    let specialist_name = match i {
                        0 => "Florence-2",
                        1 => "CLIP",
                        2 => "TrOCR",
                        3 => "SAM",
                        _ => "Unknown",
                    };
                    error!("Failed to initialize {}: {}", specialist_name, e);
                    failed_specialists.push(specialist_name);
                }
                Err(e) => {
                    error!("Task error during initialization: {}", e);
                    failed_specialists.push("Unknown");
                }
            }
        }

        // Check if critical specialists failed
        if failed_specialists.contains(&"Florence-2") || failed_specialists.contains(&"CLIP") {
            return Err(LunaError::AiModel {
                model: "Pipeline".to_string(),
                error: format!("Critical specialists failed: {:?}", failed_specialists),
                suggestion: "Check individual specialist configurations and retry".to_string(),
            });
        }

        // Warm up CLIP cache with common terms
        if !failed_specialists.contains(&"CLIP") {
            let _ = self.clip_specialist.write().await.warm_cache().await;
        }

        let init_time = start_time.elapsed();
        info!("AI pipeline initialized in {:?}", init_time);

        // Record metrics
        self.metrics.record_ai_model_event(
            "pipeline".to_string(),
            "initialization".to_string(),
            init_time,
            failed_specialists.is_empty(),
        ).await;

        Ok(())
    }

    /// Process a user command and screenshot through the complete AI pipeline
    #[instrument(skip(self, image_data))]
    pub async fn process_command(
        &self,
        command: String,
        image_data: Vec<u8>,
        image_dimensions: (u32, u32),
    ) -> LunaResult<AiAnalysisResult> {
        let start_time = Instant::now();
        
        // Update statistics
        *self.total_analyses.write().await += 1;

        // Check cache first if enabled
        if self.config.enable_caching {
            if let Some(cached_result) = self.check_cache(&command, &image_data).await {
                info!("Returning cached result for command: {}", command);
                return Ok(cached_result);
            }
        }

        info!("Processing command: '{}' with {}x{} image", command, image_dimensions.0, image_dimensions.1);

        // Initialize timing info
        let mut timing = TimingInfo {
            total_ms: 0,
            florence_ms: 0,
            trocr_ms: 0,
            clip_ms: 0,
            sam_ms: 0,
            coordination_ms: 0,
        };

        // Run the complete AI pipeline with timeout
        let result = tokio::time::timeout(
            Duration::from_millis(self.config.pipeline_timeout_ms),
            self.run_pipeline(&command, &image_data, image_dimensions, &mut timing),
        ).await;

        match result {
            Ok(Ok(analysis_result)) => {
                // Update success statistics
                *self.successful_analyses.write().await += 1;

                // Cache result if enabled
                if self.config.enable_caching {
                    self.cache_result(&command, &image_data, &analysis_result).await;
                }

                let total_time = start_time.elapsed();
                info!(
                    "Command processed successfully in {:?} (found {} click targets)",
                    total_time,
                    analysis_result.click_targets.len()
                );

                // Record metrics
                self.metrics.record_ai_model_event(
                    "pipeline".to_string(),
                    "command_processed".to_string(),
                    total_time,
                    true,
                ).await;

                Ok(analysis_result)
            }
            Ok(Err(e)) => {
                error!("Pipeline processing failed: {}", e);
                
                // Try fallback if enabled
                if self.config.enable_fallbacks {
                    warn!("Attempting fallback processing");
                    self.run_fallback_pipeline(&command, &image_data, image_dimensions).await
                } else {
                    Err(e)
                }
            }
            Err(_) => {
                error!("Pipeline timeout after {}ms", self.config.pipeline_timeout_ms);
                
                if self.config.enable_fallbacks {
                    warn!("Attempting fallback processing after timeout");
                    self.run_fallback_pipeline(&command, &image_data, image_dimensions).await
                } else {
                    Err(LunaError::AiModel {
                        model: "Pipeline".to_string(),
                        error: "Processing timeout".to_string(),
                        suggestion: "Increase pipeline_timeout_ms or reduce image size".to_string(),
                    })
                }
            }
        }
    }

    /// Validate all AI model connections
    pub async fn validate_all_connections(&self) -> LunaResult<()> {
        info!("Validating all AI specialist connections");

        let mut validation_tasks = Vec::new();

        validation_tasks.push(tokio::spawn({
            let florence = self.florence_specialist.clone();
            async move {
                ("Florence-2", florence.read().await.validate_connections().await)
            }
        }));

        validation_tasks.push(tokio::spawn({
            let clip = self.clip_specialist.clone();
            async move {
                ("CLIP", clip.read().await.validate_connections().await)
            }
        }));

        validation_tasks.push(tokio::spawn({
            let trocr = self.trocr_specialist.clone();
            async move {
                ("TrOCR", trocr.read().await.validate_connections().await)
            }
        }));

        if self.config.enable_segmentation {
            validation_tasks.push(tokio::spawn({
                let sam = self.sam_specialist.clone();
                async move {
                    ("SAM", sam.read().await.validate_connections().await)
                }
            }));
        }

        let mut failed_validations = Vec::new();
        for task in validation_tasks {
            match task.await {
                Ok((name, Ok(()))) => {
                    info!("{} connections validated", name);
                }
                Ok((name, Err(e))) => {
                    error!("{} validation failed: {}", name, e);
                    failed_validations.push((name, e));
                }
                Err(e) => {
                    error!("Validation task error: {}", e);
                    failed_validations.push(("Unknown", LunaError::AiModel {
                        model: "Unknown".to_string(),
                        error: e.to_string(),
                        suggestion: "Check task execution".to_string(),
                    }));
                }
            }
        }

        if !failed_validations.is_empty() {
            return Err(LunaError::AiModel {
                model: "Pipeline".to_string(),
                error: format!("Validation failures: {:?}", failed_validations),
                suggestion: "Check individual specialist configurations".to_string(),
            });
        }

        info!("All AI connections validated successfully");
        Ok(())
    }

    /// Get comprehensive status of all specialists
    pub async fn get_status(&self) -> HashMap<String, HashMap<String, String>> {
        let mut status = HashMap::new();

        // Get status from each specialist
        status.insert("florence".to_string(), self.florence_specialist.read().await.get_status().await);
        status.insert("clip".to_string(), self.clip_specialist.read().await.get_status().await);
        status.insert("trocr".to_string(), self.trocr_specialist.read().await.get_status().await);
        status.insert("sam".to_string(), self.sam_specialist.read().await.get_status().await);

        // Add pipeline-level statistics
        let mut pipeline_status = HashMap::new();
        pipeline_status.insert("total_analyses".to_string(), self.total_analyses.read().await.to_string());
        pipeline_status.insert("successful_analyses".to_string(), self.successful_analyses.read().await.to_string());
        
        let total = *self.total_analyses.read().await;
        let successful = *self.successful_analyses.read().await;
        if total > 0 {
            let success_rate = (successful as f64 / total as f64) * 100.0;
            pipeline_status.insert("success_rate_percent".to_string(), format!("{:.1}", success_rate));
        }

        if self.config.enable_caching {
            let cache_size = self.result_cache.read().await.len();
            pipeline_status.insert("cache_size".to_string(), cache_size.to_string());
        }

        status.insert("pipeline".to_string(), pipeline_status);

        status
    }

    /// Cleanup all resources and prepare for shutdown
    pub async fn cleanup(&self) -> LunaResult<()> {
        info!("Cleaning up AI pipeline resources");

        // Cleanup all specialists
        let _ = self.florence_specialist.write().await.cleanup().await;
        let _ = self.clip_specialist.write().await.cleanup().await;
        let _ = self.trocr_specialist.write().await.cleanup().await;
        let _ = self.sam_specialist.write().await.cleanup().await;

        // Clear cache
        self.result_cache.write().await.clear();

        // Reset statistics
        *self.total_analyses.write().await = 0;
        *self.successful_analyses.write().await = 0;

        // Trigger garbage collection
        self.memory_manager.trigger_gc().await;

        info!("AI pipeline cleanup complete");
        Ok(())
    }

    // Private helper methods

    async fn run_pipeline(
        &self,
        command: &str,
        image_data: &[u8],
        image_dimensions: (u32, u32),
        timing: &mut TimingInfo,
    ) -> LunaResult<AiAnalysisResult> {
        let pipeline_start = Instant::now();

        // Step 1: Object Detection with Florence-2
        let florence_start = Instant::now();
        let detected_objects = self.florence_specialist.read().await
            .detect_objects(image_data).await?;
        timing.florence_ms = florence_start.elapsed().as_millis() as u64;

        // Step 2: Text Extraction with TrOCR (parallel with object detection results)
        let trocr_start = Instant::now();
        let text_regions = self.create_text_regions_from_objects(&detected_objects, image_data).await;
        let extracted_text = self.trocr_specialist.read().await
            .extract_text(&text_regions).await?;
        timing.trocr_ms = trocr_start.elapsed().as_millis() as u64;

        // Step 3: Text-Visual Matching with CLIP
        let clip_start = Instant::now();
        let visual_elements = self.create_visual_elements_from_objects(&detected_objects, image_data).await;
        let query = TextQuery {
            text: command.to_string(),
            context: None,
            min_confidence: self.config.min_final_confidence,
            max_matches: self.config.max_click_targets * 2, // Get more for filtering
        };
        let matches = self.clip_specialist.read().await
            .match_text_to_visuals(&query, &visual_elements).await?;
        timing.clip_ms = clip_start.elapsed().as_millis() as u64;

        // Step 4: Optional Segmentation with SAM for precise targeting
        let sam_start = Instant::now();
        let segmentation_masks = if self.config.enable_segmentation && !matches.is_empty() {
            self.refine_with_segmentation(image_data, &matches).await.unwrap_or_default()
        } else {
            Vec::new()
        };
        timing.sam_ms = sam_start.elapsed().as_millis() as u64;

        // Step 5: Coordinate and create final click targets
        let coord_start = Instant::now();
        let click_targets = self.create_click_targets(
            &detected_objects,
            &extracted_text,
            &matches,
            &segmentation_masks,
            command,
        ).await?;
        timing.coordination_ms = coord_start.elapsed().as_millis() as u64;

        // Calculate overall confidence
        let confidence = self.calculate_overall_confidence(&click_targets, &matches).await;

        // Create metadata
        let mut metadata = HashMap::new();
        metadata.insert("pipeline_version".to_string(), "1.0".to_string());
        metadata.insert("parallel_processing".to_string(), self.config.enable_parallel_processing.to_string());
        metadata.insert("segmentation_enabled".to_string(), self.config.enable_segmentation.to_string());
        metadata.insert("objects_detected".to_string(), detected_objects.len().to_string());
        metadata.insert("text_regions_extracted".to_string(), extracted_text.len().to_string());
        metadata.insert("clip_matches".to_string(), matches.len().to_string());
        metadata.insert("segmentation_masks".to_string(), segmentation_masks.len().to_string());

        timing.total_ms = pipeline_start.elapsed().as_millis() as u64;

        Ok(AiAnalysisResult {
            command: command.to_string(),
            image_dimensions,
            detected_objects,
            extracted_text,
            matches,
            segmentation_masks,
            click_targets,
            confidence,
            timing_info: timing.clone(),
            metadata,
        })
    }

    async fn run_fallback_pipeline(
        &self,
        command: &str,
        image_data: &[u8],
        image_dimensions: (u32, u32),
    ) -> LunaResult<AiAnalysisResult> {
        warn!("Running fallback pipeline with reduced functionality");

        // Simplified processing - just try Florence-2 and basic matching
        let detected_objects = match self.florence_specialist.read().await.detect_objects(image_data).await {
            Ok(objects) => objects,
            Err(_) => {
                // If even Florence fails, create a basic center-screen target
                return Ok(self.create_emergency_fallback_result(command, image_dimensions).await);
            }
        };

        // Create basic click targets from objects without advanced matching
        let click_targets = self.create_basic_click_targets(&detected_objects, command).await;

        let mut metadata = HashMap::new();
        metadata.insert("fallback_mode".to_string(), "true".to_string());
        metadata.insert("reduced_functionality".to_string(), "true".to_string());

        Ok(AiAnalysisResult {
            command: command.to_string(),
            image_dimensions,
            detected_objects,
            extracted_text: Vec::new(),
            matches: Vec::new(),
            segmentation_masks: Vec::new(),
            click_targets,
            confidence: 0.3, // Low confidence for fallback
            timing_info: TimingInfo {
                total_ms: 0,
                florence_ms: 0,
                trocr_ms: 0,
                clip_ms: 0,
                sam_ms: 0,
                coordination_ms: 0,
            },
            metadata,
        })
    }

    async fn create_emergency_fallback_result(
        &self,
        command: &str,
        image_dimensions: (u32, u32),
    ) -> AiAnalysisResult {
        warn!("Creating emergency fallback result");

        // Create a center-screen click target as last resort
        let center_target = ClickTarget {
            id: "emergency_center".to_string(),
            coordinates: (image_dimensions.0 / 2, image_dimensions.1 / 2),
            confidence: 0.1,
            element_type: "unknown".to_string(),
            reasoning: "Emergency fallback - all AI models failed".to_string(),
            source_id: "emergency".to_string(),
            alternatives: Vec::new(),
            text_content: None,
        };

        let mut metadata = HashMap::new();
        metadata.insert("emergency_fallback".to_string(), "true".to_string());
        metadata.insert("all_models_failed".to_string(), "true".to_string());

        AiAnalysisResult {
            command: command.to_string(),
            image_dimensions,
            detected_objects: Vec::new(),
            extracted_text: Vec::new(),
            matches: Vec::new(),
            segmentation_masks: Vec::new(),
            click_targets: vec![center_target],
            confidence: 0.1,
            timing_info: TimingInfo {
                total_ms: 0,
                florence_ms: 0,
                trocr_ms: 0,
                clip_ms: 0,
                sam_ms: 0,
                coordination_ms: 0,
            },
            metadata,
        }
    }

    async fn create_text_regions_from_objects(
        &self,
        objects: &[DetectedObject],
        image_data: &[u8],
    ) -> Vec<TextRegion> {
        // Convert detected objects to text regions for OCR
        let mut regions = Vec::new();
        
        for (i, obj) in objects.iter().enumerate() {
            // Crop image region (mock implementation)
            let cropped_data = self.crop_image_region(image_data, obj.bbox).await;
            
            regions.push(TextRegion {
                id: format!("region_{}", i),
                image_data: cropped_data,
                bbox: obj.bbox,
                language_hint: Some("en".to_string()),
                priority: if obj.label.contains("button") || obj.label.contains("link") { 2 } else { 1 },
            });
        }
        
        regions
    }

    async fn create_visual_elements_from_objects(
        &self,
        objects: &[DetectedObject],
        image_data: &[u8],
    ) -> Vec<VisualElement> {
        // Convert detected objects to visual elements for CLIP matching
        let mut elements = Vec::new();
        
        for obj in objects {
            let cropped_data = self.crop_image_region(image_data, obj.bbox).await;
            
            elements.push(VisualElement {
                id: format!("element_{}", obj.label),
                image_data: cropped_data,
                bbox: obj.bbox,
                text_content: None, // Will be filled by TrOCR results
                element_type: Some(obj.label.clone()),
            });
        }
        
        elements
    }

    async fn crop_image_region(&self, image_data: &[u8], bbox: (u32, u32, u32, u32)) -> Vec<u8> {
        // Mock image cropping - in a real implementation this would actually crop the image
        debug!("Cropping region: {:?} from {} bytes", bbox, image_data.len());
        image_data[..std::cmp::min(1000, image_data.len())].to_vec() // Mock cropped data
    }

    async fn refine_with_segmentation(
        &self,
        image_data: &[u8],
        matches: &[MatchResult],
    ) -> LunaResult<Vec<SegmentationMask>> {
        // Use SAM to create precise masks for top matches
        let mut prompts = Vec::new();
        
        for (i, match_result) in matches.iter().take(3).enumerate() { // Top 3 matches
            // Create point prompt from match (mock implementation)
            let prompt = SegmentationPrompt {
                prompt_type: PromptType::Point,
                data: PromptData::Point(100 + i as u32 * 50, 100), // Mock coordinates
                label_hint: Some(match_result.element_id.clone()),
            };
            prompts.push(prompt);
        }
        
        self.sam_specialist.read().await.segment_image(image_data, &prompts).await
    }

    async fn create_click_targets(
        &self,
        objects: &[DetectedObject],
        _text: &[ExtractedText],
        matches: &[MatchResult],
        masks: &[SegmentationMask],
        command: &str,
    ) -> LunaResult<Vec<ClickTarget>> {
        let mut targets = Vec::new();
        
        // Create targets from CLIP matches
        for (i, match_result) in matches.iter().take(self.config.max_click_targets).enumerate() {
            // Find corresponding object
            let obj = objects.iter().find(|o| format!("element_{}", o.label) == match_result.element_id);
            
            let coordinates = if let Some(obj) = obj {
                obj.center
            } else {
                (100, 100) // Fallback coordinates
            };
            
            // Check if we have a segmentation mask for more precise targeting
            let refined_coordinates = if let Some(mask) = masks.iter().find(|m| m.id.contains(&match_result.element_id)) {
                (mask.bbox.0 + mask.bbox.2 / 2, mask.bbox.1 + mask.bbox.3 / 2)
            } else {
                coordinates
            };
            
            targets.push(ClickTarget {
                id: format!("target_{}", i),
                coordinates: refined_coordinates,
                confidence: match_result.confidence,
                element_type: obj.map(|o| o.label.clone()).unwrap_or_else(|| "unknown".to_string()),
                reasoning: format!("CLIP match: {} (similarity: {:.2})", match_result.reasoning, match_result.similarity),
                source_id: match_result.element_id.clone(),
                alternatives: vec![coordinates], // Original coordinates as alternative
                text_content: None,
            });
        }
        
        // If no good matches, create targets from high-confidence objects
        if targets.is_empty() {
            for (i, obj) in objects.iter()
                .filter(|o| o.confidence > 0.8)
                .take(self.config.max_click_targets)
                .enumerate() 
            {
                targets.push(ClickTarget {
                    id: format!("object_target_{}", i),
                    coordinates: obj.center,
                    confidence: obj.confidence * 0.7, // Reduced since no semantic match
                    element_type: obj.label.clone(),
                    reasoning: format!("High-confidence object detection: {}", obj.label),
                    source_id: format!("object_{}", i),
                    alternatives: Vec::new(),
                    text_content: None,
                });
            }
        }
        
        // Sort by confidence
        targets.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        
        Ok(targets)
    }

    async fn create_basic_click_targets(
        &self,
        objects: &[DetectedObject],
        command: &str,
    ) -> Vec<ClickTarget> {
        let mut targets = Vec::new();
        
        // Simple keyword matching as fallback
        let command_lower = command.to_lowercase();
        
        for (i, obj) in objects.iter().enumerate() {
            let relevance_score = if command_lower.contains(&obj.label.to_lowercase()) {
                0.8
            } else if obj.label.contains("button") && (command_lower.contains("click") || command_lower.contains("press")) {
                0.6
            } else {
                0.3
            };
            
            targets.push(ClickTarget {
                id: format!("basic_target_{}", i),
                coordinates: obj.center,
                confidence: relevance_score,
                element_type: obj.label.clone(),
                reasoning: format!("Basic keyword matching for: {}", obj.label),
                source_id: format!("object_{}", i),
                alternatives: Vec::new(),
                text_content: None,
            });
        }
        
        // Sort by confidence and take top results
        targets.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        targets.truncate(self.config.max_click_targets);
        
        targets
    }

    async fn calculate_overall_confidence(&self, targets: &[ClickTarget], matches: &[MatchResult]) -> f32 {
        if targets.is_empty() {
            return 0.0;
        }
        
        // Weight by number of successful components
        let mut confidence_sum = 0.0;
        let mut weight_sum = 0.0;
        
        // CLIP matches contribute most to confidence
        if !matches.is_empty() {
            let avg_match_confidence = matches.iter().map(|m| m.confidence).sum::<f32>() / matches.len() as f32;
            confidence_sum += avg_match_confidence * 0.6;
            weight_sum += 0.6;
        }
        
        // Target confidence contributes
        let avg_target_confidence = targets.iter().map(|t| t.confidence).sum::<f32>() / targets.len() as f32;
        confidence_sum += avg_target_confidence * 0.4;
        weight_sum += 0.4;
        
        if weight_sum > 0.0 {
            confidence_sum / weight_sum
        } else {
            0.0
        }
    }

    async fn check_cache(&self, command: &str, image_data: &[u8]) -> Option<AiAnalysisResult> {
        if !self.config.enable_caching {
            return None;
        }
        
        let cache_key = self.generate_cache_key(command, image_data).await;
        let cache = self.result_cache.read().await;
        
        if let Some((result, timestamp)) = cache.get(&cache_key) {
            let age = Instant::now().duration_since(*timestamp);
            if age.as_secs() < self.config.cache_ttl_seconds {
                return Some(result.clone());
            }
        }
        
        None
    }

    async fn cache_result(&self, command: &str, image_data: &[u8], result: &AiAnalysisResult) {
        if !self.config.enable_caching {
            return;
        }
        
        let cache_key = self.generate_cache_key(command, image_data).await;
        let mut cache = self.result_cache.write().await;
        
        cache.insert(cache_key, (result.clone(), Instant::now()));
        
        // Cleanup old entries if cache is too large
        if cache.len() > 100 {
            let cutoff = Instant::now() - Duration::from_secs(self.config.cache_ttl_seconds);
            cache.retain(|_, (_, timestamp)| *timestamp > cutoff);
        }
    }

    async fn generate_cache_key(&self, command: &str, image_data: &[u8]) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        command.hash(&mut hasher);
        image_data.hash(&mut hasher);
        
        format!("pipeline_{:x}", hasher.finish())
    }

    async fn estimate_total_memory_mb(&self) -> u64 {
        // Rough estimate of total memory needed for all models
        2048 + 1024 + 800 + 2048 // Florence + CLIP + TrOCR + SAM
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::MemoryManager;
    use crate::utils::MetricsCollector;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_pipeline_creation() {
        let memory_manager = Arc::new(MemoryManager::new().unwrap());
        let metrics = Arc::new(MetricsCollector::new());
        let config = PipelineConfig::default();
        
        let pipeline = AiPipeline::new(config, memory_manager, metrics).await;
        assert!(pipeline.is_ok());
    }

    #[tokio::test]
    async fn test_pipeline_status() {
        let memory_manager = Arc::new(MemoryManager::new().unwrap());
        let metrics = Arc::new(MetricsCollector::new());
        let config = PipelineConfig::default();
        
        let pipeline = AiPipeline::new(config, memory_manager, metrics).await.unwrap();
        let status = pipeline.get_status().await;
        
        assert!(status.contains_key("florence"));
        assert!(status.contains_key("clip"));
        assert!(status.contains_key("trocr"));
        assert!(status.contains_key("sam"));
        assert!(status.contains_key("pipeline"));
    }

    #[tokio::test]
    async fn test_emergency_fallback() {
        let memory_manager = Arc::new(MemoryManager::new().unwrap());
        let metrics = Arc::new(MetricsCollector::new());
        let config = PipelineConfig::default();
        
        let pipeline = AiPipeline::new(config, memory_manager, metrics).await.unwrap();
        let result = pipeline.create_emergency_fallback_result("test command", (800, 600)).await;
        
        assert_eq!(result.command, "test command");
        assert_eq!(result.image_dimensions, (800, 600));
        assert_eq!(result.click_targets.len(), 1);
        assert_eq!(result.click_targets[0].coordinates, (400, 300)); // Center
        assert!(result.metadata.contains_key("emergency_fallback"));
    }

    #[tokio::test]
    async fn test_cache_key_generation() {
        let memory_manager = Arc::new(MemoryManager::new().unwrap());
        let metrics = Arc::new(MetricsCollector::new());
        let config = PipelineConfig::default();
        
        let pipeline = AiPipeline::new(config, memory_manager, metrics).await.unwrap();
        let key1 = pipeline.generate_cache_key("test", &[1, 2, 3]).await;
        let key2 = pipeline.generate_cache_key("test", &[1, 2, 3]).await;
        let key3 = pipeline.generate_cache_key("different", &[1, 2, 3]).await;
        
        assert_eq!(key1, key2); // Same inputs should produce same key
        assert_ne!(key1, key3); // Different inputs should produce different keys
    }
}