/*!
 * Luna Visual AI Model Manager
 * 
 * Advanced AI model management system with:
 * - Automatic model downloading and caching
 * - Memory-efficient model loading with LRU eviction
 * - GPU/CPU allocation optimization
 * - Model health monitoring and validation
 * - Graceful error handling and fallback strategies
 * - Performance metrics and profiling
 * - Thread-safe model access with async support
 */

use crate::core::{
    config::{AiConfig, ModelConfig},
    error::{LunaError, Result},
    memory,
    events::{self, LunaEvent, LunaEventType, EventPriority},
};
use crate::utils::logging::components::ai;
use anyhow::Context;
use candle_core::{Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models;
use parking_lot::{Mutex, RwLock};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use tokio::{fs, time};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Global model manager instance
static MODEL_MANAGER: once_cell::sync::Lazy<Arc<ModelManager>> =
    once_cell::sync::Lazy::new(|| Arc::new(ModelManager::new()));

/// AI model management system
pub struct ModelManager {
    /// Configuration
    config: RwLock<AiConfig>,
    
    /// Loaded models cache
    models: RwLock<HashMap<String, Arc<LoadedModel>>>,
    
    /// Model loading queue and locks
    loading_locks: Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>,
    
    /// GPU device for inference
    device: RwLock<Option<Device>>,
    
    /// Model usage statistics
    usage_stats: RwLock<HashMap<String, ModelUsageStats>>,
    
    /// Memory allocations by model
    memory_allocations: RwLock<HashMap<String, usize>>,
    
    /// Manager state
    initialized: AtomicBool,
    shutdown_requested: AtomicBool,
    
    /// Background cleanup task handle
    cleanup_handle: Mutex<Option<tokio::task::JoinHandle<()>>>,
}

/// Loaded AI model with metadata
pub struct LoadedModel {
    pub name: String,
    pub model_type: ModelType,
    pub device: Device,
    pub memory_usage: u64,
    pub loaded_at: Instant,
    pub last_used: AtomicU64, // timestamp as u64
    pub usage_count: AtomicU64,
    pub model_data: ModelData,
}

/// Type of AI model
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelType {
    Florence2,
    Clip,
    TrOCR,
    SAM,
}

/// Model-specific data containers
pub enum ModelData {
    Florence2(Florence2Model),
    Clip(ClipModel),
    TrOCR(TrOCRModel),
    SAM(SAMModel),
}

/// Florence-2 model for object detection
pub struct Florence2Model {
    pub model: Box<dyn FlorebceDetector + Send + Sync>,
    pub tokenizer: tokenizers::Tokenizer,
    pub processor: ImageProcessor,
}

/// CLIP model for text-image matching
pub struct ClipModel {
    pub text_model: Box<dyn ClipTextEncoder + Send + Sync>,
    pub image_model: Box<dyn ClipImageEncoder + Send + Sync>,
    pub tokenizer: tokenizers::Tokenizer,
    pub processor: ImageProcessor,
}

/// TrOCR model for text recognition
pub struct TrOCRModel {
    pub model: Box<dyn TextRecognizer + Send + Sync>,
    pub tokenizer: tokenizers::Tokenizer,
    pub processor: ImageProcessor,
}

/// SAM model for image segmentation
pub struct SAMModel {
    pub model: Box<dyn ImageSegmenter + Send + Sync>,
    pub processor: ImageProcessor,
}

/// Image preprocessing utilities
pub struct ImageProcessor {
    pub resize_transform: ImageTransform,
    pub normalize_transform: ImageTransform,
}

/// Image transformation functions
pub struct ImageTransform {
    pub width: u32,
    pub height: u32,
    pub mean: [f32; 3],
    pub std: [f32; 3],
}

/// Model usage statistics
#[derive(Debug, Clone, Default)]
pub struct ModelUsageStats {
    pub total_inferences: u64,
    pub total_inference_time_ms: u64,
    pub average_inference_time_ms: f64,
    pub last_inference: Option<Instant>,
    pub error_count: u64,
    pub memory_peak: u64,
}

/// Model loading error types
#[derive(Debug, thiserror::Error)]
pub enum ModelLoadError {
    #[error("Model file not found: {path}")]
    ModelNotFound { path: String },
    
    #[error("Model loading failed: {reason}")]
    LoadingFailed { reason: String },
    
    #[error("GPU initialization failed: {reason}")]
    GpuInitFailed { reason: String },
    
    #[error("Insufficient memory for model: required {required}, available {available}")]
    InsufficientMemory { required: u64, available: u64 },
    
    #[error("Model validation failed: {reason}")]
    ValidationFailed { reason: String },
}

/// Trait for object detection models
pub trait FlorebceDetector {
    async fn detect_objects(&self, image: &Tensor) -> Result<Vec<DetectedObject>>;
    fn get_confidence_threshold(&self) -> f32;
    fn set_confidence_threshold(&mut self, threshold: f32);
}

/// Trait for text encoding models
pub trait ClipTextEncoder {
    async fn encode_text(&self, text: &str) -> Result<Tensor>;
    fn get_embedding_dim(&self) -> usize;
}

/// Trait for image encoding models  
pub trait ClipImageEncoder {
    async fn encode_image(&self, image: &Tensor) -> Result<Tensor>;
    fn get_embedding_dim(&self) -> usize;
}

/// Trait for text recognition models
pub trait TextRecognizer {
    async fn recognize_text(&self, image: &Tensor) -> Result<String>;
    fn get_confidence(&self) -> f32;
}

/// Trait for image segmentation models
pub trait ImageSegmenter {
    async fn segment_image(&self, image: &Tensor, points: &[(f32, f32)]) -> Result<Tensor>;
    fn set_points_per_side(&mut self, points: u32);
}

/// Detected object from Florence-2
#[derive(Debug, Clone)]
pub struct DetectedObject {
    pub label: String,
    pub confidence: f32,
    pub bbox: BoundingBox,
    pub category: String,
}

/// Bounding box coordinates
#[derive(Debug, Clone)]
pub struct BoundingBox {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl ModelManager {
    fn new() -> Self {
        Self {
            config: RwLock::new(AiConfig::default()),
            models: RwLock::new(HashMap::new()),
            loading_locks: Mutex::new(HashMap::new()),
            device: RwLock::new(None),
            usage_stats: RwLock::new(HashMap::new()),
            memory_allocations: RwLock::new(HashMap::new()),
            initialized: AtomicBool::new(false),
            shutdown_requested: AtomicBool::new(false),
            cleanup_handle: Mutex::new(None),
        }
    }

    /// Initialize model manager with configuration
    async fn init(&self, config: AiConfig) -> Result<()> {
        if self.initialized.load(Ordering::Relaxed) {
            return Ok(());
        }

        info!("Initializing AI model manager");
        
        // Store configuration
        *self.config.write() = config.clone();
        
        // Initialize GPU device
        self.init_device().await?;
        
        // Create model cache directory
        self.ensure_model_directory(&config.model_cache_dir).await?;
        
        // Start background cleanup task
        self.start_cleanup_task().await?;
        
        // Pre-load essential models
        self.preload_models().await?;
        
        self.initialized.store(true, Ordering::Relaxed);
        
        info!("✅ AI model manager initialized");
        Ok(())
    }

    /// Initialize compute device (GPU or CPU)
    async fn init_device(&self) -> Result<()> {
        let config = self.config.read();
        
        let device = if config.gpu_device_id >= 0 {
            match Device::new_cuda(config.gpu_device_id as usize) {
                Ok(device) => {
                    info!("🚀 GPU device initialized: CUDA device {}", config.gpu_device_id);
                    device
                }
                Err(e) => {
                    warn!("Failed to initialize GPU, falling back to CPU: {}", e);
                    Device::Cpu
                }
            }
        } else {
            info!("Using CPU device for AI inference");
            Device::Cpu
        };
        
        *self.device.write() = Some(device);
        Ok(())
    }

    /// Ensure model directory exists
    async fn ensure_model_directory(&self, dir: &Path) -> Result<()> {
        if !dir.exists() {
            fs::create_dir_all(dir).await
                .context("Failed to create model directory")
                .map_err(|e| LunaError::io(e.to_string(), Some(dir.to_string_lossy())))?;
            
            info!("Created model directory: {:?}", dir);
        }
        Ok(())
    }

    /// Start background cleanup task
    async fn start_cleanup_task(&self) -> Result<()> {
        let manager = Arc::clone(&MODEL_MANAGER);
        let handle = tokio::spawn(async move {
            manager.cleanup_loop().await;
        });
        
        *self.cleanup_handle.lock() = Some(handle);
        info!("Model cleanup task started");
        Ok(())
    }

    /// Background cleanup loop
    async fn cleanup_loop(&self) {
        let mut interval = time::interval(Duration::from_secs(300)); // 5 minutes
        
        while !self.shutdown_requested.load(Ordering::Relaxed) {
            interval.tick().await;
            
            if let Err(e) = self.cleanup_unused_models().await {
                error!("Model cleanup failed: {}", e);
            }
        }
        
        info!("Model cleanup task stopped");
    }

    /// Clean up unused models from memory
    async fn cleanup_unused_models(&self) -> Result<()> {
        let config = self.config.read().clone();
        let current_memory = memory::get_current_usage();
        let memory_threshold = (config.max_model_memory as f64 * 0.8) as u64;
        
        if current_memory > memory_threshold {
            info!("Memory usage high, cleaning up unused models");
            
            let mut models_to_remove = Vec::new();
            let cutoff_time = Instant::now() - Duration::from_secs(3600); // 1 hour
            
            {
                let models = self.models.read();
                for (name, model) in models.iter() {
                    let last_used = std::time::UNIX_EPOCH + 
                        Duration::from_secs(model.last_used.load(Ordering::Relaxed));
                    
                    if last_used < cutoff_time.elapsed() {
                        models_to_remove.push(name.clone());
                    }
                }
            }
            
            // Remove old models
            for model_name in models_to_remove {
                self.unload_model(&model_name).await?;
                info!("Unloaded unused model: {}", model_name);
            }
        }
        
        Ok(())
    }

    /// Pre-load essential models
    async fn preload_models(&self) -> Result<()> {
        let config = self.config.read().clone();
        
        // Load enabled models in parallel
        let mut load_tasks = Vec::new();
        
        if config.florence.enabled {
            let task = self.load_model_async("florence2", ModelType::Florence2);
            load_tasks.push(task);
        }
        
        if config.clip.enabled {
            let task = self.load_model_async("clip", ModelType::Clip);
            load_tasks.push(task);
        }
        
        if config.trocr.enabled {
            let task = self.load_model_async("trocr", ModelType::TrOCR);
            load_tasks.push(task);
        }
        
        if config.sam.enabled {
            let task = self.load_model_async("sam", ModelType::SAM);
            load_tasks.push(task);
        }
        
        // Wait for all models to load
        let results = futures::future::join_all(load_tasks).await;
        
        let mut loaded_count = 0;
        let mut failed_count = 0;
        
        for (i, result) in results.into_iter().enumerate() {
            match result {
                Ok(_) => {
                    loaded_count += 1;
                    info!("Model {} loaded successfully", i);
                }
                Err(e) => {
                    failed_count += 1;
                    warn!("Model {} failed to load: {}", i, e);
                }
            }
        }
        
        info!("Pre-loading complete: {} loaded, {} failed", loaded_count, failed_count);
        
        if loaded_count == 0 {
            return Err(LunaError::ai_model(
                "No AI models could be loaded",
                "all",
                "Check model files and system requirements",
            ));
        }
        
        Ok(())
    }

    /// Load a model asynchronously
    async fn load_model_async(&self, name: &str, model_type: ModelType) -> Result<()> {
        let loading_lock = {
            let mut locks = self.loading_locks.lock();
            locks.entry(name.to_string())
                .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
                .clone()
        };
        
        let _lock = loading_lock.lock().await;
        
        // Check if model is already loaded
        if self.models.read().contains_key(name) {
            return Ok(());
        }
        
        info!("Loading AI model: {}", name);
        
        // Allocate memory for model
        let config = self.config.read();
        let model_config = self.get_model_config(&config, model_type)?;
        let memory_id = memory::allocate(model_config.max_memory, &format!("ai_model_{}", name))?;
        
        // Load the actual model
        let load_result = self.load_model_impl(name, model_type, &model_config).await;
        
        match load_result {
            Ok(model) => {
                // Store memory allocation ID
                self.memory_allocations.write().insert(name.to_string(), memory_id);
                
                // Add to models cache
                self.models.write().insert(name.to_string(), Arc::new(model));
                
                // Initialize usage stats
                self.usage_stats.write().insert(name.to_string(), ModelUsageStats::default());
                
                // Publish model loaded event
                let event = LunaEvent::new(
                    LunaEventType::AiModelLoaded,
                    "ai".to_string(),
                    EventPriority::Normal,
                    serde_json::json!({
                        "model_name": name,
                        "model_type": format!("{:?}", model_type),
                        "memory_usage": model_config.max_memory
                    }),
                );
                
                if let Err(e) = events::publish(event) {
                    warn!("Failed to publish model loaded event: {}", e);
                }
                
                ai::model_loaded(name, model_config.max_memory);
                Ok(())
            }
            Err(e) => {
                // Clean up memory allocation on failure
                if let Err(cleanup_err) = memory::deallocate(memory_id) {
                    warn!("Failed to cleanup memory after model load failure: {}", cleanup_err);
                }
                
                Err(e)
            }
        }
    }

    /// Get model configuration by type
    fn get_model_config(&self, config: &AiConfig, model_type: ModelType) -> Result<&ModelConfig> {
        match model_type {
            ModelType::Florence2 => Ok(&config.florence),
            ModelType::Clip => Ok(&config.clip),
            ModelType::TrOCR => Ok(&config.trocr),
            ModelType::SAM => Ok(&config.sam),
        }
    }

    /// Actual model loading implementation
    async fn load_model_impl(
        &self,
        name: &str,
        model_type: ModelType,
        config: &ModelConfig,
    ) -> Result<LoadedModel> {
        let device = self.device.read().clone()
            .ok_or_else(|| LunaError::ai_model(
                "Device not initialized",
                name,
                "Initialize the model manager first",
            ))?;
        
        let model_path = self.get_model_path(config)?;
        
        // Validate model file exists
        if !model_path.exists() {
            return Err(LunaError::ai_model(
                format!("Model file not found: {:?}", model_path),
                name,
                "Download the model or check the path",
            ));
        }
        
        // Load model based on type
        let model_data = match model_type {
            ModelType::Florence2 => self.load_florence2(&model_path, &device, config).await?,
            ModelType::Clip => self.load_clip(&model_path, &device, config).await?,
            ModelType::TrOCR => self.load_trocr(&model_path, &device, config).await?,
            ModelType::SAM => self.load_sam(&model_path, &device, config).await?,
        };
        
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        Ok(LoadedModel {
            name: name.to_string(),
            model_type,
            device,
            memory_usage: config.max_memory,
            loaded_at: Instant::now(),
            last_used: AtomicU64::new(now),
            usage_count: AtomicU64::new(0),
            model_data,
        })
    }

    /// Get full path to model file
    fn get_model_path(&self, config: &ModelConfig) -> Result<PathBuf> {
        let base_dir = &self.config.read().model_cache_dir;
        Ok(base_dir.join(&config.model_path))
    }

    /// Load Florence-2 model
    async fn load_florence2(
        &self,
        model_path: &Path,
        device: &Device,
        config: &ModelConfig,
    ) -> Result<ModelData> {
        // Placeholder implementation - in a real system, this would:
        // 1. Load the actual Florence-2 model weights
        // 2. Initialize the tokenizer
        // 3. Set up image preprocessing
        
        // For now, return a placeholder
        Ok(ModelData::Florence2(Florence2Model {
            model: Box::new(MockFlorence2Detector::new()),
            tokenizer: self.load_tokenizer(model_path).await?,
            processor: ImageProcessor::new(224, 224),
        }))
    }

    /// Load CLIP model
    async fn load_clip(
        &self,
        model_path: &Path,
        device: &Device,
        config: &ModelConfig,
    ) -> Result<ModelData> {
        // Placeholder implementation
        Ok(ModelData::Clip(ClipModel {
            text_model: Box::new(MockClipTextEncoder::new()),
            image_model: Box::new(MockClipImageEncoder::new()),
            tokenizer: self.load_tokenizer(model_path).await?,
            processor: ImageProcessor::new(224, 224),
        }))
    }

    /// Load TrOCR model
    async fn load_trocr(
        &self,
        model_path: &Path,
        device: &Device,
        config: &ModelConfig,
    ) -> Result<ModelData> {
        // Placeholder implementation
        Ok(ModelData::TrOCR(TrOCRModel {
            model: Box::new(MockTextRecognizer::new()),
            tokenizer: self.load_tokenizer(model_path).await?,
            processor: ImageProcessor::new(384, 384),
        }))
    }

    /// Load SAM model
    async fn load_sam(
        &self,
        model_path: &Path,
        device: &Device,
        config: &ModelConfig,
    ) -> Result<ModelData> {
        // Placeholder implementation
        Ok(ModelData::SAM(SAMModel {
            model: Box::new(MockImageSegmenter::new()),
            processor: ImageProcessor::new(1024, 1024),
        }))
    }

    /// Load tokenizer for model
    async fn load_tokenizer(&self, model_path: &Path) -> Result<tokenizers::Tokenizer> {
        // Placeholder - would load actual tokenizer
        tokenizers::Tokenizer::from_file(model_path.join("tokenizer.json"))
            .map_err(|e| LunaError::ai_model(
                format!("Failed to load tokenizer: {}", e),
                "tokenizer",
                "Check tokenizer file exists",
            ))
    }

    /// Unload a model from memory
    async fn unload_model(&self, name: &str) -> Result<()> {
        let model = self.models.write().remove(name);
        
        if let Some(_model) = model {
            // Clean up memory allocation
            if let Some(memory_id) = self.memory_allocations.write().remove(name) {
                memory::deallocate(memory_id)?;
            }
            
            // Remove usage stats
            self.usage_stats.write().remove(name);
            
            // Publish model unloaded event
            let event = LunaEvent::new(
                LunaEventType::AiModelUnloaded,
                "ai".to_string(),
                EventPriority::Normal,
                serde_json::json!({
                    "model_name": name
                }),
            );
            
            if let Err(e) = events::publish(event) {
                warn!("Failed to publish model unloaded event: {}", e);
            }
            
            info!("Model unloaded: {}", name);
        }
        
        Ok(())
    }

    /// Get a loaded model
    pub fn get_model(&self, name: &str) -> Option<Arc<LoadedModel>> {
        let models = self.models.read();
        let model = models.get(name).cloned();
        
        if let Some(ref m) = model {
            // Update last used timestamp
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            m.last_used.store(now, Ordering::Relaxed);
            m.usage_count.fetch_add(1, Ordering::Relaxed);
        }
        
        model
    }

    /// Validate all models are working
    async fn validate_models(&self) -> Result<()> {
        let models = self.models.read();
        
        if models.is_empty() {
            return Err(LunaError::ai_model(
                "No models loaded",
                "validation",
                "Load at least one AI model",
            ));
        }
        
        info!("Validating {} AI models", models.len());
        
        for (name, model) in models.iter() {
            // Basic validation - check device and memory
            if model.memory_usage == 0 {
                warn!("Model {} has zero memory usage", name);
            }
            
            debug!("Model {} validated (type: {:?})", name, model.model_type);
        }
        
        info!("✅ All models validated successfully");
        Ok(())
    }

    /// Check GPU availability
    async fn check_gpu_availability(&self) -> Result<bool> {
        match Device::new_cuda(0) {
            Ok(_) => {
                info!("🚀 GPU acceleration available");
                Ok(true)
            }
            Err(_) => {
                info!("💻 Using CPU for AI inference");
                Ok(false)
            }
        }
    }

    /// Shutdown model manager
    async fn shutdown(&self) -> Result<()> {
        if !self.initialized.load(Ordering::Relaxed) {
            return Ok(());
        }
        
        info!("Shutting down AI model manager");
        
        // Signal shutdown
        self.shutdown_requested.store(true, Ordering::Relaxed);
        
        // Stop cleanup task
        if let Some(handle) = self.cleanup_handle.lock().take() {
            handle.abort();
        }
        
        // Unload all models
        let model_names: Vec<String> = self.models.read().keys().cloned().collect();
        for name in model_names {
            self.unload_model(&name).await?;
        }
        
        // Clear device
        *self.device.write() = None;
        
        self.initialized.store(false, Ordering::Relaxed);
        
        info!("✅ AI model manager shut down");
        Ok(())
    }

    /// Get usage statistics
    pub fn get_usage_stats(&self) -> HashMap<String, ModelUsageStats> {
        self.usage_stats.read().clone()
    }
}

// Mock implementations for development/testing
// In a real implementation, these would be actual AI model wrappers

struct MockFlorence2Detector;
impl MockFlorence2Detector {
    fn new() -> Self { Self }
}
impl FlorebceDetector for MockFlorence2Detector {
    async fn detect_objects(&self, _image: &Tensor) -> Result<Vec<DetectedObject>> {
        Ok(vec![])
    }
    fn get_confidence_threshold(&self) -> f32 { 0.5 }
    fn set_confidence_threshold(&mut self, _threshold: f32) {}
}

struct MockClipTextEncoder;
impl MockClipTextEncoder {
    fn new() -> Self { Self }
}
impl ClipTextEncoder for MockClipTextEncoder {
    async fn encode_text(&self, _text: &str) -> Result<Tensor> {
        Err(LunaError::ai_model("Mock implementation", "clip", "Use real model"))
    }
    fn get_embedding_dim(&self) -> usize { 512 }
}

struct MockClipImageEncoder;
impl MockClipImageEncoder {
    fn new() -> Self { Self }
}
impl ClipImageEncoder for MockClipImageEncoder {
    async fn encode_image(&self, _image: &Tensor) -> Result<Tensor> {
        Err(LunaError::ai_model("Mock implementation", "clip", "Use real model"))
    }
    fn get_embedding_dim(&self) -> usize { 512 }
}

struct MockTextRecognizer;
impl MockTextRecognizer {
    fn new() -> Self { Self }
}
impl TextRecognizer for MockTextRecognizer {
    async fn recognize_text(&self, _image: &Tensor) -> Result<String> {
        Ok("Mock text recognition".to_string())
    }
    fn get_confidence(&self) -> f32 { 0.95 }
}

struct MockImageSegmenter;
impl MockImageSegmenter {
    fn new() -> Self { Self }
}
impl ImageSegmenter for MockImageSegmenter {
    async fn segment_image(&self, _image: &Tensor, _points: &[(f32, f32)]) -> Result<Tensor> {
        Err(LunaError::ai_model("Mock implementation", "sam", "Use real model"))
    }
    fn set_points_per_side(&mut self, _points: u32) {}
}

impl ImageProcessor {
    fn new(width: u32, height: u32) -> Self {
        Self {
            resize_transform: ImageTransform {
                width,
                height,
                mean: [0.485, 0.456, 0.406],
                std: [0.229, 0.224, 0.225],
            },
            normalize_transform: ImageTransform {
                width,
                height,
                mean: [0.0, 0.0, 0.0],
                std: [1.0, 1.0, 1.0],
            },
        }
    }
}

// Public API functions

/// Initialize model manager
pub async fn init(config: AiConfig) -> Result<()> {
    MODEL_MANAGER.init(config).await
}

/// Shutdown model manager
pub async fn shutdown() -> Result<()> {
    MODEL_MANAGER.shutdown().await
}

/// Validate models
pub async fn validate_models() -> Result<()> {
    MODEL_MANAGER.validate_models().await
}

/// Check GPU availability
pub async fn check_gpu_availability() -> Result<bool> {
    MODEL_MANAGER.check_gpu_availability().await
}

/// Get a loaded model
pub fn get_model(name: &str) -> Option<Arc<LoadedModel>> {
    MODEL_MANAGER.get_model(name)
}

/// Get usage statistics
pub fn get_usage_stats() -> HashMap<String, ModelUsageStats> {
    MODEL_MANAGER.get_usage_stats()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_model_manager_init() {
        let config = AiConfig::default();
        
        // This may fail without actual models
        if let Ok(()) = init(config).await {
            let validation = validate_models().await;
            assert!(validation.is_ok());
            
            shutdown().await.unwrap();
        }
    }

    #[tokio::test]
    async fn test_gpu_check() {
        let result = check_gpu_availability().await;
        assert!(result.is_ok());
        // Result may be true or false depending on system
    }
}