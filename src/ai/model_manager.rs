/*!
 * Luna Model Manager - Handles loading and managing AI models
 * 
 * For the portable executable, models are embedded or downloaded on first use
 */

use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::RwLock;
use tracing::{info, debug, warn};

/// Model metadata
#[derive(Debug, Clone)]
pub struct ModelMetadata {
    pub name: String,
    pub version: String,
    pub size_bytes: u64,
    pub checksum: String,
    pub capabilities: Vec<String>,
    pub loaded: bool,
}

/// Model loading status
#[derive(Debug, Clone)]
pub enum ModelStatus {
    NotLoaded,
    Loading,
    Loaded,
    Failed(String),
}

/// Manages AI model loading and caching
pub struct ModelManager {
    /// Loaded models cache
    models: Arc<RwLock<HashMap<String, ModelStatus>>>,
    /// Model metadata
    metadata: HashMap<String, ModelMetadata>,
    /// Model storage directory
    model_dir: PathBuf,
    /// Maximum total model size in memory
    max_memory_mb: u64,
}

impl ModelManager {
    /// Create new model manager
    pub async fn new() -> Result<Self> {
        info!("Initializing model manager...");
        
        let model_dir = Self::get_model_directory();
        std::fs::create_dir_all(&model_dir)?;
        
        let mut manager = Self {
            models: Arc::new(RwLock::new(HashMap::new())),
            metadata: HashMap::new(),
            model_dir,
            max_memory_mb: 512, // 512MB default limit for portable app
        };
        
        // Initialize model metadata
        manager.init_metadata();
        
        info!("Model manager initialized");
        Ok(manager)
    }
    
    fn init_metadata(&mut self) {
        // CLIP model metadata
        self.metadata.insert("clip".to_string(), ModelMetadata {
            name: "CLIP".to_string(),
            version: "1.0".to_string(),
            size_bytes: 150 * 1024 * 1024, // 150MB
            checksum: "clip_embedded".to_string(),
            capabilities: vec!["vision".to_string(), "text".to_string()],
            loaded: false,
        });
        
        // Florence-2 model metadata
        self.metadata.insert("florence".to_string(), ModelMetadata {
            name: "Florence-2".to_string(),
            version: "1.0".to_string(),
            size_bytes: 280 * 1024 * 1024, // 280MB
            checksum: "florence_embedded".to_string(),
            capabilities: vec!["detailed_vision".to_string(), "ocr".to_string()],
            loaded: false,
        });
        
        // TrOCR model metadata
        self.metadata.insert("trocr".to_string(), ModelMetadata {
            name: "TrOCR".to_string(),
            version: "1.0".to_string(),
            size_bytes: 120 * 1024 * 1024, // 120MB
            checksum: "trocr_embedded".to_string(),
            capabilities: vec!["ocr".to_string()],
            loaded: false,
        });
    }
    
    /// Get model directory for this instance
    fn get_model_directory() -> PathBuf {
        // For portable app, use a subdirectory next to the executable
        let exe_dir = std::env::current_exe()
            .map(|p| p.parent().unwrap_or(&std::path::Path::new(".")).to_path_buf())
            .unwrap_or_else(|_| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        
        exe_dir.join("models")
    }
    
    /// Load a model by name
    pub async fn load_model(&self, model_name: &str) -> Result<()> {
        debug!("Loading model: {}", model_name);
        
        // Check if already loaded
        {
            let models = self.models.read();
            if let Some(status) = models.get(model_name) {
                match status {
                    ModelStatus::Loaded => {
                        debug!("Model {} already loaded", model_name);
                        return Ok(());
                    }
                    ModelStatus::Loading => {
                        return Err(anyhow::anyhow!("Model {} is already loading", model_name));
                    }
                    _ => {}
                }
            }
        }
        
        // Set loading status
        {
            let mut models = self.models.write();
            models.insert(model_name.to_string(), ModelStatus::Loading);
        }
        
        // Get model metadata
        let metadata = self.metadata.get(model_name)
            .ok_or_else(|| anyhow::anyhow!("Unknown model: {}", model_name))?;
        
        // Check memory limits
        if !self.check_memory_limits(&metadata).await {
            let mut models = self.models.write();
            models.insert(model_name.to_string(), ModelStatus::Failed("Memory limit exceeded".to_string()));
            return Err(anyhow::anyhow!("Not enough memory to load model {}", model_name));
        }
        
        // Load the model (simplified for portable app)
        match self.load_model_impl(model_name, metadata).await {
            Ok(()) => {
                let mut models = self.models.write();
                models.insert(model_name.to_string(), ModelStatus::Loaded);
                info!("✅ Model {} loaded successfully", model_name);
                Ok(())
            }
            Err(e) => {
                let mut models = self.models.write();
                models.insert(model_name.to_string(), ModelStatus::Failed(e.to_string()));
                Err(e)
            }
        }
    }
    
    async fn load_model_impl(&self, model_name: &str, metadata: &ModelMetadata) -> Result<()> {
        debug!("Loading model implementation for: {}", model_name);
        
        // For the portable app, we'll use embedded model stubs
        // In a real implementation, this would load actual model files
        
        match model_name {
            "clip" => {
                // Simulate loading time
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                debug!("CLIP model stub loaded");
            }
            "florence" => {
                tokio::time::sleep(tokio::time::Duration::from_millis(750)).await;
                debug!("Florence-2 model stub loaded");
            }
            "trocr" => {
                tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
                debug!("TrOCR model stub loaded");
            }
            _ => {
                return Err(anyhow::anyhow!("Unsupported model: {}", model_name));
            }
        }
        
        Ok(())
    }
    
    async fn check_memory_limits(&self, metadata: &ModelMetadata) -> bool {
        // Simple memory check - in real implementation would check actual memory usage
        let current_memory = self.get_estimated_memory_usage().await;
        let required_memory = metadata.size_bytes / (1024 * 1024); // Convert to MB
        
        current_memory + required_memory <= self.max_memory_mb
    }
    
    async fn get_estimated_memory_usage(&self) -> u64 {
        let models = self.models.read();
        let mut total_mb = 0;
        
        for (model_name, status) in models.iter() {
            if matches!(status, ModelStatus::Loaded) {
                if let Some(metadata) = self.metadata.get(model_name) {
                    total_mb += metadata.size_bytes / (1024 * 1024);
                }
            }
        }
        
        total_mb
    }
    
    /// Unload a model to free memory
    pub async fn unload_model(&self, model_name: &str) -> Result<()> {
        debug!("Unloading model: {}", model_name);
        
        let mut models = self.models.write();
        if let Some(status) = models.get(model_name) {
            match status {
                ModelStatus::Loaded => {
                    models.insert(model_name.to_string(), ModelStatus::NotLoaded);
                    info!("Model {} unloaded", model_name);
                    Ok(())
                }
                _ => {
                    warn!("Model {} was not loaded", model_name);
                    Ok(())
                }
            }
        } else {
            Err(anyhow::anyhow!("Unknown model: {}", model_name))
        }
    }
    
    /// Check if a model is loaded
    pub fn is_model_loaded(&self, model_name: &str) -> bool {
        let models = self.models.read();
        matches!(models.get(model_name), Some(ModelStatus::Loaded))
    }
    
    /// Get model status
    pub fn get_model_status(&self, model_name: &str) -> Option<ModelStatus> {
        let models = self.models.read();
        models.get(model_name).cloned()
    }
    
    /// Get all model statuses
    pub fn get_all_statuses(&self) -> HashMap<String, ModelStatus> {
        let models = self.models.read();
        models.clone()
    }
    
    /// Get model metadata
    pub fn get_metadata(&self, model_name: &str) -> Option<&ModelMetadata> {
        self.metadata.get(model_name)
    }
    
    /// Get all model metadata
    pub fn get_all_metadata(&self) -> &HashMap<String, ModelMetadata> {
        &self.metadata
    }
    
    /// Preload all models
    pub async fn preload_all(&self) -> Result<()> {
        info!("Preloading all models...");
        
        for model_name in self.metadata.keys() {
            match self.load_model(model_name).await {
                Ok(()) => debug!("Preloaded model: {}", model_name),
                Err(e) => warn!("Failed to preload model {}: {}", model_name, e),
            }
        }
        
        info!("Model preloading complete");
        Ok(())
    }
    
    /// Unload all models
    pub async fn unload_all(&self) -> Result<()> {
        info!("Unloading all models...");
        
        for model_name in self.metadata.keys() {
            let _ = self.unload_model(model_name).await;
        }
        
        info!("All models unloaded");
        Ok(())
    }
    
    /// Get memory usage statistics
    pub async fn get_memory_stats(&self) -> MemoryStats {
        let used_mb = self.get_estimated_memory_usage().await;
        let models = self.models.read();
        let loaded_count = models.values()
            .filter(|status| matches!(status, ModelStatus::Loaded))
            .count();
        
        MemoryStats {
            used_mb,
            max_mb: self.max_memory_mb,
            usage_percentage: (used_mb * 100) / self.max_memory_mb,
            loaded_models: loaded_count,
            total_models: self.metadata.len(),
        }
    }
    
    /// Set memory limit
    pub fn set_memory_limit(&mut self, max_mb: u64) {
        self.max_memory_mb = max_mb;
        info!("Memory limit set to {}MB", max_mb);
    }
}

/// Memory usage statistics
#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub used_mb: u64,
    pub max_mb: u64,
    pub usage_percentage: u64,
    pub loaded_models: usize,
    pub total_models: usize,
}

impl MemoryStats {
    pub fn is_memory_critical(&self) -> bool {
        self.usage_percentage > 90
    }
    
    pub fn is_memory_high(&self) -> bool {
        self.usage_percentage > 75
    }
}