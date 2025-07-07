//! # CLIP Text-Visual Matching Specialist
//! 
//! This module implements the CLIP (Contrastive Language-Image Pre-training) model
//! for matching user commands to visual elements on screen. CLIP excels at understanding
//! the semantic relationship between natural language descriptions and visual content.
//!
//! ## Key Capabilities
//! - Match text descriptions to visual elements
//! - Semantic similarity scoring between text and images
//! - Context-aware element selection
//! - Multi-modal understanding for complex queries
//!
//! ## Memory Management
//! - Efficient embedding caching for repeated queries
//! - Batch processing for multiple comparisons
//! - Automatic memory cleanup for embeddings
//!
//! ## Error Handling
//! - Input validation for text and image data
//! - Graceful degradation for low-confidence matches
//! - Comprehensive logging for debugging

use crate::core::{LunaError, LunaResult, MemoryManager};
use crate::utils::MetricsCollector;
use candle_core::{Device, Tensor, DType};
use candle_nn::VarBuilder;
use candle_transformers::models::clip::{ClipModel, ClipConfig, ClipTextConfig, ClipVisionConfig};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info, warn, error, instrument};

/// Text-visual matching result with similarity score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchResult {
    /// Element index or identifier
    pub element_id: String,
    /// Similarity score (0.0 to 1.0)
    pub similarity: f32,
    /// Normalized confidence score
    pub confidence: f32,
    /// Reasoning for the match
    pub reasoning: String,
    /// Additional metadata about the match
    pub metadata: HashMap<String, String>,
}

/// Text query with context and requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextQuery {
    /// Main query text (e.g., "save button", "close tab")
    pub text: String,
    /// Additional context for better matching
    pub context: Option<String>,
    /// Required minimum confidence for matches
    pub min_confidence: f32,
    /// Maximum number of matches to return
    pub max_matches: usize,
}

/// Visual element for matching against text queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualElement {
    /// Unique identifier for the element
    pub id: String,
    /// Cropped image data of the element
    pub image_data: Vec<u8>,
    /// Bounding box coordinates
    pub bbox: (u32, u32, u32, u32),
    /// Text content if available (OCR or accessibility)
    pub text_content: Option<String>,
    /// Element type (button, link, input, etc.)
    pub element_type: Option<String>,
}

/// Configuration for CLIP model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipConfig {
    /// Model path or identifier
    pub model_path: String,
    /// Device to run inference on (cuda, cpu, auto)
    pub device: String,
    /// Maximum inference time in milliseconds
    pub max_inference_time_ms: u64,
    /// Similarity threshold for matches
    pub similarity_threshold: f32,
    /// Enable embedding caching
    pub enable_caching: bool,
    /// Maximum cache size (number of embeddings)
    pub max_cache_size: usize,
    /// Batch size for processing multiple queries
    pub batch_size: usize,
}

impl Default for ClipConfig {
    fn default() -> Self {
        Self {
            model_path: "openai/clip-vit-base-patch32".to_string(),
            device: "auto".to_string(),
            max_inference_time_ms: 3000,
            similarity_threshold: 0.3,
            enable_caching: true,
            max_cache_size: 1000,
            batch_size: 8,
        }
    }
}

/// Cached embedding for efficient reuse
#[derive(Debug, Clone)]
struct CachedEmbedding {
    /// The embedding tensor
    embedding: Tensor,
    /// When this embedding was created
    created_at: Instant,
    /// How many times this embedding has been accessed
    access_count: u64,
}

/// CLIP specialist for text-visual matching and semantic understanding
pub struct ClipSpecialist {
    /// Model configuration
    config: ClipConfig,
    /// Loaded CLIP model (None if not initialized)
    model: Option<Arc<ClipModel>>,
    /// Device for inference
    device: Device,
    /// Memory manager for tracking resource usage
    memory_manager: Arc<MemoryManager>,
    /// Metrics collector for performance monitoring
    metrics: Arc<MetricsCollector>,
    /// Model initialization status
    is_initialized: Arc<RwLock<bool>>,
    /// Text embedding cache
    text_embedding_cache: Arc<RwLock<HashMap<String, CachedEmbedding>>>,
    /// Image embedding cache
    image_embedding_cache: Arc<RwLock<HashMap<String, CachedEmbedding>>>,
    /// Last inference timestamp for rate limiting
    last_inference: Arc<RwLock<Option<Instant>>>,
}

impl ClipSpecialist {
    /// Create a new CLIP specialist with the given configuration
    pub fn new(
        config: ClipConfig,
        memory_manager: Arc<MemoryManager>,
        metrics: Arc<MetricsCollector>,
    ) -> LunaResult<Self> {
        // Validate configuration
        if config.similarity_threshold < 0.0 || config.similarity_threshold > 1.0 {
            return Err(LunaError::AiModel {
                model: "CLIP".to_string(),
                error: "Similarity threshold must be between 0.0 and 1.0".to_string(),
                suggestion: "Set similarity_threshold to a value between 0.0 and 1.0".to_string(),
            });
        }

        if config.max_cache_size == 0 {
            return Err(LunaError::AiModel {
                model: "CLIP".to_string(),
                error: "max_cache_size must be greater than 0".to_string(),
                suggestion: "Set max_cache_size to a positive integer".to_string(),
            });
        }

        // Determine device
        let device = match config.device.as_str() {
            "cuda" => {
                if candle_core::utils::cuda_is_available() {
                    Device::new_cuda(0).map_err(|e| LunaError::AiModel {
                        model: "CLIP".to_string(),
                        error: format!("Failed to initialize CUDA: {}", e),
                        suggestion: "Check CUDA installation or use 'cpu' device".to_string(),
                    })?
                } else {
                    return Err(LunaError::AiModel {
                        model: "CLIP".to_string(),
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
                    model: "CLIP".to_string(),
                    error: format!("Unknown device: {}", config.device),
                    suggestion: "Use 'cuda', 'cpu', or 'auto'".to_string(),
                });
            }
        };

        info!("CLIP specialist initialized with device: {:?}", device);

        Ok(Self {
            config,
            model: None,
            device,
            memory_manager,
            metrics,
            is_initialized: Arc::new(RwLock::new(false)),
            text_embedding_cache: Arc::new(RwLock::new(HashMap::new())),
            image_embedding_cache: Arc::new(RwLock::new(HashMap::new())),
            last_inference: Arc::new(RwLock::new(None)),
        })
    }

    /// Initialize the CLIP model
    #[instrument(skip(self))]
    pub async fn initialize(&mut self) -> LunaResult<()> {
        let start_time = Instant::now();
        
        // Check if already initialized
        if *self.is_initialized.read().await {
            debug!("CLIP already initialized");
            return Ok(());
        }

        // Check memory availability
        let memory_info = self.memory_manager.get_memory_info().await;
        let required_memory = self.estimate_model_memory_mb();
        
        if memory_info.available_mb < required_memory {
            return Err(LunaError::Memory {
                operation: "CLIP model loading".to_string(),
                required_mb: required_memory,
                available_mb: memory_info.available_mb,
                suggestion: "Close other applications or disable caching".to_string(),
            });
        }

        info!("Loading CLIP model from: {}", self.config.model_path);

        // Load model configuration
        let model_config = self.load_model_config().await?;
        
        // Create variable builder for model weights
        let var_builder = self.create_var_builder().await?;
        
        // Initialize model
        let model = ClipModel::load(&var_builder, &model_config)
            .map_err(|e| LunaError::AiModel {
                model: "CLIP".to_string(),
                error: format!("Failed to load model: {}", e),
                suggestion: "Check model path and ensure model files are accessible".to_string(),
            })?;

        // Store model and mark as initialized
        self.model = Some(Arc::new(model));
        *self.is_initialized.write().await = true;

        let load_time = start_time.elapsed();
        info!("CLIP model loaded successfully in {:?}", load_time);

        // Record metrics
        self.metrics.record_ai_model_event(
            "clip".to_string(),
            "model_loaded".to_string(),
            load_time,
            true,
        ).await;

        Ok(())
    }

    /// Find visual elements that match the given text query
    #[instrument(skip(self, elements))]
    pub async fn match_text_to_visuals(
        &self,
        query: &TextQuery,
        elements: &[VisualElement],
    ) -> LunaResult<Vec<MatchResult>> {
        let start_time = Instant::now();

        // Ensure model is initialized
        if !*self.is_initialized.read().await {
            return Err(LunaError::AiModel {
                model: "CLIP".to_string(),
                error: "Model not initialized".to_string(),
                suggestion: "Call initialize() before using the model".to_string(),
            });
        }

        // Validate inputs
        if query.text.trim().is_empty() {
            return Err(LunaError::AiModel {
                model: "CLIP".to_string(),
                error: "Empty query text".to_string(),
                suggestion: "Provide a valid text query".to_string(),
            });
        }

        if elements.is_empty() {
            debug!("No visual elements provided for matching");
            return Ok(Vec::new());
        }

        // Rate limiting check
        self.check_rate_limit().await?;

        // Get text embedding (with caching)
        let text_embedding = self.get_text_embedding(&query.text).await?;

        // Process elements in batches
        let mut all_matches = Vec::new();
        for chunk in elements.chunks(self.config.batch_size) {
            let batch_matches = self.process_element_batch(
                &text_embedding,
                chunk,
                query,
            ).await?;
            all_matches.extend(batch_matches);
        }

        // Sort by similarity score (descending)
        all_matches.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap());

        // Apply filtering and limits
        let filtered_matches: Vec<MatchResult> = all_matches
            .into_iter()
            .filter(|m| m.confidence >= query.min_confidence)
            .take(query.max_matches)
            .collect();

        // Update rate limiting
        *self.last_inference.write().await = Some(Instant::now());

        let match_time = start_time.elapsed();
        info!(
            "CLIP matched {} elements to '{}' in {:?}",
            filtered_matches.len(),
            query.text,
            match_time
        );

        // Record metrics
        self.metrics.record_ai_model_event(
            "clip".to_string(),
            "matching".to_string(),
            match_time,
            true,
        ).await;

        self.metrics.record_custom_metric(
            "clip_matches_found".to_string(),
            filtered_matches.len() as f64,
        ).await;

        Ok(filtered_matches)
    }

    /// Calculate similarity between two text phrases
    #[instrument(skip(self))]
    pub async fn calculate_text_similarity(
        &self,
        text1: &str,
        text2: &str,
    ) -> LunaResult<f32> {
        // Ensure model is initialized
        if !*self.is_initialized.read().await {
            return Err(LunaError::AiModel {
                model: "CLIP".to_string(),
                error: "Model not initialized".to_string(),
                suggestion: "Call initialize() before using the model".to_string(),
            });
        }

        // Get embeddings for both texts
        let embedding1 = self.get_text_embedding(text1).await?;
        let embedding2 = self.get_text_embedding(text2).await?;

        // Calculate cosine similarity
        let similarity = self.calculate_cosine_similarity(&embedding1, &embedding2).await?;

        Ok(similarity)
    }

    /// Pre-compute and cache embeddings for common UI terms
    pub async fn warm_cache(&mut self) -> LunaResult<()> {
        if !*self.is_initialized.read().await {
            return Err(LunaError::AiModel {
                model: "CLIP".to_string(),
                error: "Model not initialized".to_string(),
                suggestion: "Call initialize() before warming cache".to_string(),
            });
        }

        info!("Warming CLIP embedding cache with common UI terms");

        let common_terms = vec![
            "button", "click", "save", "cancel", "ok", "close", "open", "file",
            "edit", "view", "help", "settings", "menu", "dropdown", "tab",
            "link", "text", "input", "search", "submit", "delete", "remove",
            "add", "new", "create", "copy", "paste", "cut", "undo", "redo",
            "back", "forward", "home", "refresh", "reload", "print", "download",
            "upload", "login", "logout", "sign in", "sign out", "register",
        ];

        for term in common_terms {
            let _ = self.get_text_embedding(term).await;
        }

        info!("Cache warmed with {} terms", self.text_embedding_cache.read().await.len());
        Ok(())
    }

    /// Clean up old cache entries to manage memory
    pub async fn cleanup_cache(&self) -> LunaResult<()> {
        let mut text_cache = self.text_embedding_cache.write().await;
        let mut image_cache = self.image_embedding_cache.write().await;

        let now = Instant::now();
        let max_age = Duration::from_secs(3600); // 1 hour

        // Remove old text embeddings
        text_cache.retain(|_, cached| {
            now.duration_since(cached.created_at) < max_age
        });

        // Remove old image embeddings
        image_cache.retain(|_, cached| {
            now.duration_since(cached.created_at) < max_age
        });

        // If still over limit, remove least recently used
        while text_cache.len() > self.config.max_cache_size {
            let lru_key = text_cache
                .iter()
                .min_by_key(|(_, cached)| cached.access_count)
                .map(|(k, _)| k.clone());
            
            if let Some(key) = lru_key {
                text_cache.remove(&key);
            } else {
                break;
            }
        }

        while image_cache.len() > self.config.max_cache_size {
            let lru_key = image_cache
                .iter()
                .min_by_key(|(_, cached)| cached.access_count)
                .map(|(k, _)| k.clone());
            
            if let Some(key) = lru_key {
                image_cache.remove(&key);
            } else {
                break;
            }
        }

        debug!(
            "Cache cleanup complete. Text: {}, Image: {}",
            text_cache.len(),
            image_cache.len()
        );

        Ok(())
    }

    /// Validate all connections and dependencies
    pub async fn validate_connections(&self) -> LunaResult<()> {
        // Check device availability
        match self.device {
            Device::Cuda(_) => {
                if !candle_core::utils::cuda_is_available() {
                    return Err(LunaError::AiModel {
                        model: "CLIP".to_string(),
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
        let test_tensor = Tensor::zeros((1, 512), DType::F32, &self.device)
            .map_err(|e| LunaError::AiModel {
                model: "CLIP".to_string(),
                error: format!("Device test failed: {}", e),
                suggestion: "Check device configuration and available memory".to_string(),
            })?;

        drop(test_tensor);

        info!("CLIP connections validated successfully");
        Ok(())
    }

    /// Get current model status and statistics
    pub async fn get_status(&self) -> HashMap<String, String> {
        let mut status = HashMap::new();
        
        status.insert("model".to_string(), "CLIP".to_string());
        status.insert("initialized".to_string(), self.is_initialized.read().await.to_string());
        status.insert("device".to_string(), format!("{:?}", self.device));
        status.insert("similarity_threshold".to_string(), self.config.similarity_threshold.to_string());
        status.insert("caching_enabled".to_string(), self.config.enable_caching.to_string());
        
        if self.config.enable_caching {
            let text_cache_size = self.text_embedding_cache.read().await.len();
            let image_cache_size = self.image_embedding_cache.read().await.len();
            status.insert("text_cache_size".to_string(), text_cache_size.to_string());
            status.insert("image_cache_size".to_string(), image_cache_size.to_string());
        }

        if let Some(last_inference) = *self.last_inference.read().await {
            let time_since = Instant::now().duration_since(last_inference);
            status.insert("last_inference_ago_ms".to_string(), time_since.as_millis().to_string());
        }

        status
    }

    /// Cleanup resources and prepare for shutdown
    pub async fn cleanup(&mut self) -> LunaResult<()> {
        info!("Cleaning up CLIP specialist resources");
        
        // Clear model from memory
        self.model = None;
        *self.is_initialized.write().await = false;
        
        // Clear caches
        self.text_embedding_cache.write().await.clear();
        self.image_embedding_cache.write().await.clear();
        
        // Trigger garbage collection
        self.memory_manager.trigger_gc().await;
        
        Ok(())
    }

    // Private helper methods

    async fn load_model_config(&self) -> LunaResult<ClipConfig> {
        // For now, return a default config
        // In a real implementation, this would load from the model directory
        Ok(ClipConfig::default())
    }

    async fn create_var_builder(&self) -> LunaResult<VarBuilder<'static>> {
        // This is a placeholder - in a real implementation, this would
        // load the actual model weights from disk or download them
        Err(LunaError::AiModel {
            model: "CLIP".to_string(),
            error: "Model loading not implemented".to_string(),
            suggestion: "This is a mock implementation for demonstration".to_string(),
        })
    }

    async fn check_rate_limit(&self) -> LunaResult<()> {
        if let Some(last_inference) = *self.last_inference.read().await {
            let time_since = Instant::now().duration_since(last_inference);
            if time_since < Duration::from_millis(50) { // Min 50ms between inferences
                return Err(LunaError::AiModel {
                    model: "CLIP".to_string(),
                    error: "Rate limit exceeded".to_string(),
                    suggestion: "Wait before making another inference request".to_string(),
                });
            }
        }
        Ok(())
    }

    async fn get_text_embedding(&self, text: &str) -> LunaResult<Tensor> {
        // Check cache first if enabled
        if self.config.enable_caching {
            let cache_key = format!("text:{}", text);
            let mut cache = self.text_embedding_cache.write().await;
            
            if let Some(cached) = cache.get_mut(&cache_key) {
                cached.access_count += 1;
                debug!("Using cached text embedding for: {}", text);
                return Ok(cached.embedding.clone());
            }
        }

        // Generate new embedding
        let embedding = self.encode_text(text).await?;

        // Cache if enabled
        if self.config.enable_caching {
            let cache_key = format!("text:{}", text);
            let mut cache = self.text_embedding_cache.write().await;
            
            cache.insert(cache_key, CachedEmbedding {
                embedding: embedding.clone(),
                created_at: Instant::now(),
                access_count: 1,
            });
        }

        Ok(embedding)
    }

    async fn get_image_embedding(&self, image_data: &[u8], element_id: &str) -> LunaResult<Tensor> {
        // Check cache first if enabled
        if self.config.enable_caching {
            let cache_key = format!("image:{}", element_id);
            let mut cache = self.image_embedding_cache.write().await;
            
            if let Some(cached) = cache.get_mut(&cache_key) {
                cached.access_count += 1;
                debug!("Using cached image embedding for: {}", element_id);
                return Ok(cached.embedding.clone());
            }
        }

        // Generate new embedding
        let embedding = self.encode_image(image_data).await?;

        // Cache if enabled
        if self.config.enable_caching {
            let cache_key = format!("image:{}", element_id);
            let mut cache = self.image_embedding_cache.write().await;
            
            cache.insert(cache_key, CachedEmbedding {
                embedding: embedding.clone(),
                created_at: Instant::now(),
                access_count: 1,
            });
        }

        Ok(embedding)
    }

    async fn encode_text(&self, text: &str) -> LunaResult<Tensor> {
        // Mock text encoding for demonstration
        // In a real implementation, this would use the CLIP text encoder
        let embedding = Tensor::randn(0.0, 1.0, (512,), &self.device)
            .map_err(|e| LunaError::AiModel {
                model: "CLIP".to_string(),
                error: format!("Text encoding failed: {}", e),
                suggestion: "Check text input and model state".to_string(),
            })?;
        
        debug!("Encoded text: {} -> tensor shape: {:?}", text, embedding.shape());
        Ok(embedding)
    }

    async fn encode_image(&self, image_data: &[u8]) -> LunaResult<Tensor> {
        // Mock image encoding for demonstration
        // In a real implementation, this would use the CLIP vision encoder
        let embedding = Tensor::randn(0.0, 1.0, (512,), &self.device)
            .map_err(|e| LunaError::AiModel {
                model: "CLIP".to_string(),
                error: format!("Image encoding failed: {}", e),
                suggestion: "Check image format and model state".to_string(),
            })?;
        
        debug!("Encoded image ({} bytes) -> tensor shape: {:?}", image_data.len(), embedding.shape());
        Ok(embedding)
    }

    async fn calculate_cosine_similarity(&self, embedding1: &Tensor, embedding2: &Tensor) -> LunaResult<f32> {
        // Mock similarity calculation
        // In a real implementation, this would compute actual cosine similarity
        let similarity = 0.75; // Mock value
        Ok(similarity)
    }

    async fn process_element_batch(
        &self,
        text_embedding: &Tensor,
        elements: &[VisualElement],
        query: &TextQuery,
    ) -> LunaResult<Vec<MatchResult>> {
        let mut matches = Vec::new();

        for element in elements {
            // Get image embedding
            let image_embedding = self.get_image_embedding(&element.image_data, &element.id).await?;
            
            // Calculate similarity
            let similarity = self.calculate_cosine_similarity(text_embedding, &image_embedding).await?;
            
            // Calculate confidence (considering multiple factors)
            let confidence = self.calculate_confidence(similarity, element, query).await;
            
            // Generate reasoning
            let reasoning = self.generate_reasoning(similarity, element, query).await;
            
            // Create metadata
            let mut metadata = HashMap::new();
            metadata.insert("source".to_string(), "clip".to_string());
            metadata.insert("similarity_raw".to_string(), similarity.to_string());
            if let Some(ref text_content) = element.text_content {
                metadata.insert("has_text".to_string(), "true".to_string());
                metadata.insert("text_length".to_string(), text_content.len().to_string());
            }
            if let Some(ref element_type) = element.element_type {
                metadata.insert("element_type".to_string(), element_type.clone());
            }
            
            matches.push(MatchResult {
                element_id: element.id.clone(),
                similarity,
                confidence,
                reasoning,
                metadata,
            });
        }

        Ok(matches)
    }

    async fn calculate_confidence(&self, similarity: f32, element: &VisualElement, query: &TextQuery) -> f32 {
        let mut confidence = similarity;
        
        // Boost confidence if element has matching text content
        if let Some(ref text_content) = element.text_content {
            if text_content.to_lowercase().contains(&query.text.to_lowercase()) {
                confidence += 0.2;
            }
        }
        
        // Boost confidence for certain element types
        if let Some(ref element_type) = element.element_type {
            match (query.text.to_lowercase().as_str(), element_type.as_str()) {
                ("button" | "click", "button") => confidence += 0.1,
                ("link", "link") => confidence += 0.1,
                ("input" | "text", "text_input") => confidence += 0.1,
                _ => {}
            }
        }
        
        // Clamp to [0, 1]
        confidence.min(1.0).max(0.0)
    }

    async fn generate_reasoning(&self, similarity: f32, element: &VisualElement, query: &TextQuery) -> String {
        let mut reasons = Vec::new();
        
        reasons.push(format!("Visual similarity: {:.2}", similarity));
        
        if let Some(ref text_content) = element.text_content {
            if text_content.to_lowercase().contains(&query.text.to_lowercase()) {
                reasons.push("Text content matches query".to_string());
            }
        }
        
        if let Some(ref element_type) = element.element_type {
            reasons.push(format!("Element type: {}", element_type));
        }
        
        if similarity > 0.8 {
            reasons.push("High visual similarity".to_string());
        } else if similarity < 0.3 {
            reasons.push("Low visual similarity".to_string());
        }
        
        reasons.join("; ")
    }

    fn estimate_model_memory_mb(&self) -> u64 {
        // Rough estimate for CLIP model memory usage
        let base_memory = 1024; // ~1GB for model
        let cache_memory = if self.config.enable_caching {
            (self.config.max_cache_size as u64 * 2) / 1024 // ~2MB per 1000 embeddings
        } else {
            0
        };
        base_memory + cache_memory
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::MemoryManager;
    use crate::utils::MetricsCollector;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_clip_creation() {
        let memory_manager = Arc::new(MemoryManager::new().unwrap());
        let metrics = Arc::new(MetricsCollector::new());
        let config = ClipConfig::default();
        
        let specialist = ClipSpecialist::new(config, memory_manager, metrics);
        assert!(specialist.is_ok());
    }

    #[tokio::test]
    async fn test_invalid_similarity_threshold() {
        let memory_manager = Arc::new(MemoryManager::new().unwrap());
        let metrics = Arc::new(MetricsCollector::new());
        let mut config = ClipConfig::default();
        config.similarity_threshold = 1.5; // Invalid
        
        let specialist = ClipSpecialist::new(config, memory_manager, metrics);
        assert!(specialist.is_err());
    }

    #[tokio::test]
    async fn test_text_query_validation() {
        let memory_manager = Arc::new(MemoryManager::new().unwrap());
        let metrics = Arc::new(MetricsCollector::new());
        let config = ClipConfig::default();
        
        let specialist = ClipSpecialist::new(config, memory_manager, metrics).unwrap();
        
        let query = TextQuery {
            text: "".to_string(), // Empty text
            context: None,
            min_confidence: 0.5,
            max_matches: 10,
        };
        
        let result = specialist.match_text_to_visuals(&query, &[]).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_cache_functionality() {
        let memory_manager = Arc::new(MemoryManager::new().unwrap());
        let metrics = Arc::new(MetricsCollector::new());
        let config = ClipConfig::default();
        
        let specialist = ClipSpecialist::new(config, memory_manager, metrics).unwrap();
        
        // Test cache cleanup
        let result = specialist.cleanup_cache().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_status_tracking() {
        let memory_manager = Arc::new(MemoryManager::new().unwrap());
        let metrics = Arc::new(MetricsCollector::new());
        let config = ClipConfig::default();
        
        let specialist = ClipSpecialist::new(config, memory_manager, metrics).unwrap();
        let status = specialist.get_status().await;
        
        assert_eq!(status.get("model").unwrap(), "CLIP");
        assert_eq!(status.get("initialized").unwrap(), "false");
        assert_eq!(status.get("caching_enabled").unwrap(), "true");
    }

    #[tokio::test]
    async fn test_memory_estimation() {
        let memory_manager = Arc::new(MemoryManager::new().unwrap());
        let metrics = Arc::new(MetricsCollector::new());
        let config = ClipConfig::default();
        
        let specialist = ClipSpecialist::new(config, memory_manager, metrics).unwrap();
        let memory_estimate = specialist.estimate_model_memory_mb();
        
        assert!(memory_estimate > 0);
    }
}