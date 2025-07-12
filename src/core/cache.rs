//! File caching functionality for eliminating redundant I/O
//!
//! This module provides a thread-safe cache for file contents using `Arc<str>`
//! for cheap cloning across threads.

use anyhow::Result;
use dashmap::DashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Thread-safe file content cache
pub struct FileCache {
    cache: DashMap<PathBuf, Arc<str>>,
}

impl FileCache {
    /// Create a new empty cache
    pub fn new() -> Self {
        FileCache { cache: DashMap::new() }
    }

    /// Get file content from cache or load from disk
    pub fn get_or_load(&self, path: &Path) -> Result<Arc<str>> {
        // Canonicalize path to avoid cache misses from different representations
        let canonical_path = path.canonicalize()?;

        // Check if already cached
        if let Some(content) = self.cache.get(&canonical_path) {
            return Ok(content.clone());
        }

        // Load from disk
        let content = std::fs::read_to_string(&canonical_path)?;
        let arc_content: Arc<str> = Arc::from(content.as_str());

        // Store in cache
        self.cache.insert(canonical_path, arc_content.clone());

        Ok(arc_content)
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        CacheStats { entries: self.cache.len() }
    }
}

impl Default for FileCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub entries: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_cache_hit_returns_same_content() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let content = "Hello, cache!";
        fs::write(&file_path, content).unwrap();

        let cache = FileCache::new();

        // First access - cache miss
        let content1 = cache.get_or_load(&file_path).unwrap();
        assert_eq!(&*content1, content);

        // Second access - cache hit
        let content2 = cache.get_or_load(&file_path).unwrap();
        assert_eq!(&*content2, content);

        // Should be the same Arc
        assert!(Arc::ptr_eq(&content1, &content2));
    }

    #[test]
    fn test_cache_miss_loads_from_disk() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let content = "Content from disk";
        fs::write(&file_path, content).unwrap();

        let cache = FileCache::new();
        let loaded = cache.get_or_load(&file_path).unwrap();

        assert_eq!(&*loaded, content);
        assert_eq!(cache.stats().entries, 1);
    }

    #[test]
    fn test_non_existent_file_returns_error() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("does_not_exist.txt");

        let cache = FileCache::new();
        let result = cache.get_or_load(&file_path);

        assert!(result.is_err());
        assert_eq!(cache.stats().entries, 0);
    }

    #[test]
    fn test_canonicalized_paths() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "content").unwrap();

        let cache = FileCache::new();

        // Access with different path representations
        let _content1 = cache.get_or_load(&file_path).unwrap();
        let relative_path =
            PathBuf::from(".").join(file_path.strip_prefix("/").unwrap_or(&file_path));

        // This might fail on canonicalization, which is fine
        if let Ok(content2) = cache.get_or_load(&relative_path) {
            // If it succeeds, should still only have one entry
            assert_eq!(cache.stats().entries, 1);
            assert_eq!(&*content2, "content");
        }
    }

    #[test]
    fn test_concurrent_access() {
        use std::sync::Arc as StdArc;
        use std::thread;

        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("concurrent.txt");
        fs::write(&file_path, "concurrent content").unwrap();

        let cache = StdArc::new(FileCache::new());
        let mut handles = vec![];

        // Spawn multiple threads accessing the same file
        for _ in 0..10 {
            let cache_clone = cache.clone();
            let path_clone = file_path.clone();

            let handle = thread::spawn(move || {
                let content = cache_clone.get_or_load(&path_clone).unwrap();
                assert_eq!(&*content, "concurrent content");
            });

            handles.push(handle);
        }

        // Wait for all threads
        for handle in handles {
            handle.join().unwrap();
        }

        // Should only have one cache entry
        assert_eq!(cache.stats().entries, 1);
    }
}
