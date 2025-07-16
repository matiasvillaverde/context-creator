//! AST cache for semantic analysis

use crate::utils::error::CodeDigestError;
use lru::LruCache;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tree_sitter::{Parser, Tree};

/// Cache key for AST storage
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CacheKey {
    /// Canonical file path
    path: PathBuf,
    /// File content hash for validation
    content_hash: u64,
    /// Language of the file
    language: String,
}

impl CacheKey {
    fn new(path: &Path, content: &str, language: &str) -> Result<Self, CodeDigestError> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let canonical_path = path.canonicalize().map_err(CodeDigestError::IoError)?;

        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        let content_hash = hasher.finish();

        Ok(Self {
            path: canonical_path,
            content_hash,
            language: language.to_string(),
        })
    }
}

/// Cached AST entry
struct CacheEntry {
    /// Parsed syntax tree
    tree: Tree,
    /// Source content (needed for tree-sitter queries)
    content: String,
}

/// Thread-safe LRU cache for parsed ASTs
pub struct AstCache {
    cache: Mutex<LruCache<CacheKey, CacheEntry>>,
}

impl AstCache {
    /// Create a new AST cache with the specified capacity
    pub fn new(capacity: usize) -> Self {
        let capacity = NonZeroUsize::new(capacity).unwrap_or(NonZeroUsize::new(100).unwrap());
        Self {
            cache: Mutex::new(LruCache::new(capacity)),
        }
    }

    /// Get or parse an AST for the given file
    pub fn get_or_parse(
        &self,
        path: &Path,
        content: &str,
        language: &str,
        mut parser: Parser,
    ) -> Result<(Tree, String), CodeDigestError> {
        let key = CacheKey::new(path, content, language)?;

        let mut cache = self
            .cache
            .lock()
            .map_err(|_| CodeDigestError::MutexPoisoned)?;

        // Check if we have a cached entry
        if let Some(entry) = cache.get(&key) {
            // Clone the tree and return
            let tree = entry.tree.clone();
            let content = entry.content.clone();
            return Ok((tree, content));
        }

        // Parse the file
        drop(cache); // Release lock during parsing

        let tree = parser.parse(content, None).ok_or_else(|| {
            CodeDigestError::ParseError(format!(
                "Failed to parse {} file: {}",
                language,
                path.display()
            ))
        })?;

        let entry = CacheEntry {
            tree: tree.clone(),
            content: content.to_string(),
        };

        // Re-acquire lock and insert
        let mut cache = self
            .cache
            .lock()
            .map_err(|_| CodeDigestError::MutexPoisoned)?;
        cache.put(key, entry);

        Ok((tree, content.to_string()))
    }

    /// Clear the cache
    pub fn clear(&self) -> Result<(), CodeDigestError> {
        let mut cache = self
            .cache
            .lock()
            .map_err(|_| CodeDigestError::MutexPoisoned)?;
        cache.clear();
        Ok(())
    }

    /// Get current cache size
    pub fn len(&self) -> Result<usize, CodeDigestError> {
        let cache = self
            .cache
            .lock()
            .map_err(|_| CodeDigestError::MutexPoisoned)?;
        Ok(cache.len())
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> Result<bool, CodeDigestError> {
        let cache = self
            .cache
            .lock()
            .map_err(|_| CodeDigestError::MutexPoisoned)?;
        Ok(cache.is_empty())
    }

    /// Get cache capacity
    pub fn capacity(&self) -> Result<usize, CodeDigestError> {
        let cache = self
            .cache
            .lock()
            .map_err(|_| CodeDigestError::MutexPoisoned)?;
        Ok(cache.cap().get())
    }

    /// Resize the cache
    pub fn resize(&self, capacity: usize) -> Result<(), CodeDigestError> {
        if let Some(cap) = NonZeroUsize::new(capacity) {
            let mut cache = self
                .cache
                .lock()
                .map_err(|_| CodeDigestError::MutexPoisoned)?;
            cache.resize(cap);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(dead_code)]
    fn get_test_parser() -> Parser {
        // Note: In real usage, we'd set a proper language
        Parser::new()
    }

    #[test]
    fn test_cache_key_creation() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.rs");
        fs::write(&file_path, "fn main() {}").unwrap();

        let key1 = CacheKey::new(&file_path, "fn main() {}", "rust").unwrap();
        let key2 = CacheKey::new(&file_path, "fn main() {}", "rust").unwrap();
        let key3 = CacheKey::new(&file_path, "fn test() {}", "rust").unwrap();

        assert_eq!(key1, key2);
        assert_ne!(key1, key3); // Different content
    }

    #[test]
    fn test_cache_operations() {
        let cache = AstCache::new(10);

        assert_eq!(cache.len().unwrap(), 0);
        assert!(cache.is_empty().unwrap());
        assert_eq!(cache.capacity().unwrap(), 10);

        cache.clear().unwrap();
        assert!(cache.is_empty().unwrap());

        cache.resize(20).unwrap();
        assert_eq!(cache.capacity().unwrap(), 20);
    }
}
