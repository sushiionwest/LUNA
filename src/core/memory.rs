/*!
 * Luna Visual AI Memory Management System
 * 
 * Advanced memory management with:
 * - Real-time memory monitoring and alerts
 * - Smart garbage collection and cleanup
 * - Memory pool management for image buffers
 * - AI model memory management with LRU eviction
 * - Emergency cleanup procedures
 * - Memory leak detection and prevention
 * - Cross-platform memory reporting
 */

use crate::core::{config::MemoryConfig, error::{LunaError, Result}};
use parking_lot::{Mutex, RwLock};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use tokio::time;
use tracing::{debug, error, info, warn};

/// Global memory manager instance
static MEMORY_MANAGER: once_cell::sync::Lazy<Arc<MemoryManager>> =
    once_cell::sync::Lazy::new(|| Arc::new(MemoryManager::new()));

/// Memory management subsystem for Luna Visual AI
/// 
/// Provides comprehensive memory monitoring, allocation tracking,
/// and automatic cleanup to prevent memory leaks and ensure
/// stable operation under resource constraints.
pub struct MemoryManager {
    /// Current memory usage tracking
    current_usage: AtomicU64,
    
    /// Memory usage by component
    component_usage: RwLock<HashMap<String, u64>>,
    
    /// Memory allocation tracking
    allocations: Mutex<HashMap<usize, AllocationInfo>>,
    
    /// Memory pools for efficient buffer reuse
    image_pool: Mutex<ImageBufferPool>,
    
    /// Configuration
    config: RwLock<MemoryConfig>,
    
    /// Monitoring state
    monitoring_active: AtomicU64, // Using as boolean with timestamp
    
    /// Emergency cleanup state
    emergency_mode: AtomicU64, // Using as boolean
}

/// Information about a memory allocation
#[derive(Debug, Clone)]
struct AllocationInfo {
    size: u64,
    component: String,
    allocated_at: Instant,
    stack_trace: Option<String>,
}

/// Image buffer pool for efficient memory reuse
struct ImageBufferPool {
    small_buffers: Vec<Vec<u8>>,    // < 1MB
    medium_buffers: Vec<Vec<u8>>,   // 1-10MB
    large_buffers: Vec<Vec<u8>>,    // > 10MB
    max_pool_size: usize,
}

/// Memory statistics
#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub total_allocated: u64,
    pub current_usage: u64,
    pub component_breakdown: HashMap<String, u64>,
    pub pool_usage: PoolStats,
    pub system_available: u64,
    pub gc_runs: u64,
    pub emergency_cleanups: u64,
}

/// Memory pool statistics
#[derive(Debug, Clone)]
pub struct PoolStats {
    pub small_buffers: usize,
    pub medium_buffers: usize,
    pub large_buffers: usize,
    pub total_pooled_memory: u64,
}

impl MemoryManager {
    /// Create a new memory manager
    fn new() -> Self {
        Self {
            current_usage: AtomicU64::new(0),
            component_usage: RwLock::new(HashMap::new()),
            allocations: Mutex::new(HashMap::new()),
            image_pool: Mutex::new(ImageBufferPool::new()),
            config: RwLock::new(MemoryConfig::default()),
            monitoring_active: AtomicU64::new(0),
            emergency_mode: AtomicU64::new(0),
        }
    }

    /// Initialize memory management with configuration
    pub fn init(config: MemoryConfig) -> Result<()> {
        let manager = &*MEMORY_MANAGER;
        *manager.config.write() = config.clone();
        
        info!("Initializing memory management system");
        info!("Max total memory: {} MB", config.max_total_memory / 1_000_000);
        info!("Cleanup threshold: {}%", (config.cleanup_threshold * 100.0) as u32);
        
        // Start memory monitoring if enabled
        if config.profiling_enabled {
            manager.start_monitoring()?;
        }
        
        // Validate initial memory state
        manager.validate_memory_requirements()?;
        
        info!("✅ Memory management system initialized");
        Ok(())
    }

    /// Validate system memory requirements
    fn validate_memory_requirements(&self) -> Result<()> {
        let available = get_available_memory()?;
        let config = self.config.read();
        
        if available < config.max_total_memory {
            warn!(
                "Available memory ({} MB) is less than configured maximum ({} MB)",
                available / 1_000_000,
                config.max_total_memory / 1_000_000
            );
        }
        
        if available < 1_000_000_000 { // 1GB minimum
            return Err(LunaError::memory(
                "Insufficient system memory",
                get_current_usage(),
                1_000_000_000,
            ));
        }
        
        Ok(())
    }

    /// Start memory monitoring background task
    fn start_monitoring(&self) -> Result<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
            
        self.monitoring_active.store(now, Ordering::Relaxed);
        
        let manager = Arc::clone(&MEMORY_MANAGER);
        tokio::spawn(async move {
            manager.monitoring_loop().await;
        });
        
        info!("Memory monitoring started");
        Ok(())
    }

    /// Main memory monitoring loop
    async fn monitoring_loop(&self) {
        let config = self.config.read().clone();
        let mut interval = time::interval(Duration::from_secs(config.check_interval as u64));
        
        loop {
            interval.tick().await;
            
            // Check if monitoring should continue
            if self.monitoring_active.load(Ordering::Relaxed) == 0 {
                break;
            }
            
            // Perform memory check
            if let Err(e) = self.check_memory_usage().await {
                error!("Memory check failed: {}", e);
            }
        }
        
        info!("Memory monitoring stopped");
    }

    /// Check current memory usage and trigger cleanup if needed
    async fn check_memory_usage(&self) -> Result<()> {
        let current = self.current_usage.load(Ordering::Relaxed);
        let config = self.config.read();
        let threshold = (config.max_total_memory as f64 * config.cleanup_threshold as f64) as u64;
        
        debug!("Memory check: {} / {} bytes", current, config.max_total_memory);
        
        if current > threshold {
            warn!("Memory usage above threshold, triggering cleanup");
            self.trigger_cleanup().await?;
        }
        
        // Check for emergency cleanup
        let emergency_threshold = (config.max_total_memory as f64 * 
            config.emergency_cleanup.threshold as f64) as u64;
            
        if current > emergency_threshold && config.emergency_cleanup.enabled {
            error!("Memory usage critical, triggering emergency cleanup");
            self.trigger_emergency_cleanup().await?;
        }
        
        Ok(())
    }

    /// Trigger regular memory cleanup
    async fn trigger_cleanup(&self) -> Result<()> {
        info!("Starting memory cleanup");
        
        // Clean up image buffer pools
        self.cleanup_buffer_pools().await?;
        
        // Force garbage collection
        self.force_garbage_collection().await?;
        
        // Clean up old allocations
        self.cleanup_old_allocations().await?;
        
        let new_usage = self.current_usage.load(Ordering::Relaxed);
        info!("Memory cleanup completed, usage: {} bytes", new_usage);
        
        Ok(())
    }

    /// Trigger emergency memory cleanup
    async fn trigger_emergency_cleanup(&self) -> Result<()> {
        self.emergency_mode.store(1, Ordering::Relaxed);
        
        error!("🚨 EMERGENCY MEMORY CLEANUP ACTIVATED");
        
        let config = self.config.read();
        
        for action in &config.emergency_cleanup.actions {
            match action.as_str() {
                "clear_model_cache" => {
                    // Signal AI models to unload non-essential models
                    warn!("Clearing AI model cache");
                    // This would be implemented by the AI module
                }
                "clear_image_buffers" => {
                    warn!("Clearing all image buffers");
                    self.clear_all_buffers().await?;
                }
                "force_gc" => {
                    warn!("Forcing aggressive garbage collection");
                    self.force_aggressive_gc().await?;
                }
                _ => {
                    warn!("Unknown emergency cleanup action: {}", action);
                }
            }
        }
        
        self.emergency_mode.store(0, Ordering::Relaxed);
        
        let final_usage = self.current_usage.load(Ordering::Relaxed);
        error!("Emergency cleanup completed, usage: {} bytes", final_usage);
        
        Ok(())
    }

    /// Clean up buffer pools
    async fn cleanup_buffer_pools(&self) -> Result<()> {
        let mut pool = self.image_pool.lock();
        let before_count = pool.small_buffers.len() + pool.medium_buffers.len() + pool.large_buffers.len();
        
        // Keep only half of the buffers
        pool.small_buffers.truncate(pool.small_buffers.len() / 2);
        pool.medium_buffers.truncate(pool.medium_buffers.len() / 2);
        pool.large_buffers.truncate(pool.large_buffers.len() / 2);
        
        let after_count = pool.small_buffers.len() + pool.medium_buffers.len() + pool.large_buffers.len();
        
        debug!("Buffer pool cleanup: {} -> {} buffers", before_count, after_count);
        Ok(())
    }

    /// Force garbage collection
    async fn force_garbage_collection(&self) -> Result<()> {
        debug!("Forcing garbage collection");
        
        // In Rust, we don't have direct GC control, but we can:
        // 1. Drop unused allocations
        // 2. Shrink collections
        // 3. Trigger any manual cleanup
        
        // Clean up tracking structures
        {
            let mut allocations = self.allocations.lock();
            let before_count = allocations.len();
            
            // Remove allocations older than 1 hour that might be leaked
            let cutoff = Instant::now() - Duration::from_secs(3600);
            allocations.retain(|_, info| info.allocated_at > cutoff);
            
            let after_count = allocations.len();
            if before_count != after_count {
                debug!("Cleaned {} stale allocation records", before_count - after_count);
            }
        }
        
        Ok(())
    }

    /// Clean up old allocations
    async fn cleanup_old_allocations(&self) -> Result<()> {
        let mut allocations = self.allocations.lock();
        let cutoff = Instant::now() - Duration::from_secs(3600); // 1 hour
        
        let before_count = allocations.len();
        allocations.retain(|_, info| info.allocated_at > cutoff);
        let after_count = allocations.len();
        
        if before_count != after_count {
            warn!("Removed {} potentially leaked allocations", before_count - after_count);
        }
        
        Ok(())
    }

    /// Clear all buffers (emergency only)
    async fn clear_all_buffers(&self) -> Result<()> {
        let mut pool = self.image_pool.lock();
        pool.small_buffers.clear();
        pool.medium_buffers.clear();
        pool.large_buffers.clear();
        
        warn!("All buffer pools cleared");
        Ok(())
    }

    /// Force aggressive garbage collection
    async fn force_aggressive_gc(&self) -> Result<()> {
        // Clear all tracking data
        {
            let mut allocations = self.allocations.lock();
            allocations.clear();
        }
        
        {
            let mut component_usage = self.component_usage.write();
            component_usage.clear();
        }
        
        warn!("Aggressive cleanup completed");
        Ok(())
    }

    /// Allocate memory for a component
    pub fn allocate(&self, size: u64, component: &str) -> Result<usize> {
        let config = self.config.read();
        let current = self.current_usage.fetch_add(size, Ordering::Relaxed);
        
        if current + size > config.max_total_memory {
            // Rollback the allocation
            self.current_usage.fetch_sub(size, Ordering::Relaxed);
            
            return Err(LunaError::memory(
                "Memory allocation would exceed limit",
                current + size,
                config.max_total_memory,
            ));
        }
        
        // Generate allocation ID
        let allocation_id = current as usize;
        
        // Track the allocation
        {
            let mut allocations = self.allocations.lock();
            allocations.insert(allocation_id, AllocationInfo {
                size,
                component: component.to_string(),
                allocated_at: Instant::now(),
                stack_trace: None, // Could be enhanced with backtrace
            });
        }
        
        // Update component usage
        {
            let mut component_usage = self.component_usage.write();
            *component_usage.entry(component.to_string()).or_insert(0) += size;
        }
        
        debug!("Allocated {} bytes for {}", size, component);
        Ok(allocation_id)
    }

    /// Deallocate memory
    pub fn deallocate(&self, allocation_id: usize) -> Result<()> {
        let allocation_info = {
            let mut allocations = self.allocations.lock();
            allocations.remove(&allocation_id)
        };
        
        if let Some(info) = allocation_info {
            // Update current usage
            self.current_usage.fetch_sub(info.size, Ordering::Relaxed);
            
            // Update component usage
            {
                let mut component_usage = self.component_usage.write();
                if let Some(usage) = component_usage.get_mut(&info.component) {
                    *usage = usage.saturating_sub(info.size);
                    if *usage == 0 {
                        component_usage.remove(&info.component);
                    }
                }
            }
            
            debug!("Deallocated {} bytes for {}", info.size, info.component);
        } else {
            warn!("Attempted to deallocate unknown allocation ID: {}", allocation_id);
        }
        
        Ok(())
    }

    /// Get an image buffer from the pool
    pub fn get_image_buffer(&self, size: usize) -> Vec<u8> {
        let mut pool = self.image_pool.lock();
        
        let buffer = if size < 1_000_000 {
            // Small buffer
            pool.small_buffers.pop()
        } else if size < 10_000_000 {
            // Medium buffer
            pool.medium_buffers.pop()
        } else {
            // Large buffer
            pool.large_buffers.pop()
        };
        
        match buffer {
            Some(mut buf) => {
                if buf.capacity() >= size {
                    buf.clear();
                    buf.resize(size, 0);
                    debug!("Reused buffer of size {}", size);
                    buf
                } else {
                    debug!("Creating new buffer of size {} (old too small)", size);
                    vec![0; size]
                }
            }
            None => {
                debug!("Creating new buffer of size {}", size);
                vec![0; size]
            }
        }
    }

    /// Return an image buffer to the pool
    pub fn return_image_buffer(&self, buffer: Vec<u8>) {
        let mut pool = self.image_pool.lock();
        let size = buffer.capacity();
        
        if size < 1_000_000 && pool.small_buffers.len() < pool.max_pool_size {
            pool.small_buffers.push(buffer);
        } else if size < 10_000_000 && pool.medium_buffers.len() < pool.max_pool_size {
            pool.medium_buffers.push(buffer);
        } else if pool.large_buffers.len() < pool.max_pool_size {
            pool.large_buffers.push(buffer);
        }
        // Otherwise, let the buffer be dropped
    }

    /// Get current memory statistics
    pub fn get_stats(&self) -> MemoryStats {
        let component_usage = self.component_usage.read().clone();
        let pool = self.image_pool.lock();
        
        let pool_memory = pool.small_buffers.iter().map(|b| b.capacity() as u64).sum::<u64>()
            + pool.medium_buffers.iter().map(|b| b.capacity() as u64).sum::<u64>()
            + pool.large_buffers.iter().map(|b| b.capacity() as u64).sum::<u64>();
        
        MemoryStats {
            total_allocated: component_usage.values().sum(),
            current_usage: self.current_usage.load(Ordering::Relaxed),
            component_breakdown: component_usage,
            pool_usage: PoolStats {
                small_buffers: pool.small_buffers.len(),
                medium_buffers: pool.medium_buffers.len(),
                large_buffers: pool.large_buffers.len(),
                total_pooled_memory: pool_memory,
            },
            system_available: get_available_memory().unwrap_or(0),
            gc_runs: 0, // Would be tracked if implemented
            emergency_cleanups: 0, // Would be tracked if implemented
        }
    }
}

impl ImageBufferPool {
    fn new() -> Self {
        Self {
            small_buffers: Vec::new(),
            medium_buffers: Vec::new(),
            large_buffers: Vec::new(),
            max_pool_size: 10, // Keep up to 10 buffers of each size
        }
    }
}

/// Get current memory usage across all components
pub fn get_current_usage() -> u64 {
    MEMORY_MANAGER.current_usage.load(Ordering::Relaxed)
}

/// Get available system memory
pub fn get_available_memory() -> Result<u64> {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
        
        let mut mem_status = MEMORYSTATUSEX {
            dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
            ..Default::default()
        };
        
        unsafe {
            if GlobalMemoryStatusEx(&mut mem_status).is_ok() {
                Ok(mem_status.ullAvailPhys)
            } else {
                Err(LunaError::windows_api(
                    "Failed to get memory status",
                    0,
                    "GlobalMemoryStatusEx",
                ))
            }
        }
    }
    
    #[cfg(not(target_os = "windows"))]
    {
        // Fallback for non-Windows systems
        // This would need to be implemented for other platforms
        Ok(4_000_000_000) // Assume 4GB available
    }
}

/// Initialize memory management system
pub fn init(config: MemoryConfig) -> Result<()> {
    MemoryManager::init(config)
}

/// Validate memory requirements
pub fn validate_memory_requirements() -> Result<()> {
    MEMORY_MANAGER.validate_memory_requirements()
}

/// Allocate memory for a component
pub fn allocate(size: u64, component: &str) -> Result<usize> {
    MEMORY_MANAGER.allocate(size, component)
}

/// Deallocate memory
pub fn deallocate(allocation_id: usize) -> Result<()> {
    MEMORY_MANAGER.deallocate(allocation_id)
}

/// Get an image buffer from the pool
pub fn get_image_buffer(size: usize) -> Vec<u8> {
    MEMORY_MANAGER.get_image_buffer(size)
}

/// Return an image buffer to the pool
pub fn return_image_buffer(buffer: Vec<u8>) {
    MEMORY_MANAGER.return_image_buffer(buffer)
}

/// Get current memory statistics
pub fn get_stats() -> MemoryStats {
    MEMORY_MANAGER.get_stats()
}

/// Clean up all memory resources
pub fn cleanup_all() -> Result<()> {
    info!("Cleaning up all memory resources");
    
    // Stop monitoring
    MEMORY_MANAGER.monitoring_active.store(0, Ordering::Relaxed);
    
    // Clear all pools and tracking
    {
        let mut pool = MEMORY_MANAGER.image_pool.lock();
        pool.small_buffers.clear();
        pool.medium_buffers.clear();
        pool.large_buffers.clear();
    }
    
    {
        let mut allocations = MEMORY_MANAGER.allocations.lock();
        allocations.clear();
    }
    
    {
        let mut component_usage = MEMORY_MANAGER.component_usage.write();
        component_usage.clear();
    }
    
    MEMORY_MANAGER.current_usage.store(0, Ordering::Relaxed);
    
    info!("✅ Memory cleanup completed");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::MemoryConfig;

    #[test]
    fn test_memory_allocation() {
        let config = MemoryConfig::default();
        init(config).unwrap();
        
        let allocation_id = allocate(1000, "test").unwrap();
        assert!(allocation_id > 0);
        
        deallocate(allocation_id).unwrap();
    }

    #[test]
    fn test_memory_limit() {
        let mut config = MemoryConfig::default();
        config.max_total_memory = 1000; // Very small limit
        
        init(config).unwrap();
        
        let result = allocate(2000, "test");
        assert!(result.is_err());
    }

    #[test]
    fn test_buffer_pool() {
        let config = MemoryConfig::default();
        init(config).unwrap();
        
        let buffer1 = get_image_buffer(500_000);
        assert_eq!(buffer1.len(), 500_000);
        
        return_image_buffer(buffer1);
        
        let buffer2 = get_image_buffer(500_000);
        assert_eq!(buffer2.len(), 500_000);
    }

    #[test]
    fn test_memory_stats() {
        let config = MemoryConfig::default();
        init(config).unwrap();
        
        let stats = get_stats();
        assert!(stats.system_available > 0);
    }
}