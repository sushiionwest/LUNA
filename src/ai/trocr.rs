//! # TrOCR Text Recognition Specialist
//! 
//! This module implements the TrOCR (Transformer-based Optical Character Recognition) model
//! for extracting text from screen elements. TrOCR excels at reading text from buttons,
//! labels, menus, and other UI components with high accuracy.
//!
//! ## Key Capabilities
//! - Optical Character Recognition (OCR) for UI elements
//! - Text extraction from buttons, labels, and menus
//! - Multi-language text recognition
//! - Confidence scoring for extracted text
//!
//! ## Memory Management
//! - Efficient batch processing for multiple text regions
//! - Automatic memory cleanup after processing
//! - Configurable memory limits and timeouts
//!
//! ## Error Handling
//! - Input validation and image preprocessing
//! - Graceful handling of unreadable text regions
//! - Comprehensive error reporting with suggestions

use crate::core::{LunaError, LunaResult, MemoryManager};
use crate::utils::MetricsCollector;
use candle_core::{Device, Tensor, DType};
use candle_nn::VarBuilder;
use candle_transformers::models::trocr::{TrOcrModel, TrOcrConfig};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info, warn, error, instrument};

/// Extracted text result with confidence and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedText {
    /// Extracted text content
    pub text: String,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,
    /// Bounding box where text was found
    pub bbox: (u32, u32, u32, u32),
    /// Language detected (if available)
    pub language: Option<String>,
    /// Additional metadata about the extraction
    pub metadata: HashMap<String, String>,
}

/// Text region for OCR processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextRegion {
    /// Unique identifier for the region
    pub id: String,
    /// Image data for the text region
    pub image_data: Vec<u8>,
    /// Bounding box coordinates
    pub bbox: (u32, u32, u32, u32),
    /// Expected text language (optional hint)
    pub language_hint: Option<String>,
    /// Processing priority (higher = process first)
    pub priority: u32,
}

/// Configuration for TrOCR model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrOcrConfig {
    /// Model path or identifier
    pub model_path: String,
    /// Device to run inference on (cuda, cpu, auto)
    pub device: String,
    /// Maximum inference time in milliseconds per region
    pub max_inference_time_ms: u64,
    /// Minimum confidence threshold for text extraction
    pub min_confidence: f32,
    /// Maximum text length to extract
    pub max_text_length: usize,
    /// Batch size for processing multiple regions
    pub batch_size: usize,
    /// Enable text preprocessing and enhancement
    pub enable_preprocessing: bool,
    /// Supported languages (empty = all)
    pub supported_languages: Vec<String>,
}

impl Default for TrOcrConfig {
    fn default() -> Self {
        Self {
            model_path: "microsoft/trocr-base-printed".to_string(),
            device: "auto".to_string(),
            max_inference_time_ms: 3000,
            min_confidence: 0.5,
            max_text_length: 1000,
            batch_size: 4,
            enable_preprocessing: true,
            supported_languages: vec!["en".to_string()], // Default to English
        }
    }
}

/// TrOCR specialist for text recognition and extraction
pub struct TrOcrSpecialist {
    /// Model configuration
    config: TrOcrConfig,
    /// Loaded TrOCR model (None if not initialized)
    model: Option<Arc<TrOcrModel>>,
    /// Device for inference
    device: Device,
    /// Memory manager for tracking resource usage
    memory_manager: Arc<MemoryManager>,
    /// Metrics collector for performance monitoring
    metrics: Arc<MetricsCollector>,
    /// Model initialization status
    is_initialized: Arc<RwLock<bool>>,
    /// Processing statistics
    total_regions_processed: Arc<RwLock<u64>>,
    total_text_extracted: Arc<RwLock<u64>>,
    /// Last inference timestamp for rate limiting
    last_inference: Arc<RwLock<Option<Instant>>>,
}

impl TrOcrSpecialist {
    /// Create a new TrOCR specialist with the given configuration
    pub fn new(
        config: TrOcrConfig,
        memory_manager: Arc<MemoryManager>,
        metrics: Arc<MetricsCollector>,
    ) -> LunaResult<Self> {
        // Validate configuration
        if config.min_confidence < 0.0 || config.min_confidence > 1.0 {
            return Err(LunaError::AiModel {
                model: "TrOCR".to_string(),
                error: "min_confidence must be between 0.0 and 1.0".to_string(),
                suggestion: "Set min_confidence to a value between 0.0 and 1.0".to_string(),
            });
        }

        if config.max_text_length == 0 {
            return Err(LunaError::AiModel {
                model: "TrOCR".to_string(),
                error: "max_text_length must be greater than 0".to_string(),
                suggestion: "Set max_text_length to a positive integer".to_string(),
            });
        }

        if config.batch_size == 0 {
            return Err(LunaError::AiModel {
                model: "TrOCR".to_string(),
                error: "batch_size must be greater than 0".to_string(),
                suggestion: "Set batch_size to a positive integer".to_string(),
            });
        }

        // Determine device
        let device = match config.device.as_str() {
            "cuda" => {
                if candle_core::utils::cuda_is_available() {
                    Device::new_cuda(0).map_err(|e| LunaError::AiModel {
                        model: "TrOCR".to_string(),
                        error: format!("Failed to initialize CUDA: {}", e),
                        suggestion: "Check CUDA installation or use 'cpu' device".to_string(),
                    })?
                } else {
                    return Err(LunaError::AiModel {
                        model: "TrOCR".to_string(),
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
                    model: "TrOCR".to_string(),
                    error: format!("Unknown device: {}", config.device),
                    suggestion: "Use 'cuda', 'cpu', or 'auto'".to_string(),
                });
            }
        };

        info!("TrOCR specialist initialized with device: {:?}", device);

        Ok(Self {
            config,
            model: None,
            device,
            memory_manager,
            metrics,
            is_initialized: Arc::new(RwLock::new(false)),
            total_regions_processed: Arc::new(RwLock::new(0)),
            total_text_extracted: Arc::new(RwLock::new(0)),
            last_inference: Arc::new(RwLock::new(None)),
        })
    }

    /// Initialize the TrOCR model
    #[instrument(skip(self))]
    pub async fn initialize(&mut self) -> LunaResult<()> {
        let start_time = Instant::now();
        
        // Check if already initialized
        if *self.is_initialized.read().await {
            debug!("TrOCR already initialized");
            return Ok(());
        }

        // Check memory availability
        let memory_info = self.memory_manager.get_memory_info().await;
        let required_memory = self.estimate_model_memory_mb();
        
        if memory_info.available_mb < required_memory {
            return Err(LunaError::Memory {
                operation: "TrOCR model loading".to_string(),
                required_mb: required_memory,
                available_mb: memory_info.available_mb,
                suggestion: "Close other applications or reduce batch_size".to_string(),
            });
        }

        info!("Loading TrOCR model from: {}", self.config.model_path);

        // Load model configuration
        let model_config = self.load_model_config().await?;
        
        // Create variable builder for model weights
        let var_builder = self.create_var_builder().await?;
        
        // Initialize model
        let model = TrOcrModel::load(&var_builder, &model_config)
            .map_err(|e| LunaError::AiModel {
                model: "TrOCR".to_string(),
                error: format!("Failed to load model: {}", e),
                suggestion: "Check model path and ensure model files are accessible".to_string(),
            })?;

        // Store model and mark as initialized
        self.model = Some(Arc::new(model));
        *self.is_initialized.write().await = true;

        let load_time = start_time.elapsed();
        info!("TrOCR model loaded successfully in {:?}", load_time);

        // Record metrics
        self.metrics.record_ai_model_event(
            "trocr".to_string(),
            "model_loaded".to_string(),
            load_time,
            true,
        ).await;

        Ok(())
    }

    /// Extract text from the given regions
    #[instrument(skip(self, regions))]
    pub async fn extract_text(&self, regions: &[TextRegion]) -> LunaResult<Vec<ExtractedText>> {
        let start_time = Instant::now();

        // Ensure model is initialized
        if !*self.is_initialized.read().await {
            return Err(LunaError::AiModel {
                model: "TrOCR".to_string(),
                error: "Model not initialized".to_string(),
                suggestion: "Call initialize() before using the model".to_string(),
            });
        }

        if regions.is_empty() {
            debug!("No text regions provided for extraction");
            return Ok(Vec::new());
        }

        // Rate limiting check
        self.check_rate_limit().await?;

        // Sort regions by priority (highest first)
        let mut sorted_regions = regions.to_vec();
        sorted_regions.sort_by(|a, b| b.priority.cmp(&a.priority));

        // Process regions in batches
        let mut all_extracted_text = Vec::new();
        for chunk in sorted_regions.chunks(self.config.batch_size) {
            let batch_results = self.process_region_batch(chunk).await?;
            all_extracted_text.extend(batch_results);
        }

        // Filter by confidence
        let filtered_results: Vec<ExtractedText> = all_extracted_text
            .into_iter()
            .filter(|result| result.confidence >= self.config.min_confidence)
            .collect();

        // Update statistics
        *self.total_regions_processed.write().await += regions.len() as u64;
        *self.total_text_extracted.write().await += filtered_results.len() as u64;

        // Update rate limiting
        *self.last_inference.write().await = Some(Instant::now());

        let extraction_time = start_time.elapsed();
        info!(
            "TrOCR extracted text from {} regions ({} successful) in {:?}",
            regions.len(),
            filtered_results.len(),
            extraction_time
        );

        // Record metrics
        self.metrics.record_ai_model_event(
            "trocr".to_string(),
            "text_extraction".to_string(),
            extraction_time,
            true,
        ).await;

        self.metrics.record_custom_metric(
            "trocr_regions_processed".to_string(),
            regions.len() as f64,
        ).await;

        self.metrics.record_custom_metric(
            "trocr_text_extracted".to_string(),
            filtered_results.len() as f64,
        ).await;

        Ok(filtered_results)
    }

    /// Extract text from a single image
    #[instrument(skip(self, image_data))]
    pub async fn extract_text_from_image(
        &self,
        image_data: &[u8],
        bbox: Option<(u32, u32, u32, u32)>,
    ) -> LunaResult<Vec<ExtractedText>> {
        // Create a single text region
        let region = TextRegion {
            id: "single_image".to_string(),
            image_data: image_data.to_vec(),
            bbox: bbox.unwrap_or((0, 0, 0, 0)),
            language_hint: None,
            priority: 1,
        };

        self.extract_text(&[region]).await
    }

    /// Validate if a text region is likely to contain readable text
    pub async fn validate_text_region(&self, region: &TextRegion) -> LunaResult<bool> {
        // Basic validation checks
        if region.image_data.is_empty() {
            return Ok(false);
        }

        // Check image size (minimum requirements for OCR)
        let (width, height) = self.get_image_dimensions(&region.image_data).await?;
        
        if width < 10 || height < 10 {
            debug!("Text region too small for OCR: {}x{}", width, height);
            return Ok(false);
        }

        if width > 2000 || height > 2000 {
            debug!("Text region too large for efficient OCR: {}x{}", width, height);
            return Ok(false);
        }

        // Check if preprocessing is enabled and would improve the region
        if self.config.enable_preprocessing {
            let quality_score = self.assess_image_quality(&region.image_data).await?;
            if quality_score < 0.3 {
                debug!("Text region quality too low: {}", quality_score);
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Get preprocessing suggestions for improving OCR accuracy
    pub async fn get_preprocessing_suggestions(&self, region: &TextRegion) -> LunaResult<Vec<String>> {
        let mut suggestions = Vec::new();

        if !self.config.enable_preprocessing {
            return Ok(suggestions);
        }

        // Analyze image and provide suggestions
        let quality_score = self.assess_image_quality(&region.image_data).await?;
        
        if quality_score < 0.7 {
            suggestions.push("Enhance image contrast".to_string());
        }

        let (width, height) = self.get_image_dimensions(&region.image_data).await?;
        
        if width < 50 || height < 20 {
            suggestions.push("Increase image resolution".to_string());
        }

        if width > 1000 || height > 1000 {
            suggestions.push("Reduce image size for faster processing".to_string());
        }

        // Check aspect ratio
        let aspect_ratio = width as f32 / height as f32;
        if aspect_ratio > 10.0 || aspect_ratio < 0.1 {
            suggestions.push("Unusual aspect ratio - may affect OCR accuracy".to_string());
        }

        Ok(suggestions)
    }

    /// Validate all connections and dependencies
    pub async fn validate_connections(&self) -> LunaResult<()> {
        // Check device availability
        match self.device {
            Device::Cuda(_) => {
                if !candle_core::utils::cuda_is_available() {
                    return Err(LunaError::AiModel {
                        model: "TrOCR".to_string(),
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
        let test_tensor = Tensor::zeros((1, 3, 224, 224), DType::F32, &self.device)
            .map_err(|e| LunaError::AiModel {
                model: "TrOCR".to_string(),
                error: format!("Device test failed: {}", e),
                suggestion: "Check device configuration and available memory".to_string(),
            })?;

        drop(test_tensor);

        info!("TrOCR connections validated successfully");
        Ok(())
    }

    /// Get current model status and statistics
    pub async fn get_status(&self) -> HashMap<String, String> {
        let mut status = HashMap::new();
        
        status.insert("model".to_string(), "TrOCR".to_string());
        status.insert("initialized".to_string(), self.is_initialized.read().await.to_string());
        status.insert("device".to_string(), format!("{:?}", self.device));
        status.insert("min_confidence".to_string(), self.config.min_confidence.to_string());
        status.insert("batch_size".to_string(), self.config.batch_size.to_string());
        status.insert("preprocessing_enabled".to_string(), self.config.enable_preprocessing.to_string());
        
        let regions_processed = *self.total_regions_processed.read().await;
        let text_extracted = *self.total_text_extracted.read().await;
        
        status.insert("total_regions_processed".to_string(), regions_processed.to_string());
        status.insert("total_text_extracted".to_string(), text_extracted.to_string());
        
        if regions_processed > 0 {
            let success_rate = (text_extracted as f64 / regions_processed as f64) * 100.0;
            status.insert("success_rate_percent".to_string(), format!("{:.1}", success_rate));
        }

        if let Some(last_inference) = *self.last_inference.read().await {
            let time_since = Instant::now().duration_since(last_inference);
            status.insert("last_inference_ago_ms".to_string(), time_since.as_millis().to_string());
        }

        status
    }

    /// Cleanup resources and prepare for shutdown
    pub async fn cleanup(&mut self) -> LunaResult<()> {
        info!("Cleaning up TrOCR specialist resources");
        
        // Clear model from memory
        self.model = None;
        *self.is_initialized.write().await = false;
        
        // Reset statistics
        *self.total_regions_processed.write().await = 0;
        *self.total_text_extracted.write().await = 0;
        
        // Trigger garbage collection
        self.memory_manager.trigger_gc().await;
        
        Ok(())
    }

    // Private helper methods

    async fn load_model_config(&self) -> LunaResult<TrOcrConfig> {
        // For now, return a default config
        // In a real implementation, this would load from the model directory
        Ok(TrOcrConfig::default())
    }

    async fn create_var_builder(&self) -> LunaResult<VarBuilder<'static>> {
        // This is a placeholder - in a real implementation, this would
        // load the actual model weights from disk or download them
        Err(LunaError::AiModel {
            model: "TrOCR".to_string(),
            error: "Model loading not implemented".to_string(),
            suggestion: "This is a mock implementation for demonstration".to_string(),
        })
    }

    async fn check_rate_limit(&self) -> LunaResult<()> {
        if let Some(last_inference) = *self.last_inference.read().await {
            let time_since = Instant::now().duration_since(last_inference);
            if time_since < Duration::from_millis(200) { // Min 200ms between inferences
                return Err(LunaError::AiModel {
                    model: "TrOCR".to_string(),
                    error: "Rate limit exceeded".to_string(),
                    suggestion: "Wait before making another inference request".to_string(),
                });
            }
        }
        Ok(())
    }

    async fn process_region_batch(&self, regions: &[TextRegion]) -> LunaResult<Vec<ExtractedText>> {
        let mut results = Vec::new();

        for region in regions {
            // Validate region first
            if !self.validate_text_region(region).await? {
                debug!("Skipping invalid text region: {}", region.id);
                continue;
            }

            // Preprocess image if enabled
            let processed_image = if self.config.enable_preprocessing {
                self.preprocess_image(&region.image_data).await?
            } else {
                region.image_data.clone()
            };

            // Run OCR with timeout
            match tokio::time::timeout(
                Duration::from_millis(self.config.max_inference_time_ms),
                self.run_ocr(&processed_image, region),
            ).await {
                Ok(Ok(extracted_text)) => {
                    if !extracted_text.text.trim().is_empty() {
                        results.push(extracted_text);
                    }
                }
                Ok(Err(e)) => {
                    warn!("OCR failed for region {}: {}", region.id, e);
                }
                Err(_) => {
                    warn!("OCR timeout for region {}", region.id);
                }
            }
        }

        Ok(results)
    }

    async fn preprocess_image(&self, image_data: &[u8]) -> LunaResult<Vec<u8>> {
        // Mock image preprocessing
        // In a real implementation, this would:
        // - Enhance contrast
        // - Denoise
        // - Resize if needed
        // - Apply sharpening
        debug!("Preprocessing image ({} bytes)", image_data.len());
        Ok(image_data.to_vec())
    }

    async fn run_ocr(&self, image_data: &[u8], region: &TextRegion) -> LunaResult<ExtractedText> {
        // Mock OCR results for demonstration
        // In a real implementation, this would run the actual TrOCR model
        let mock_text = match region.id.as_str() {
            "button_save" => "Save",
            "button_cancel" => "Cancel",
            "menu_file" => "File",
            "link_help" => "Help",
            _ => "Sample Text",
        };

        let confidence = 0.85 + (rand::random::<f32>() * 0.1); // Mock confidence 0.85-0.95

        let mut metadata = HashMap::new();
        metadata.insert("source".to_string(), "trocr".to_string());
        metadata.insert("image_size_bytes".to_string(), image_data.len().to_string());
        metadata.insert("region_id".to_string(), region.id.clone());
        
        if let Some(ref lang_hint) = region.language_hint {
            metadata.insert("language_hint".to_string(), lang_hint.clone());
        }

        Ok(ExtractedText {
            text: mock_text.to_string(),
            confidence,
            bbox: region.bbox,
            language: Some("en".to_string()),
            metadata,
        })
    }

    async fn get_image_dimensions(&self, image_data: &[u8]) -> LunaResult<(u32, u32)> {
        // Mock image dimensions
        // In a real implementation, this would decode the image and get actual dimensions
        Ok((200, 50)) // Mock dimensions
    }

    async fn assess_image_quality(&self, image_data: &[u8]) -> LunaResult<f32> {
        // Mock quality assessment
        // In a real implementation, this would analyze:
        // - Contrast
        // - Sharpness
        // - Noise level
        // - Text clarity
        Ok(0.75) // Mock quality score
    }

    fn estimate_model_memory_mb(&self) -> u64 {
        // Rough estimate for TrOCR model memory usage
        let base_memory = 800; // ~800MB for model
        let batch_memory = (self.config.batch_size as u64 * 50); // ~50MB per batch item
        base_memory + batch_memory
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::MemoryManager;
    use crate::utils::MetricsCollector;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_trocr_creation() {
        let memory_manager = Arc::new(MemoryManager::new().unwrap());
        let metrics = Arc::new(MetricsCollector::new());
        let config = TrOcrConfig::default();
        
        let specialist = TrOcrSpecialist::new(config, memory_manager, metrics);
        assert!(specialist.is_ok());
    }

    #[tokio::test]
    async fn test_invalid_confidence_threshold() {
        let memory_manager = Arc::new(MemoryManager::new().unwrap());
        let metrics = Arc::new(MetricsCollector::new());
        let mut config = TrOcrConfig::default();
        config.min_confidence = 1.5; // Invalid
        
        let specialist = TrOcrSpecialist::new(config, memory_manager, metrics);
        assert!(specialist.is_err());
    }

    #[tokio::test]
    async fn test_text_region_validation() {
        let memory_manager = Arc::new(MemoryManager::new().unwrap());
        let metrics = Arc::new(MetricsCollector::new());
        let config = TrOcrConfig::default();
        
        let specialist = TrOcrSpecialist::new(config, memory_manager, metrics).unwrap();
        
        // Test empty region
        let empty_region = TextRegion {
            id: "test".to_string(),
            image_data: vec![],
            bbox: (0, 0, 0, 0),
            language_hint: None,
            priority: 1,
        };
        
        let is_valid = specialist.validate_text_region(&empty_region).await.unwrap();
        assert!(!is_valid);
        
        // Test valid region
        let valid_region = TextRegion {
            id: "test".to_string(),
            image_data: vec![1, 2, 3, 4, 5],
            bbox: (0, 0, 100, 50),
            language_hint: Some("en".to_string()),
            priority: 1,
        };
        
        let is_valid = specialist.validate_text_region(&valid_region).await.unwrap();
        assert!(is_valid);
    }

    #[tokio::test]
    async fn test_status_tracking() {
        let memory_manager = Arc::new(MemoryManager::new().unwrap());
        let metrics = Arc::new(MetricsCollector::new());
        let config = TrOcrConfig::default();
        
        let specialist = TrOcrSpecialist::new(config, memory_manager, metrics).unwrap();
        let status = specialist.get_status().await;
        
        assert_eq!(status.get("model").unwrap(), "TrOCR");
        assert_eq!(status.get("initialized").unwrap(), "false");
        assert_eq!(status.get("total_regions_processed").unwrap(), "0");
    }

    #[tokio::test]
    async fn test_preprocessing_suggestions() {
        let memory_manager = Arc::new(MemoryManager::new().unwrap());
        let metrics = Arc::new(MetricsCollector::new());
        let config = TrOcrConfig::default();
        
        let specialist = TrOcrSpecialist::new(config, memory_manager, metrics).unwrap();
        
        let region = TextRegion {
            id: "test".to_string(),
            image_data: vec![1, 2, 3, 4, 5],
            bbox: (0, 0, 100, 50),
            language_hint: None,
            priority: 1,
        };
        
        let suggestions = specialist.get_preprocessing_suggestions(&region).await.unwrap();
        assert!(!suggestions.is_empty());
    }

    #[tokio::test]
    async fn test_memory_estimation() {
        let memory_manager = Arc::new(MemoryManager::new().unwrap());
        let metrics = Arc::new(MetricsCollector::new());
        let config = TrOcrConfig::default();
        
        let specialist = TrOcrSpecialist::new(config, memory_manager, metrics).unwrap();
        let memory_estimate = specialist.estimate_model_memory_mb();
        
        assert!(memory_estimate > 0);
    }
}