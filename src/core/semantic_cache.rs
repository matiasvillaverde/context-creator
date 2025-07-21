//! Semantic analysis caching to avoid redundant parsing
//!
//! This module provides a thread-safe cache for semantic analysis results,
//! keyed by file path and content hash to ensure cache invalidation on changes.

use crate::core::semantic::analyzer::AnalysisResult;
use dashmap::DashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Cache key combining file path and content hash
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct CacheKey {
    path: PathBuf,
    content_hash: u64,
}

/// Thread-safe semantic analysis cache
pub struct SemanticCache {
    cache: DashMap<CacheKey, Arc<AnalysisResult>>,
}

impl SemanticCache {
    /// Create a new empty cache
    pub fn new() -> Self {
        SemanticCache {
            cache: DashMap::new(),
        }
    }

    /// Get cached analysis result or None if not cached
    pub fn get(&self, path: &Path, content_hash: u64) -> Option<Arc<AnalysisResult>> {
        let key = CacheKey {
            path: path.to_path_buf(),
            content_hash,
        };
        self.cache.get(&key).map(|entry| entry.clone())
    }

    /// Store analysis result in cache
    pub fn insert(&self, path: &Path, content_hash: u64, result: AnalysisResult) {
        let key = CacheKey {
            path: path.to_path_buf(),
            content_hash,
        };
        self.cache.insert(key, Arc::new(result));
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            entries: self.cache.len(),
        }
    }

    /// Clear all cache entries
    pub fn clear(&self) {
        self.cache.clear();
    }
}

impl Default for SemanticCache {
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
    use crate::core::semantic::analyzer::Import;

    #[test]
    fn test_cache_hit_returns_same_result() {
        let cache = SemanticCache::new();
        let path = PathBuf::from("/test/file.rs");
        let content_hash = 12345u64;

        let result = AnalysisResult {
            imports: vec![Import {
                module: "std::collections".to_string(),
                items: vec!["HashMap".to_string()],
                is_relative: false,
                line: 1,
            }],
            function_calls: vec![],
            type_references: vec![],
            exported_functions: vec![],
            errors: vec![],
        };

        // Store in cache
        cache.insert(&path, content_hash, result);

        // Retrieve from cache
        let cached = cache.get(&path, content_hash).unwrap();
        assert_eq!(cached.imports.len(), 1);
        assert_eq!(cached.imports[0].module, "std::collections");
    }

    #[test]
    fn test_cache_miss_on_different_hash() {
        let cache = SemanticCache::new();
        let path = PathBuf::from("/test/file.rs");

        let result = AnalysisResult::default();
        cache.insert(&path, 12345, result);

        // Different hash should miss
        assert!(cache.get(&path, 67890).is_none());
    }

    #[test]
    fn test_cache_miss_on_different_path() {
        let cache = SemanticCache::new();
        let path1 = PathBuf::from("/test/file1.rs");
        let path2 = PathBuf::from("/test/file2.rs");

        let result = AnalysisResult::default();
        cache.insert(&path1, 12345, result);

        // Different path should miss
        assert!(cache.get(&path2, 12345).is_none());
    }

    #[test]
    fn test_cache_stats() {
        let cache = SemanticCache::new();
        assert_eq!(cache.stats().entries, 0);

        cache.insert(&PathBuf::from("/test1.rs"), 111, AnalysisResult::default());
        cache.insert(&PathBuf::from("/test2.rs"), 222, AnalysisResult::default());

        assert_eq!(cache.stats().entries, 2);
    }

    #[test]
    fn test_cache_clear() {
        let cache = SemanticCache::new();

        cache.insert(&PathBuf::from("/test.rs"), 123, AnalysisResult::default());
        assert_eq!(cache.stats().entries, 1);

        cache.clear();
        assert_eq!(cache.stats().entries, 0);
    }
}
