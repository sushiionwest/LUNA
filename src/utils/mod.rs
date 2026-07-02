// Utility functions with minimal dependencies
// Replaces external utility crates with standard library implementations

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub mod logging;
pub mod geometry;
pub mod image_processing;

// Simple error type for utility functions
#[derive(Debug)]
pub enum UtilError {
    IoError(std::io::Error),
    ParseError(String),
    InvalidInput(String),
}

impl From<std::io::Error> for UtilError {
    fn from(error: std::io::Error) -> Self {
        UtilError::IoError(error)
    }
}

impl std::fmt::Display for UtilError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UtilError::IoError(e) => write!(f, "IO error: {}", e),
            UtilError::ParseError(e) => write!(f, "Parse error: {}", e),
            UtilError::InvalidInput(e) => write!(f, "Invalid input: {}", e),
        }
    }
}

impl std::error::Error for UtilError {}

// Configuration management without external config crates
pub struct ConfigManager {
    settings: HashMap<String, String>,
    config_path: PathBuf,
}

impl ConfigManager {
    pub fn new(config_path: impl AsRef<Path>) -> Result<Self, UtilError> {
        let config_path = config_path.as_ref().to_path_buf();
        let mut manager = Self {
            settings: HashMap::new(),
            config_path,
        };
        manager.load_config()?;
        Ok(manager)
    }

    pub fn load_config(&mut self) -> Result<(), UtilError> {
        if !self.config_path.exists() {
            return Ok(()); // No config file yet
        }

        let content = fs::read_to_string(&self.config_path)?;
        for line in content.lines() {
            if let Some((key, value)) = parse_key_value(line) {
                self.settings.insert(key, value);
            }
        }
        Ok(())
    }

    pub fn save_config(&self) -> Result<(), UtilError> {
        let mut content = String::new();
        for (key, value) in &self.settings {
            content.push_str(&format!("{}={}\n", key, value));
        }
        fs::write(&self.config_path, content)?;
        Ok(())
    }

    pub fn get(&self, key: &str) -> Option<&String> {
        self.settings.get(key)
    }

    pub fn set(&mut self, key: String, value: String) {
        self.settings.insert(key, value);
    }

    pub fn get_bool(&self, key: &str, default: bool) -> bool {
        self.settings.get(key)
            .and_then(|v| v.parse().ok())
            .unwrap_or(default)
    }

    pub fn get_int(&self, key: &str, default: i32) -> i32 {
        self.settings.get(key)
            .and_then(|v| v.parse().ok())
            .unwrap_or(default)
    }
}

fn parse_key_value(line: &str) -> Option<(String, String)> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    
    if let Some(pos) = line.find('=') {
        let key = line[..pos].trim().to_string();
        let value = line[pos + 1..].trim().to_string();
        Some((key, value))
    } else {
        None
    }
}

// Timer utilities without external timing crates
pub struct Timer {
    start_time: SystemTime,
    name: String,
}

impl Timer {
    pub fn new(name: &str) -> Self {
        Self {
            start_time: SystemTime::now(),
            name: name.to_string(),
        }
    }

    pub fn elapsed_ms(&self) -> u64 {
        self.start_time
            .elapsed()
            .unwrap_or_default()
            .as_millis() as u64
    }

    pub fn elapsed_seconds(&self) -> f64 {
        self.start_time
            .elapsed()
            .unwrap_or_default()
            .as_secs_f64()
    }

    pub fn reset(&mut self) {
        self.start_time = SystemTime::now();
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        println!("Timer '{}' finished in {:.2}ms", self.name, self.elapsed_ms());
    }
}

// Simple cache implementation without external caching crates
pub struct SimpleCache<K, V> {
    data: HashMap<K, CacheEntry<V>>,
    max_size: usize,
    ttl_seconds: u64,
}

struct CacheEntry<V> {
    value: V,
    timestamp: u64,
}

impl<K: std::hash::Hash + Eq + Clone, V: Clone> SimpleCache<K, V> {
    pub fn new(max_size: usize, ttl_seconds: u64) -> Self {
        Self {
            data: HashMap::new(),
            max_size,
            ttl_seconds,
        }
    }

    pub fn get(&mut self, key: &K) -> Option<V> {
        let now = current_timestamp();
        
        if let Some(entry) = self.data.get(key) {
            if now - entry.timestamp < self.ttl_seconds {
                return Some(entry.value.clone());
            } else {
                self.data.remove(key);
            }
        }
        None
    }

    pub fn set(&mut self, key: K, value: V) {
        let now = current_timestamp();
        
        // Remove expired entries
        self.cleanup_expired();
        
        // If at capacity, remove oldest
        if self.data.len() >= self.max_size {
            if let Some(oldest_key) = self.find_oldest_key() {
                self.data.remove(&oldest_key);
            }
        }
        
        self.data.insert(key, CacheEntry {
            value,
            timestamp: now,
        });
    }

    fn cleanup_expired(&mut self) {
        let now = current_timestamp();
        self.data.retain(|_, entry| now - entry.timestamp < self.ttl_seconds);
    }

    fn find_oldest_key(&self) -> Option<K> {
        self.data.iter()
            .min_by_key(|(_, entry)| entry.timestamp)
            .map(|(key, _)| key.clone())
    }
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

// String utilities without external string processing crates
pub fn sanitize_filename(filename: &str) -> String {
    filename.chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect()
}

pub fn truncate_string(s: &str, max_length: usize) -> String {
    if s.len() <= max_length {
        s.to_string()
    } else {
        format!("{}...", &s[..max_length.saturating_sub(3)])
    }
}

pub fn escape_regex(text: &str) -> String {
    let mut result = String::with_capacity(text.len() * 2);
    for c in text.chars() {
        match c {
            '\\' | '^' | '$' | '.' | '[' | ']' | '|' | '(' | ')' | '?' | '*' | '+' | '{' | '}' => {
                result.push('\\');
                result.push(c);
            }
            _ => result.push(c),
        }
    }
    result
}

// File utilities without external file processing crates
pub fn ensure_directory_exists(path: impl AsRef<Path>) -> Result<(), UtilError> {
    let path = path.as_ref();
    if !path.exists() {
        fs::create_dir_all(path)?;
    }
    Ok(())
}

pub fn get_file_size(path: impl AsRef<Path>) -> Result<u64, UtilError> {
    let metadata = fs::metadata(path)?;
    Ok(metadata.len())
}

pub fn list_files_with_extension(dir: impl AsRef<Path>, extension: &str) -> Result<Vec<PathBuf>, UtilError> {
    let mut files = Vec::new();
    let entries = fs::read_dir(dir)?;
    
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        
        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext.to_string_lossy().to_lowercase() == extension.to_lowercase() {
                    files.push(path);
                }
            }
        }
    }
    
    files.sort();
    Ok(files)
}

// Performance monitoring without external profiling crates
pub struct PerformanceMonitor {
    measurements: HashMap<String, Vec<u64>>,
}

impl PerformanceMonitor {
    pub fn new() -> Self {
        Self {
            measurements: HashMap::new(),
        }
    }

    pub fn measure<F, R>(&mut self, name: &str, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let start = SystemTime::now();
        let result = f();
        let duration = start.elapsed().unwrap_or_default().as_millis() as u64;
        
        self.measurements.entry(name.to_string())
            .or_insert_with(Vec::new)
            .push(duration);
        
        result
    }

    pub fn get_average(&self, name: &str) -> Option<f64> {
        self.measurements.get(name).and_then(|measurements| {
            if measurements.is_empty() {
                None
            } else {
                let sum: u64 = measurements.iter().sum();
                Some(sum as f64 / measurements.len() as f64)
            }
        })
    }

    pub fn get_stats(&self, name: &str) -> Option<(f64, u64, u64)> {
        self.measurements.get(name).and_then(|measurements| {
            if measurements.is_empty() {
                None
            } else {
                let sum: u64 = measurements.iter().sum();
                let avg = sum as f64 / measurements.len() as f64;
                let min = *measurements.iter().min().unwrap();
                let max = *measurements.iter().max().unwrap();
                Some((avg, min, max))
            }
        })
    }
}

impl Default for PerformanceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

// Data serialization without external serialization crates
pub fn serialize_simple_map(map: &HashMap<String, String>) -> String {
    map.iter()
        .map(|(k, v)| format!("{}:{}", escape_colon(k), escape_colon(v)))
        .collect::<Vec<_>>()
        .join(";")
}

pub fn deserialize_simple_map(data: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    
    for pair in data.split(';') {
        if let Some(colon_pos) = pair.find(':') {
            let key = unescape_colon(&pair[..colon_pos]);
            let value = unescape_colon(&pair[colon_pos + 1..]);
            map.insert(key, value);
        }
    }
    
    map
}

fn escape_colon(s: &str) -> String {
    s.replace(':', "\\:")
        .replace(';', "\\;")
        .replace('\\', "\\\\")
}

fn unescape_colon(s: &str) -> String {
    s.replace("\\\\", "\x00")
        .replace("\\:", ":")
        .replace("\\;", ";")
        .replace('\x00', "\\")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_config_manager() {
        let temp_path = std::env::temp_dir().join("test_config.txt");
        
        // Create config
        let mut config = ConfigManager::new(&temp_path).unwrap();
        config.set("key1".to_string(), "value1".to_string());
        config.set("key2".to_string(), "123".to_string());
        config.save_config().unwrap();
        
        // Load config
        let config2 = ConfigManager::new(&temp_path).unwrap();
        assert_eq!(config2.get("key1"), Some(&"value1".to_string()));
        assert_eq!(config2.get_int("key2", 0), 123);
        
        // Clean up
        let _ = fs::remove_file(&temp_path);
    }

    #[test]
    fn test_simple_cache() {
        let mut cache = SimpleCache::new(2, 1); // 2 items, 1 second TTL
        
        cache.set("key1", "value1");
        cache.set("key2", "value2");
        
        assert_eq!(cache.get(&"key1"), Some("value1".to_string()));
        assert_eq!(cache.get(&"key2"), Some("value2".to_string()));
        
        // Test capacity limit
        cache.set("key3", "value3");
        assert!(cache.get(&"key1").is_none() || cache.get(&"key2").is_none()); // One should be evicted
    }

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("file<>name.txt"), "file__name.txt");
        assert_eq!(sanitize_filename("normal_file.txt"), "normal_file.txt");
    }

    #[test]
    fn test_serialize_deserialize() {
        let mut original = HashMap::new();
        original.insert("key1".to_string(), "value1".to_string());
        original.insert("key:2".to_string(), "value;2".to_string());
        
        let serialized = serialize_simple_map(&original);
        let deserialized = deserialize_simple_map(&serialized);
        
        assert_eq!(original, deserialized);
    }
}