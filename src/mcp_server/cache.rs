//! Cache implementation for MCP server

use moka::future::Cache;
use serde::{Deserialize, Serialize};
use std::hash::Hash;
use std::path::PathBuf;
use std::time::Duration;

/// Cache key for process_local_codebase requests
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct ProcessLocalCacheKey {
    pub prompt: String,
    pub path: PathBuf,
    pub include_patterns: Vec<String>,
    pub ignore_patterns: Vec<String>,
    pub include_imports: bool,
    pub max_tokens: Option<u32>,
    pub llm_tool: Option<String>,
}

impl ProcessLocalCacheKey {
    pub fn from_request(request: &super::ProcessLocalRequest) -> Self {
        Self {
            prompt: request.prompt.clone(),
            path: request.path.clone(),
            include_patterns: request.include_patterns.clone(),
            ignore_patterns: request.ignore_patterns.clone(),
            include_imports: request.include_imports,
            max_tokens: request.max_tokens,
            llm_tool: request.llm_tool.clone(),
        }
    }
}

/// Cached response for process_local_codebase
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessLocalCacheValue {
    pub answer: String,
    pub markdown: String,
    pub file_count: usize,
    pub token_count: usize,
    pub llm_tool: String,
}

/// Cache key for process_remote_repo requests
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct ProcessRemoteCacheKey {
    pub prompt: String,
    pub repo_url: String,
    pub include_patterns: Vec<String>,
    pub ignore_patterns: Vec<String>,
    pub include_imports: bool,
    pub max_tokens: Option<u32>,
    pub llm_tool: Option<String>,
}

impl ProcessRemoteCacheKey {
    pub fn from_request(request: &super::ProcessRemoteRequest) -> Self {
        Self {
            prompt: request.prompt.clone(),
            repo_url: request.repo_url.clone(),
            include_patterns: request.include_patterns.clone(),
            ignore_patterns: request.ignore_patterns.clone(),
            include_imports: request.include_imports,
            max_tokens: request.max_tokens,
            llm_tool: request.llm_tool.clone(),
        }
    }
}

/// Cached response for process_remote_repo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessRemoteCacheValue {
    pub answer: String,
    pub markdown: String,
    pub file_count: usize,
    pub token_count: usize,
    pub repo_name: String,
    pub llm_tool: String,
}

/// MCP server cache
#[derive(Debug)]
pub struct McpCache {
    process_local_cache: Cache<ProcessLocalCacheKey, ProcessLocalCacheValue>,
    process_remote_cache: Cache<ProcessRemoteCacheKey, ProcessRemoteCacheValue>,
}

impl McpCache {
    /// Create a new cache with default configuration
    pub fn new() -> Self {
        let process_local_cache = Cache::builder()
            .max_capacity(100) // Cache up to 100 requests
            .time_to_live(Duration::from_secs(300)) // 5 minutes TTL
            .build();

        let process_remote_cache = Cache::builder()
            .max_capacity(50) // Cache up to 50 remote requests
            .time_to_live(Duration::from_secs(600)) // 10 minutes TTL
            .build();

        Self {
            process_local_cache,
            process_remote_cache,
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

    /// Get a cached process_remote_repo response
    pub async fn get_process_remote(
        &self,
        key: &ProcessRemoteCacheKey,
    ) -> Option<ProcessRemoteCacheValue> {
        self.process_remote_cache.get(key).await
    }

    /// Cache a process_remote_repo response
    pub async fn set_process_remote(
        &self,
        key: ProcessRemoteCacheKey,
        value: ProcessRemoteCacheValue,
    ) {
        self.process_remote_cache.insert(key, value).await;
    }

    /// Clear all caches
    pub fn clear_all(&self) {
        self.process_local_cache.invalidate_all();
        self.process_remote_cache.invalidate_all();
    }
}

impl Default for McpCache {
    fn default() -> Self {
        Self::new()
    }
}
