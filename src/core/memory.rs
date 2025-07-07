/*!
 * Luna Memory Manager - Simple memory monitoring for portable app
 */

use std::sync::Arc;
use parking_lot::RwLock;
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use tracing::{debug, warn};

/// Memory usage tracker
#[derive(Debug, Clone)]
pub struct MemoryUsage {
    pub total_bytes: u64,
    pub heap_bytes: u64,
    pub timestamp: DateTime<Utc>,
}

/// Memory pool for reusable objects
#[derive(Debug)]
pub struct MemoryPool<T> {
    items: Vec<T>,
    capacity: usize,
}

impl<T> MemoryPool<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            items: Vec::with_capacity(capacity),
            capacity,
        }
    }
    
    pub fn get(&mut self) -> Option<T> {
        self.items.pop()
    }
    
    pub fn return_item(&mut self, item: T) {
        if self.items.len() < self.capacity {
            self.items.push(item);
        }
        // If pool is full, item will be dropped
    }
    
    pub fn len(&self) -> usize {
        self.items.len()
    }
    
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

/// Simple memory manager for Luna
#[derive(Debug)]
pub struct MemoryManager {
    /// Memory usage history
    usage_history: Arc<RwLock<Vec<MemoryUsage>>>,
    /// Pool for reusable image buffers
    image_pool: Arc<RwLock<MemoryPool<Vec<u8>>>>,
    /// Pool for reusable string buffers
    string_pool: Arc<RwLock<MemoryPool<String>>>,
    /// Memory-mapped file cache
    cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
    /// Maximum memory usage in bytes
    max_memory_bytes: u64,
    /// Maximum cache entries
    max_cache_entries: usize,
}

#[derive(Debug, Clone)]
struct CacheEntry {
    data: Vec<u8>,
    last_accessed: DateTime<Utc>,
    access_count: u64,
}

impl MemoryManager {
    /// Create new memory manager
    pub fn new() -> Self {
        Self {
            usage_history: Arc::new(RwLock::new(Vec::new())),
            image_pool: Arc::new(RwLock::new(MemoryPool::new(10))),
            string_pool: Arc::new(RwLock::new(MemoryPool::new(50))),
            cache: Arc::new(RwLock::new(HashMap::new())),
            max_memory_bytes: 512 * 1024 * 1024, // 512MB default
            max_cache_entries: 100,
        }
    }
    
    /// Set memory limits
    pub fn set_limits(&mut self, max_memory_mb: u64, max_cache_entries: usize) {
        self.max_memory_bytes = max_memory_mb * 1024 * 1024;
        self.max_cache_entries = max_cache_entries;
    }
    
    /// Get current memory usage
    pub fn get_usage(&self) -> u64 {
        // Simple implementation - in a real app you'd use proper memory tracking
        let cache = self.cache.read();
        let cache_size: u64 = cache.values()
            .map(|entry| entry.data.len() as u64)
            .sum();
        
        cache_size
    }
    
    /// Record memory usage sample
    pub fn record_usage(&self, total_bytes: u64, heap_bytes: u64) {
        let usage = MemoryUsage {
            total_bytes,
            heap_bytes,
            timestamp: Utc::now(),
        };
        
        let mut history = self.usage_history.write();
        history.push(usage);
        
        // Keep only last 100 samples
        if history.len() > 100 {
            history.remove(0);
        }
        
        // Warn if memory usage is high
        if total_bytes > self.max_memory_bytes * 8 / 10 { // 80% threshold
            warn!("High memory usage: {}MB", total_bytes / 1024 / 1024);
        }
    }
    
    /// Get memory usage history
    pub fn get_usage_history(&self) -> Vec<MemoryUsage> {
        let history = self.usage_history.read();
        history.clone()
    }
    
    /// Get reusable image buffer
    pub fn get_image_buffer(&self, size: usize) -> Vec<u8> {
        let mut pool = self.image_pool.write();
        if let Some(mut buffer) = pool.get() {
            buffer.clear();
            buffer.reserve(size);
            buffer
        } else {
            Vec::with_capacity(size)
        }
    }
    
    /// Return image buffer to pool
    pub fn return_image_buffer(&self, buffer: Vec<u8>) {
        let mut pool = self.image_pool.write();
        pool.return_item(buffer);
    }
    
    /// Get reusable string buffer
    pub fn get_string_buffer(&self) -> String {
        let mut pool = self.string_pool.write();
        if let Some(mut string) = pool.get() {
            string.clear();
            string
        } else {
            String::new()
        }
    }
    
    /// Return string buffer to pool
    pub fn return_string_buffer(&self, string: String) {
        let mut pool = self.string_pool.write();
        pool.return_item(string);
    }
    
    /// Cache data with key
    pub fn cache_data(&self, key: String, data: Vec<u8>) {
        let mut cache = self.cache.write();
        
        // Clean up old entries if cache is full
        if cache.len() >= self.max_cache_entries {
            self.cleanup_cache(&mut cache);
        }
        
        let entry = CacheEntry {
            data,
            last_accessed: Utc::now(),
            access_count: 1,
        };
        
        cache.insert(key, entry);
        debug!("Cached data, total entries: {}", cache.len());
    }
    
    /// Get cached data
    pub fn get_cached_data(&self, key: &str) -> Option<Vec<u8>> {
        let mut cache = self.cache.write();
        if let Some(entry) = cache.get_mut(key) {
            entry.last_accessed = Utc::now();
            entry.access_count += 1;
            Some(entry.data.clone())
        } else {
            None
        }
    }
    
    /// Remove cached data
    pub fn remove_cached_data(&self, key: &str) {
        let mut cache = self.cache.write();
        cache.remove(key);
    }
    
    /// Clear all cached data
    pub fn clear_cache(&self) {
        let mut cache = self.cache.write();
        cache.clear();
        debug!("Cache cleared");
    }
    
    /// Cleanup old cache entries
    fn cleanup_cache(&self, cache: &mut HashMap<String, CacheEntry>) {
        let now = Utc::now();
        let threshold = chrono::Duration::minutes(10);
        
        // Remove entries older than threshold
        cache.retain(|_key, entry| {
            now.signed_duration_since(entry.last_accessed) < threshold
        });
        
        // If still too many entries, remove least recently used
        if cache.len() >= self.max_cache_entries {
            let mut entries: Vec<_> = cache.iter().collect();
            entries.sort_by_key(|(_, entry)| entry.last_accessed);
            
            // Remove oldest 25%
            let remove_count = cache.len() / 4;
            for (key, _) in entries.iter().take(remove_count) {
                cache.remove(*key);
            }
        }
        
        debug!("Cache cleanup complete, {} entries remaining", cache.len());
    }
    
    /// Get memory statistics
    pub fn get_stats(&self) -> MemoryStats {
        let cache = self.cache.read();
        let image_pool = self.image_pool.read();
        let string_pool = self.string_pool.read();
        let usage_history = self.usage_history.read();
        
        let cache_size: u64 = cache.values()
            .map(|entry| entry.data.len() as u64)
            .sum();
        
        let current_usage = usage_history.last()
            .map(|u| u.total_bytes)
            .unwrap_or(0);
        
        MemoryStats {
            current_usage_bytes: current_usage,
            cache_size_bytes: cache_size,
            cache_entries: cache.len(),
            image_pool_size: image_pool.len(),
            string_pool_size: string_pool.len(),
            max_memory_bytes: self.max_memory_bytes,
            usage_percentage: (current_usage * 100) / self.max_memory_bytes,
        }
    }
    
    /// Force garbage collection if available
    pub fn force_gc(&self) {
        // In Rust, we don't have explicit GC, but we can clear caches
        self.clear_cache();
        debug!("Memory cleanup completed");
    }
}

impl Default for MemoryManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Memory statistics
#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub current_usage_bytes: u64,
    pub cache_size_bytes: u64,
    pub cache_entries: usize,
    pub image_pool_size: usize,
    pub string_pool_size: usize,
    pub max_memory_bytes: u64,
    pub usage_percentage: u64,
}

/// RAII wrapper for pooled resources
pub struct PooledBuffer {
    buffer: Option<Vec<u8>>,
    manager: Arc<MemoryManager>,
}

impl PooledBuffer {
    pub fn new(manager: Arc<MemoryManager>, size: usize) -> Self {
        let buffer = manager.get_image_buffer(size);
        Self {
            buffer: Some(buffer),
            manager,
        }
    }
    
    pub fn as_mut(&mut self) -> &mut Vec<u8> {
        self.buffer.as_mut().unwrap()
    }
    
    pub fn as_ref(&self) -> &Vec<u8> {
        self.buffer.as_ref().unwrap()
    }
}

impl Drop for PooledBuffer {
    fn drop(&mut self) {
        if let Some(buffer) = self.buffer.take() {
            self.manager.return_image_buffer(buffer);
        }
    }
}