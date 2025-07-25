//! Cache implementation for MCP server

use moka::future::Cache;
use serde::{Deserialize, Serialize};
use std::hash::Hash;
use std::path::PathBuf;
use std::time::Duration;

/// Cache key for process_local_codebase requests
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct ProcessLocalCacheKey {
    pub path: PathBuf,
    pub include_patterns: Vec<String>,
    pub ignore_patterns: Vec<String>,
    pub include_imports: bool,
    pub max_tokens: Option<u32>,
}

impl ProcessLocalCacheKey {
    pub fn from_request(request: &super::ProcessLocalRequest) -> Self {
        Self {
            path: request.path.clone(),
            include_patterns: request.include_patterns.clone(),
            ignore_patterns: request.ignore_patterns.clone(),
            include_imports: request.include_imports,
            max_tokens: request.max_tokens,
        }
    }
}

/// Cached response for process_local_codebase
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessLocalCacheValue {
    pub markdown: String,
    pub file_count: usize,
    pub token_count: usize,
}

/// MCP server cache
pub struct McpCache {
    process_local_cache: Cache<ProcessLocalCacheKey, ProcessLocalCacheValue>,
}

impl McpCache {
    /// Create a new cache with default configuration
    pub fn new() -> Self {
        let process_local_cache = Cache::builder()
            .max_capacity(100) // Cache up to 100 requests
            .time_to_live(Duration::from_secs(300)) // 5 minutes TTL
            .build();

        Self {
            process_local_cache,
        }
    }

    /// Get a cached process_local_codebase response
    pub async fn get_process_local(
        &self,
        key: &ProcessLocalCacheKey,
    ) -> Option<ProcessLocalCacheValue> {
        self.process_local_cache.get(key).await
    }

    /// Cache a process_local_codebase response
    pub async fn set_process_local(
        &self,
        key: ProcessLocalCacheKey,
        value: ProcessLocalCacheValue,
    ) {
        self.process_local_cache.insert(key, value).await;
    }

    /// Clear all caches
    pub fn clear_all(&self) {
        self.process_local_cache.invalidate_all();
    }
}

impl Default for McpCache {
    fn default() -> Self {
        Self::new()
    }
}
