//! MCP (Model Context Protocol) server implementation for context-creator
//!
//! This module provides a JSON-RPC server that allows AI agents to
//! analyze codebases programmatically.

use anyhow::Result;
use jsonrpsee::{
    core::RpcResult,
    proc_macros::rpc,
    server::{Server, ServerHandle as JsonRpcServerHandle},
};
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use std::net::SocketAddr;

pub mod cache;
pub mod handlers;
pub mod rmcp_handlers;
pub mod rmcp_server;

/// Health check response structure
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: u64,
    pub version: String,
}

/// RPC trait for health check
#[rpc(server)]
pub trait HealthRpc {
    /// Returns the current health status of the server
    #[method(name = "health_check")]
    async fn health_check(&self) -> RpcResult<HealthResponse>;
}

/// Request structure for process_local_codebase
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct ProcessLocalRequest {
    /// The question/prompt to ask about the codebase (primary feature)
    pub prompt: String,
    /// Path to the codebase to analyze
    pub path: std::path::PathBuf,
    /// Optional: specific file patterns to include
    pub include_patterns: Vec<String>,
    /// Optional: patterns to ignore
    pub ignore_patterns: Vec<String>,
    /// Optional: whether to trace imports
    pub include_imports: bool,
    /// Optional: max tokens for context (auto-calculated based on LLM if not specified)
    pub max_tokens: Option<u32>,
    /// Optional: LLM tool to use (default: gemini)
    pub llm_tool: Option<String>,
    /// Optional: return markdown context along with answer
    pub include_context: Option<bool>,
}

/// Response structure for process_local_codebase
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct ProcessLocalResponse {
    /// The LLM's answer to the prompt
    pub answer: String,
    /// Optional: the markdown context used (if include_context is true)
    pub context: Option<String>,
    /// Number of files analyzed
    pub file_count: usize,
    /// Token count of the context
    pub token_count: usize,
    /// Processing time in milliseconds
    pub processing_time_ms: u64,
    /// LLM tool used
    pub llm_tool: String,
}

/// Request structure for process_remote_repo
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct ProcessRemoteRequest {
    /// The question/prompt to ask about the repository (primary feature)
    pub prompt: String,
    /// Git repository URL to analyze
    pub repo_url: String,
    /// Optional: specific file patterns to include
    pub include_patterns: Vec<String>,
    /// Optional: patterns to ignore
    pub ignore_patterns: Vec<String>,
    /// Optional: whether to trace imports
    pub include_imports: bool,
    /// Optional: max tokens for context (auto-calculated based on LLM if not specified)
    pub max_tokens: Option<u32>,
    /// Optional: LLM tool to use (default: gemini)
    pub llm_tool: Option<String>,
    /// Optional: return markdown context along with answer
    pub include_context: Option<bool>,
}

/// Response structure for process_remote_repo
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct ProcessRemoteResponse {
    /// The LLM's answer to the prompt
    pub answer: String,
    /// Optional: the markdown context used (if include_context is true)
    pub context: Option<String>,
    /// Number of files analyzed
    pub file_count: usize,
    /// Token count of the context
    pub token_count: usize,
    /// Processing time in milliseconds
    pub processing_time_ms: u64,
    /// Repository name
    pub repo_name: String,
    /// LLM tool used
    pub llm_tool: String,
}

/// Request structure for get_file_metadata
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct GetFileMetadataRequest {
    pub file_path: std::path::PathBuf,
}

/// Response structure for get_file_metadata
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct GetFileMetadataResponse {
    pub path: std::path::PathBuf,
    pub size: u64,
    pub modified: u64,
    pub is_symlink: bool,
    pub language: Option<String>,
}

/// Request structure for search_codebase
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct SearchCodebaseRequest {
    pub path: std::path::PathBuf,
    pub query: String,
    pub max_results: Option<u32>,
    pub file_pattern: Option<String>,
}

/// Search result entry
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct SearchResult {
    pub file_path: std::path::PathBuf,
    pub line_number: usize,
    pub line_content: String,
    pub match_context: String,
}

/// Response structure for search_codebase
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct SearchCodebaseResponse {
    pub results: Vec<SearchResult>,
    pub total_matches: usize,
    pub files_searched: usize,
    pub search_time_ms: u64,
}

/// Request structure for diff_files
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct DiffFilesRequest {
    pub file1_path: std::path::PathBuf,
    pub file2_path: std::path::PathBuf,
    pub context_lines: Option<u32>,
}

/// Diff hunk structure
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct DiffHunk {
    pub old_start: usize,
    pub old_lines: usize,
    pub new_start: usize,
    pub new_lines: usize,
    pub content: String,
}

/// Response structure for diff_files
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct DiffFilesResponse {
    pub file1_path: std::path::PathBuf,
    pub file2_path: std::path::PathBuf,
    pub hunks: Vec<DiffHunk>,
    pub added_lines: usize,
    pub removed_lines: usize,
    pub is_binary: bool,
}

/// Request structure for semantic_search
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct SemanticSearchRequest {
    pub path: std::path::PathBuf,
    pub query: String,
    pub search_type: SemanticSearchType,
    pub max_results: Option<u32>,
}

/// Type of semantic search
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SemanticSearchType {
    /// Find functions/methods by name
    Functions,
    /// Find classes/structs/interfaces by name
    Types,
    /// Find imports/dependencies
    Imports,
    /// Find all references to a symbol
    References,
}

/// Semantic search result
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct SemanticSearchResult {
    pub file_path: std::path::PathBuf,
    pub symbol_name: String,
    pub symbol_type: String,
    pub line_number: usize,
    pub context: String,
}

/// Response structure for semantic_search
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct SemanticSearchResponse {
    pub results: Vec<SemanticSearchResult>,
    pub total_matches: usize,
    pub files_analyzed: usize,
    pub search_time_ms: u64,
}

/// RPC trait for codebase processing
#[rpc(server)]
pub trait CodebaseRpc {
    /// Process a local codebase directory
    #[method(name = "process_local_codebase")]
    async fn process_local_codebase(
        &self,
        request: ProcessLocalRequest,
    ) -> RpcResult<ProcessLocalResponse>;

    /// Process a remote repository
    #[method(name = "process_remote_repo")]
    async fn process_remote_repo(
        &self,
        request: ProcessRemoteRequest,
    ) -> RpcResult<ProcessRemoteResponse>;

    /// Get metadata for a specific file
    #[method(name = "get_file_metadata")]
    async fn get_file_metadata(
        &self,
        request: GetFileMetadataRequest,
    ) -> RpcResult<GetFileMetadataResponse>;

    /// Search codebase for a query string
    #[method(name = "search_codebase")]
    async fn search_codebase(
        &self,
        request: SearchCodebaseRequest,
    ) -> RpcResult<SearchCodebaseResponse>;

    /// Get diff between two files
    #[method(name = "diff_files")]
    async fn diff_files(&self, request: DiffFilesRequest) -> RpcResult<DiffFilesResponse>;

    /// Perform semantic search across codebase
    #[method(name = "semantic_search")]
    async fn semantic_search(
        &self,
        request: SemanticSearchRequest,
    ) -> RpcResult<SemanticSearchResponse>;
}

/// Server handle wrapper for managing the MCP server lifecycle
pub struct ServerHandle {
    inner: JsonRpcServerHandle,
    local_addr: SocketAddr,
}

impl ServerHandle {
    /// Get the local address the server is listening on
    pub fn local_addr(&self) -> Result<SocketAddr> {
        Ok(self.local_addr)
    }

    /// Stop the server gracefully
    pub fn stop(self) -> Result<()> {
        self.inner.stop()?;
        Ok(())
    }
}

/// Start the MCP server on the specified address
pub async fn start_server(addr: &str) -> Result<ServerHandle> {
    let addr: SocketAddr = addr.parse()?;

    // Build the server
    let server = Server::builder().build(addr).await?;

    // Get the actual address (in case port 0 was used)
    let local_addr = server.local_addr()?;

    // Create shared cache
    let cache = std::sync::Arc::new(cache::McpCache::new());

    // Create and register the handlers
    let health_impl = handlers::HealthRpcImpl;
    let codebase_impl = handlers::CodebaseRpcImpl::new(cache);

    // Merge RPC modules
    let mut rpc_module = health_impl.into_rpc();
    rpc_module.merge(codebase_impl.into_rpc())?;

    // Start the server in the background
    let handle = server.start(rpc_module);

    Ok(ServerHandle {
        inner: handle,
        local_addr,
    })
}
