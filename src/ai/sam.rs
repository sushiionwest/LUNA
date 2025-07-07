//! # SAM (Segment Anything Model) Specialist
//! 
//! This module implements the SAM (Segment Anything Model) for precise image segmentation
//! of UI elements. SAM excels at creating pixel-perfect masks for buttons, windows,
//! text regions, and other interface components.
//!
//! ## Key Capabilities
//! - Precise pixel-level segmentation of UI elements
//! - Interactive segmentation with point and box prompts
//! - Automatic mask generation for entire screenshots
//! - High-quality masks for click target refinement
//!
//! ## Memory Management
//! - Efficient mask caching for repeated elements
//! - Batch processing for multiple segmentation requests
//! - Automatic cleanup of large segmentation data
//!
//! ## Error Handling
//! - Input validation for images and prompts
//! - Graceful handling of segmentation failures
//! - Memory-aware processing with automatic fallbacks

use crate::core::{LunaError, LunaResult, MemoryManager};
use crate::utils::MetricsCollector;
use candle_core::{Device, Tensor, DType};
use candle_nn::VarBuilder;
use candle_transformers::models::sam::{SamModel, SamConfig};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info, warn, error, instrument};

/// Segmentation mask result with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentationMask {
    /// Unique identifier for the mask
    pub id: String,
    /// Binary mask data (0 = background, 255 = foreground)
    pub mask_data: Vec<u8>,
    /// Mask dimensions (width, height)
    pub dimensions: (u32, u32),
    /// Confidence score for the segmentation
    pub confidence: f32,
    /// Bounding box of the segmented area
    pub bbox: (u32, u32, u32, u32),
    /// Area of the segmented region in pixels
    pub area_pixels: u32,
    /// Additional metadata about the segmentation
    pub metadata: HashMap<String, String>,
}

/// Segmentation prompt for guided segmentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentationPrompt {
    /// Prompt type
    pub prompt_type: PromptType,
    /// Coordinates or region data
    pub data: PromptData,
    /// Expected object label (optional hint)
    pub label_hint: Option<String>,
}

/// Types of segmentation prompts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PromptType {
    /// Single point prompt
    Point,
    /// Multiple points prompt
    Points,
    /// Bounding box prompt
    Box,
    /// Everything (automatic segmentation)
    Everything,
}

/// Prompt data for different types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PromptData {
    /// Single point (x, y)
    Point(u32, u32),
    /// Multiple points [(x, y), ...]
    Points(Vec<(u32, u32)>),
    /// Bounding box (x, y, width, height)
    Box(u32, u32, u32, u32),
    /// No specific data for everything mode
    None,
}

/// Configuration for SAM model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamConfig {
    /// Model path or identifier
    pub model_path: String,
    /// Device to run inference on (cuda, cpu, auto)
    pub device: String,
    /// Maximum inference time in milliseconds
    pub max_inference_time_ms: u64,
    /// Minimum mask confidence threshold
    pub min_confidence: f32,
    /// Minimum mask area in pixels
    pub min_area_pixels: u32,
    /// Maximum number of masks to return
    pub max_masks: usize,
    /// Enable mask post-processing
    pub enable_postprocessing: bool,
    /// Enable mask caching
    pub enable_caching: bool,
    /// Maximum cache size (number of masks)
    pub max_cache_size: usize,
}

impl Default for SamConfig {
    fn default() -> Self {
        Self {
            model_path: "facebook/sam-vit-base".to_string(),
            device: "auto".to_string(),
            max_inference_time_ms: 5000,
            min_confidence: 0.7,
            min_area_pixels: 100,
            max_masks: 10,
            enable_postprocessing: true,
            enable_caching: true,
            max_cache_size: 100,
        }
    }
}

/// Cached mask for efficient reuse
#[derive(Debug, Clone)]
struct CachedMask {
    /// The segmentation mask
    mask: SegmentationMask,
    /// When this mask was created
    created_at: Instant,
    /// How many times this mask has been accessed
    access_count: u64,
}

/// SAM specialist for image segmentation and mask generation
pub struct SamSpecialist {
    /// Model configuration
    config: SamConfig,
    /// Loaded SAM model (None if not initialized)
    model: Option<Arc<SamModel>>,
    /// Device for inference
    device: Device,
    /// Memory manager for tracking resource usage
    memory_manager: Arc<MemoryManager>,
    /// Metrics collector for performance monitoring
    metrics: Arc<MetricsCollector>,
    /// Model initialization status
    is_initialized: Arc<RwLock<bool>>,
    /// Mask cache for efficient reuse
    mask_cache: Arc<RwLock<HashMap<String, CachedMask>>>,
    /// Processing statistics
    total_segmentations: Arc<RwLock<u64>>,
    total_masks_generated: Arc<RwLock<u64>>,
    /// Last inference timestamp for rate limiting
    last_inference: Arc<RwLock<Option<Instant>>>,
}

impl SamSpecialist {
    /// Create a new SAM specialist with the given configuration
    pub fn new(
        config: SamConfig,
        memory_manager: Arc<MemoryManager>,
        metrics: Arc<MetricsCollector>,
    ) -> LunaResult<Self> {
        // Validate configuration
        if config.min_confidence < 0.0 || config.min_confidence > 1.0 {
            return Err(LunaError::AiModel {
                model: "SAM".to_string(),
                error: "min_confidence must be between 0.0 and 1.0".to_string(),
                suggestion: "Set min_confidence to a value between 0.0 and 1.0".to_string(),
            });
        }

        if config.max_masks == 0 {
            return Err(LunaError::AiModel {
                model: "SAM".to_string(),
                error: "max_masks must be greater than 0".to_string(),
                suggestion: "Set max_masks to a positive integer".to_string(),
            });
        }

        if config.max_cache_size == 0 {
            return Err(LunaError::AiModel {
                model: "SAM".to_string(),
                error: "max_cache_size must be greater than 0".to_string(),
                suggestion: "Set max_cache_size to a positive integer".to_string(),
            });
        }

        // Determine device
        let device = match config.device.as_str() {
            "cuda" => {
                if candle_core::utils::cuda_is_available() {
                    Device::new_cuda(0).map_err(|e| LunaError::AiModel {
                        model: "SAM".to_string(),
                        error: format!("Failed to initialize CUDA: {}", e),
                        suggestion: "Check CUDA installation or use 'cpu' device".to_string(),
                    })?
                } else {
                    return Err(LunaError::AiModel {
                        model: "SAM".to_string(),
                        error: "CUDA requested but not available".to_string(),
                        suggestion: "Install CUDA drivers or use 'auto' device".to_string(),
                    });
                }
            }
            "cpu" => Device::Cpu,
            "auto" => {
                if candle_core::utils::cuda_is_available() {
                    Device::new_cuda(0).unwrap_or(Device::Cpu)
                } else {
                    Device::Cpu
                }
            }
            _ => {
                return Err(LunaError::AiModel {
                    model: "SAM".to_string(),
                    error: format!("Unknown device: {}", config.device),
                    suggestion: "Use 'cuda', 'cpu', or 'auto'".to_string(),
                });
            }
        };

        info!("SAM specialist initialized with device: {:?}", device);

        Ok(Self {
            config,
            model: None,
            device,
            memory_manager,
            metrics,
            is_initialized: Arc::new(RwLock::new(false)),
            mask_cache: Arc::new(RwLock::new(HashMap::new())),
            total_segmentations: Arc::new(RwLock::new(0)),
            total_masks_generated: Arc::new(RwLock::new(0)),
            last_inference: Arc::new(RwLock::new(None)),
        })
    }

    /// Initialize the SAM model
    #[instrument(skip(self))]
    pub async fn initialize(&mut self) -> LunaResult<()> {
        let start_time = Instant::now();
        
        // Check if already initialized
        if *self.is_initialized.read().await {
            debug!("SAM already initialized");
            return Ok(());
        }

        // Check memory availability
        let memory_info = self.memory_manager.get_memory_info().await;
        let required_memory = self.estimate_model_memory_mb();
        
        if memory_info.available_mb < required_memory {
            return Err(LunaError::Memory {
                operation: "SAM model loading".to_string(),
                required_mb: required_memory,
                available_mb: memory_info.available_mb,
                suggestion: "Close other applications or disable caching".to_string(),
            });
        }

        info!("Loading SAM model from: {}", self.config.model_path);

        // Load model configuration
        let model_config = self.load_model_config().await?;
        
        // Create variable builder for model weights
        let var_builder = self.create_var_builder().await?;
        
        // Initialize model
        let model = SamModel::load(&var_builder, &model_config)
            .map_err(|e| LunaError::AiModel {
                model: "SAM".to_string(),
                error: format!("Failed to load model: {}", e),
                suggestion: "Check model path and ensure model files are accessible".to_string(),
            })?;

        // Store model and mark as initialized
        self.model = Some(Arc::new(model));
        *self.is_initialized.write().await = true;

        let load_time = start_time.elapsed();
        info!("SAM model loaded successfully in {:?}", load_time);

        // Record metrics
        self.metrics.record_ai_model_event(
            "sam".to_string(),
            "model_loaded".to_string(),
            load_time,
            true,
        ).await;

        Ok(())
    }

    /// Generate segmentation masks for the given image and prompts
    #[instrument(skip(self, image_data, prompts))]
    pub async fn segment_image(
        &self,
        image_data: &[u8],
        prompts: &[SegmentationPrompt],
    ) -> LunaResult<Vec<SegmentationMask>> {
        let start_time = Instant::now();

        // Ensure model is initialized
        if !*self.is_initialized.read().await {
            return Err(LunaError::AiModel {
                model: "SAM".to_string(),
                error: "Model not initialized".to_string(),
                suggestion: "Call initialize() before using the model".to_string(),
            });
        }

        // Validate inputs
        if image_data.is_empty() {
            return Err(LunaError::AiModel {
                model: "SAM".to_string(),
                error: "Empty image data".to_string(),
                suggestion: "Provide valid image data".to_string(),
            });
        }

        if prompts.is_empty() {
            debug!("No prompts provided, using automatic segmentation");
            return self.segment_everything(image_data).await;
        }

        // Rate limiting check
        self.check_rate_limit().await?;

        // Check memory before processing
        let memory_info = self.memory_manager.get_memory_info().await;
        let required_memory = self.estimate_inference_memory_mb(image_data.len());
        
        if memory_info.available_mb < required_memory {
            // Trigger garbage collection
            self.memory_manager.trigger_gc().await;
            
            // Check again
            let memory_info = self.memory_manager.get_memory_info().await;
            if memory_info.available_mb < required_memory {
                return Err(LunaError::Memory {
                    operation: "SAM segmentation".to_string(),
                    required_mb: required_memory,
                    available_mb: memory_info.available_mb,
                    suggestion: "Reduce image size or number of prompts".to_string(),
                });
            }
        }

        // Process prompts and generate masks
        let mut all_masks = Vec::new();
        for prompt in prompts {
            match self.process_single_prompt(image_data, prompt).await {
                Ok(masks) => all_masks.extend(masks),
                Err(e) => {
                    warn!("Failed to process prompt: {}", e);
                    continue;
                }
            }
        }

        // Post-process masks if enabled
        if self.config.enable_postprocessing {
            all_masks = self.postprocess_masks(all_masks).await?;
        }

        // Filter and limit results
        let filtered_masks: Vec<SegmentationMask> = all_masks
            .into_iter()
            .filter(|mask| mask.confidence >= self.config.min_confidence)
            .filter(|mask| mask.area_pixels >= self.config.min_area_pixels)
            .take(self.config.max_masks)
            .collect();

        // Update statistics
        *self.total_segmentations.write().await += 1;
        *self.total_masks_generated.write().await += filtered_masks.len() as u64;

        // Update rate limiting
        *self.last_inference.write().await = Some(Instant::now());

        let segmentation_time = start_time.elapsed();
        info!(
            "SAM generated {} masks from {} prompts in {:?}",
            filtered_masks.len(),
            prompts.len(),
            segmentation_time
        );

        // Record metrics
        self.metrics.record_ai_model_event(
            "sam".to_string(),
            "segmentation".to_string(),
            segmentation_time,
            true,
        ).await;

        self.metrics.record_custom_metric(
            "sam_masks_generated".to_string(),
            filtered_masks.len() as f64,
        ).await;

        Ok(filtered_masks)
    }

    /// Segment everything in the image automatically
    #[instrument(skip(self, image_data))]
    pub async fn segment_everything(&self, image_data: &[u8]) -> LunaResult<Vec<SegmentationMask>> {
        let prompt = SegmentationPrompt {
            prompt_type: PromptType::Everything,
            data: PromptData::None,
            label_hint: None,
        };

        self.segment_image(image_data, &[prompt]).await
    }

    /// Segment a specific region using a bounding box
    #[instrument(skip(self, image_data))]
    pub async fn segment_region(
        &self,
        image_data: &[u8],
        bbox: (u32, u32, u32, u32),
        label_hint: Option<String>,
    ) -> LunaResult<Vec<SegmentationMask>> {
        let prompt = SegmentationPrompt {
            prompt_type: PromptType::Box,
            data: PromptData::Box(bbox.0, bbox.1, bbox.2, bbox.3),
            label_hint,
        };

        self.segment_image(image_data, &[prompt]).await
    }

    /// Segment around a specific point
    #[instrument(skip(self, image_data))]
    pub async fn segment_point(
        &self,
        image_data: &[u8],
        point: (u32, u32),
        label_hint: Option<String>,
    ) -> LunaResult<Vec<SegmentationMask>> {
        let prompt = SegmentationPrompt {
            prompt_type: PromptType::Point,
            data: PromptData::Point(point.0, point.1),
            label_hint,
        };

        self.segment_image(image_data, &[prompt]).await
    }

    /// Refine click targets using segmentation masks
    pub async fn refine_click_targets(
        &self,
        image_data: &[u8],
        rough_targets: &[(u32, u32)], // (x, y) coordinates
    ) -> LunaResult<Vec<(u32, u32)>> {
        let mut refined_targets = Vec::new();

        for &(x, y) in rough_targets {
            // Segment around the point
            let masks = self.segment_point(image_data, (x, y), None).await?;
            
            if let Some(best_mask) = masks.first() {
                // Find the centroid of the mask for a better click point
                let centroid = self.calculate_mask_centroid(best_mask).await?;
                refined_targets.push(centroid);
            } else {
                // If no mask found, keep original point
                refined_targets.push((x, y));
            }
        }

        Ok(refined_targets)
    }

    /// Clean up old cache entries to manage memory
    pub async fn cleanup_cache(&self) -> LunaResult<()> {
        let mut cache = self.mask_cache.write().await;

        let now = Instant::now();
        let max_age = Duration::from_secs(1800); // 30 minutes

        // Remove old masks
        cache.retain(|_, cached| {
            now.duration_since(cached.created_at) < max_age
        });

        // If still over limit, remove least recently used
        while cache.len() > self.config.max_cache_size {
            let lru_key = cache
                .iter()
                .min_by_key(|(_, cached)| cached.access_count)
                .map(|(k, _)| k.clone());
            
            if let Some(key) = lru_key {
                cache.remove(&key);
            } else {
                break;
            }
        }

        debug!("Mask cache cleanup complete. Size: {}", cache.len());
        Ok(())
    }

    /// Validate all connections and dependencies
    pub async fn validate_connections(&self) -> LunaResult<()> {
        // Check device availability
        match self.device {
            Device::Cuda(_) => {
                if !candle_core::utils::cuda_is_available() {
                    return Err(LunaError::AiModel {
                        model: "SAM".to_string(),
                        error: "CUDA device configured but not available".to_string(),
                        suggestion: "Check CUDA installation or reconfigure device".to_string(),
                    });
                }
            }
            Device::Cpu => {
                // CPU is always available
            }
            _ => {}
        }

        // Test tensor operations
        let test_tensor = Tensor::zeros((1, 3, 1024, 1024), DType::F32, &self.device)
            .map_err(|e| LunaError::AiModel {
                model: "SAM".to_string(),
                error: format!("Device test failed: {}", e),
                suggestion: "Check device configuration and available memory".to_string(),
            })?;

        drop(test_tensor);

        info!("SAM connections validated successfully");
        Ok(())
    }

    /// Get current model status and statistics
    pub async fn get_status(&self) -> HashMap<String, String> {
        let mut status = HashMap::new();
        
        status.insert("model".to_string(), "SAM".to_string());
        status.insert("initialized".to_string(), self.is_initialized.read().await.to_string());
        status.insert("device".to_string(), format!("{:?}", self.device));
        status.insert("min_confidence".to_string(), self.config.min_confidence.to_string());
        status.insert("max_masks".to_string(), self.config.max_masks.to_string());
        status.insert("postprocessing_enabled".to_string(), self.config.enable_postprocessing.to_string());
        status.insert("caching_enabled".to_string(), self.config.enable_caching.to_string());
        
        let total_segmentations = *self.total_segmentations.read().await;
        let total_masks = *self.total_masks_generated.read().await;
        
        status.insert("total_segmentations".to_string(), total_segmentations.to_string());
        status.insert("total_masks_generated".to_string(), total_masks.to_string());
        
        if total_segmentations > 0 {
            let avg_masks = total_masks as f64 / total_segmentations as f64;
            status.insert("avg_masks_per_segmentation".to_string(), format!("{:.1}", avg_masks));
        }

        if self.config.enable_caching {
            let cache_size = self.mask_cache.read().await.len();
            status.insert("cache_size".to_string(), cache_size.to_string());
        }

        if let Some(last_inference) = *self.last_inference.read().await {
            let time_since = Instant::now().duration_since(last_inference);
            status.insert("last_inference_ago_ms".to_string(), time_since.as_millis().to_string());
        }

        status
    }

    /// Cleanup resources and prepare for shutdown
    pub async fn cleanup(&mut self) -> LunaResult<()> {
        info!("Cleaning up SAM specialist resources");
        
        // Clear model from memory
        self.model = None;
        *self.is_initialized.write().await = false;
        
        // Clear cache
        self.mask_cache.write().await.clear();
        
        // Reset statistics
        *self.total_segmentations.write().await = 0;
        *self.total_masks_generated.write().await = 0;
        
        // Trigger garbage collection
        self.memory_manager.trigger_gc().await;
        
        Ok(())
    }

    // Private helper methods

    async fn load_model_config(&self) -> LunaResult<SamConfig> {
        // For now, return a default config
        // In a real implementation, this would load from the model directory
        Ok(SamConfig::default())
    }

    async fn create_var_builder(&self) -> LunaResult<VarBuilder<'static>> {
        // This is a placeholder - in a real implementation, this would
        // load the actual model weights from disk or download them
        Err(LunaError::AiModel {
            model: "SAM".to_string(),
            error: "Model loading not implemented".to_string(),
            suggestion: "This is a mock implementation for demonstration".to_string(),
        })
    }

    async fn check_rate_limit(&self) -> LunaResult<()> {
        if let Some(last_inference) = *self.last_inference.read().await {
            let time_since = Instant::now().duration_since(last_inference);
            if time_since < Duration::from_millis(500) { // Min 500ms between inferences
                return Err(LunaError::AiModel {
                    model: "SAM".to_string(),
                    error: "Rate limit exceeded".to_string(),
                    suggestion: "Wait before making another inference request".to_string(),
                });
            }
        }
        Ok(())
    }

    async fn process_single_prompt(
        &self,
        image_data: &[u8],
        prompt: &SegmentationPrompt,
    ) -> LunaResult<Vec<SegmentationMask>> {
        // Check cache first if enabled
        if self.config.enable_caching {
            let cache_key = self.generate_cache_key(image_data, prompt).await;
            let mut cache = self.mask_cache.write().await;
            
            if let Some(cached) = cache.get_mut(&cache_key) {
                cached.access_count += 1;
                debug!("Using cached mask for prompt: {:?}", prompt.prompt_type);
                return Ok(vec![cached.mask.clone()]);
            }
        }

        // Run segmentation
        let masks = self.run_segmentation(image_data, prompt).await?;

        // Cache results if enabled
        if self.config.enable_caching && !masks.is_empty() {
            let cache_key = self.generate_cache_key(image_data, prompt).await;
            let mut cache = self.mask_cache.write().await;
            
            // Cache the first (best) mask
            cache.insert(cache_key, CachedMask {
                mask: masks[0].clone(),
                created_at: Instant::now(),
                access_count: 1,
            });
        }

        Ok(masks)
    }

    async fn run_segmentation(
        &self,
        image_data: &[u8],
        prompt: &SegmentationPrompt,
    ) -> LunaResult<Vec<SegmentationMask>> {
        // Mock segmentation results for demonstration
        // In a real implementation, this would run the actual SAM model
        let (width, height) = (800, 600); // Mock image dimensions
        
        let masks = match prompt.prompt_type {
            PromptType::Point => {
                if let PromptData::Point(x, y) = prompt.data {
                    vec![self.create_mock_mask(format!("point_{}_{}", x, y), x, y, width, height).await]
                } else {
                    vec![]
                }
            }
            PromptType::Box => {
                if let PromptData::Box(x, y, w, h) = prompt.data {
                    vec![self.create_mock_mask(format!("box_{}_{}_{}_{}",x, y, w, h), x + w/2, y + h/2, width, height).await]
                } else {
                    vec![]
                }
            }
            PromptType::Everything => {
                // Generate multiple masks for different regions
                vec![
                    self.create_mock_mask("region_1".to_string(), 100, 100, width, height).await,
                    self.create_mock_mask("region_2".to_string(), 300, 200, width, height).await,
                    self.create_mock_mask("region_3".to_string(), 500, 300, width, height).await,
                ]
            }
            PromptType::Points => {
                if let PromptData::Points(points) = &prompt.data {
                    let mut masks = Vec::new();
                    for (i, &(x, y)) in points.iter().enumerate() {
                        masks.push(self.create_mock_mask(format!("point_{}", i), x, y, width, height).await);
                    }
                    masks
                } else {
                    vec![]
                }
            }
        };

        Ok(masks)
    }

    async fn create_mock_mask(
        &self,
        id: String,
        center_x: u32,
        center_y: u32,
        image_width: u32,
        image_height: u32,
    ) -> SegmentationMask {
        // Create a mock circular mask around the center point
        let mask_size = 50; // Mock mask radius
        let bbox = (
            center_x.saturating_sub(mask_size),
            center_y.saturating_sub(mask_size),
            mask_size * 2,
            mask_size * 2,
        );

        // Generate mock mask data (binary image)
        let mask_data = vec![128; (mask_size * 2 * mask_size * 2) as usize]; // Mock gray mask

        let mut metadata = HashMap::new();
        metadata.insert("source".to_string(), "sam".to_string());
        metadata.insert("center_x".to_string(), center_x.to_string());
        metadata.insert("center_y".to_string(), center_y.to_string());
        metadata.insert("image_dimensions".to_string(), format!("{}x{}", image_width, image_height));

        SegmentationMask {
            id,
            mask_data,
            dimensions: (mask_size * 2, mask_size * 2),
            confidence: 0.85 + (rand::random::<f32>() * 0.1), // Mock confidence 0.85-0.95
            bbox,
            area_pixels: (std::f32::consts::PI * mask_size as f32 * mask_size as f32) as u32,
            metadata,
        }
    }

    async fn postprocess_masks(&self, masks: Vec<SegmentationMask>) -> LunaResult<Vec<SegmentationMask>> {
        // Mock post-processing
        // In a real implementation, this would:
        // - Remove duplicate masks
        // - Merge overlapping masks
        // - Smooth mask boundaries
        // - Filter out noise
        debug!("Post-processing {} masks", masks.len());
        Ok(masks)
    }

    async fn calculate_mask_centroid(&self, mask: &SegmentationMask) -> LunaResult<(u32, u32)> {
        // Mock centroid calculation
        // In a real implementation, this would analyze the mask data
        let center = (
            mask.bbox.0 + mask.bbox.2 / 2,
            mask.bbox.1 + mask.bbox.3 / 2,
        );
        Ok(center)
    }

    async fn generate_cache_key(&self, image_data: &[u8], prompt: &SegmentationPrompt) -> String {
        // Generate a cache key based on image hash and prompt
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        image_data.hash(&mut hasher);
        format!("{:?}", prompt.prompt_type).hash(&mut hasher);
        format!("{:?}", prompt.data).hash(&mut hasher);
        
        format!("sam_{}_{:x}", prompt.prompt_type, hasher.finish())
    }

    fn estimate_model_memory_mb(&self) -> u64 {
        // Rough estimate for SAM model memory usage
        let base_memory = 2048; // ~2GB for model
        let cache_memory = if self.config.enable_caching {
            (self.config.max_cache_size as u64 * 10) // ~10MB per cached mask
        } else {
            0
        };
        base_memory + cache_memory
    }

    fn estimate_inference_memory_mb(&self, image_size_bytes: usize) -> u64 {
        // Estimate memory needed for inference
        let base_memory = 512; // Base overhead
        let image_memory = (image_size_bytes / 1024 / 1024) as u64 * 8; // 8x for processing
        base_memory + image_memory
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::MemoryManager;
    use crate::utils::MetricsCollector;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_sam_creation() {
        let memory_manager = Arc::new(MemoryManager::new().unwrap());
        let metrics = Arc::new(MetricsCollector::new());
        let config = SamConfig::default();
        
        let specialist = SamSpecialist::new(config, memory_manager, metrics);
        assert!(specialist.is_ok());
    }

    #[tokio::test]
    async fn test_invalid_confidence_threshold() {
        let memory_manager = Arc::new(MemoryManager::new().unwrap());
        let metrics = Arc::new(MetricsCollector::new());
        let mut config = SamConfig::default();
        config.min_confidence = 1.5; // Invalid
        
        let specialist = SamSpecialist::new(config, memory_manager, metrics);
        assert!(specialist.is_err());
    }

    #[tokio::test]
    async fn test_prompt_types() {
        let point_prompt = SegmentationPrompt {
            prompt_type: PromptType::Point,
            data: PromptData::Point(100, 200),
            label_hint: Some("button".to_string()),
        };
        
        assert!(matches!(point_prompt.prompt_type, PromptType::Point));
        
        let box_prompt = SegmentationPrompt {
            prompt_type: PromptType::Box,
            data: PromptData::Box(50, 60, 100, 80),
            label_hint: None,
        };
        
        assert!(matches!(box_prompt.prompt_type, PromptType::Box));
    }

    #[tokio::test]
    async fn test_status_tracking() {
        let memory_manager = Arc::new(MemoryManager::new().unwrap());
        let metrics = Arc::new(MetricsCollector::new());
        let config = SamConfig::default();
        
        let specialist = SamSpecialist::new(config, memory_manager, metrics).unwrap();
        let status = specialist.get_status().await;
        
        assert_eq!(status.get("model").unwrap(), "SAM");
        assert_eq!(status.get("initialized").unwrap(), "false");
        assert_eq!(status.get("total_segmentations").unwrap(), "0");
    }

    #[tokio::test]
    async fn test_cache_functionality() {
        let memory_manager = Arc::new(MemoryManager::new().unwrap());
        let metrics = Arc::new(MetricsCollector::new());
        let config = SamConfig::default();
        
        let specialist = SamSpecialist::new(config, memory_manager, metrics).unwrap();
        
        // Test cache cleanup
        let result = specialist.cleanup_cache().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_memory_estimation() {
        let memory_manager = Arc::new(MemoryManager::new().unwrap());
        let metrics = Arc::new(MetricsCollector::new());
        let config = SamConfig::default();
        
        let specialist = SamSpecialist::new(config, memory_manager, metrics).unwrap();
        let memory_estimate = specialist.estimate_model_memory_mb();
        
        assert!(memory_estimate > 0);
    }
}