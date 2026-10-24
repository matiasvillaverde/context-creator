//! RMCP-based MCP server implementation for context-creator
//!
//! This module provides an MCP-compliant server using the rmcp library
//! that allows AI agents to analyze codebases programmatically.

use anyhow::Result;
use rmcp::{
    handler::server::{
        router::tool::ToolRouter,
        wrapper::{Json, Parameters},
    },
    model::{ErrorCode, ErrorData, ServerCapabilities, ServerConfig},
    tool, tool_handler, tool_router, ServerHandler,
};
use std::sync::Arc;
use std::time::Instant;

use super::{
    DiffFilesRequest, DiffFilesResponse, GetFileMetadataRequest, GetFileMetadataResponse,
    ProcessLocalRequest, ProcessLocalResponse, ProcessRemoteRequest, ProcessRemoteResponse,
    SearchCodebaseRequest, SearchCodebaseResponse, SemanticSearchRequest, SemanticSearchResponse,
};

/// Context Creator MCP Server implementation
#[derive(Debug, Clone)]
pub struct ContextCreatorServer {
    cache: Arc<crate::mcp_server::cache::McpCache>,
    tool_router: ToolRouter<Self>,
}

impl ContextCreatorServer {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(crate::mcp_server::cache::McpCache::new()),
            tool_router: Self::tool_router(),
        }
    }
}

impl Default for ContextCreatorServer {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_router]
impl ContextCreatorServer {
    #[tool(
        description = "Get a bounded repository map of declarations and line numbers without calling an LLM"
    )]
    pub async fn repository_map(
        &self,
        Parameters(request): Parameters<super::RepositoryMapRequest>,
    ) -> Result<Json<super::RepositoryMapResponse>, ErrorData> {
        tokio::task::spawn_blocking(move || super::progressive::repository_map_sync(request))
            .await
            .map_err(|error| ErrorData::new(ErrorCode::INTERNAL_ERROR, error.to_string(), None))?
            .map(Json)
            .map_err(|error| ErrorData::new(ErrorCode::INVALID_PARAMS, error.to_string(), None))
    }

    #[tool(
        description = "Read a bounded source range relative to a repository root; respects ignore rules and denies symlinks and hidden paths"
    )]
    pub async fn read_file(
        &self,
        Parameters(request): Parameters<super::ReadFileRequest>,
    ) -> Result<Json<super::ReadFileResponse>, ErrorData> {
        tokio::task::spawn_blocking(move || super::progressive::read_file_sync(request))
            .await
            .map_err(|error| ErrorData::new(ErrorCode::INTERNAL_ERROR, error.to_string(), None))?
            .map(Json)
            .map_err(|error| ErrorData::new(ErrorCode::INVALID_PARAMS, error.to_string(), None))
    }

    #[tool(description = "Analyze a local codebase directory and answer questions about it")]
    pub async fn analyze_local(
        &self,
        Parameters(request): Parameters<ProcessLocalRequest>,
    ) -> Result<Json<ProcessLocalResponse>, ErrorData> {
        let start = Instant::now();

        let path = request.path.clone();
        tokio::task::spawn_blocking(move || super::rmcp_handlers::validate_path(&path))
            .await
            .map_err(|error| ErrorData::new(ErrorCode::INTERNAL_ERROR, error.to_string(), None))?
            .map_err(|error| ErrorData::new(ErrorCode::INVALID_PARAMS, error.to_string(), None))?;

        // Check cache first
        let cache_key = crate::mcp_server::cache::ProcessLocalCacheKey::from_request(&request);
        if let Some(cached) = self.cache.get_process_local(&cache_key).await {
            let processing_time_ms = start.elapsed().as_millis() as u64;
            let include_context =
                request.include_context.unwrap_or(false) || request.prompt.trim().is_empty();
            let markdown = cached.markdown;
            return Ok(Json(ProcessLocalResponse {
                answer: cached.answer,
                context: if include_context {
                    Some(markdown.clone())
                } else {
                    None
                },
                markdown: Some(markdown),
                file_count: cached.file_count,
                token_count: cached.token_count,
                processing_time_ms,
                llm_tool: cached.llm_tool,
            }));
        }

        // Use blocking task for file I/O
        let cache = self.cache.clone();
        let response = tokio::task::spawn_blocking(move || {
            super::handlers::process_codebase_sync(request, start)
        })
        .await
        .map_err(|e| ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?
        .map_err(|e| ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?;

        // Cache the response
        let cache_value = crate::mcp_server::cache::ProcessLocalCacheValue {
            answer: response.answer.clone(),
            markdown: response.markdown.clone().unwrap_or_default(),
            file_count: response.file_count,
            token_count: response.token_count,
            llm_tool: response.llm_tool.clone(),
        };
        cache.set_process_local(cache_key, cache_value).await;

        Ok(Json(response))
    }

    #[tool(description = "Analyze a remote Git repository and answer questions about it")]
    pub async fn analyze_remote(
        &self,
        Parameters(request): Parameters<ProcessRemoteRequest>,
    ) -> Result<Json<ProcessRemoteResponse>, ErrorData> {
        let start = Instant::now();

        // Check cache first
        let cache_key = crate::mcp_server::cache::ProcessRemoteCacheKey::from_request(&request);
        if let Some(cached) = self.cache.get_process_remote(&cache_key).await {
            let processing_time_ms = start.elapsed().as_millis() as u64;
            let include_context =
                request.include_context.unwrap_or(false) || request.prompt.trim().is_empty();
            let markdown = cached.markdown;
            return Ok(Json(ProcessRemoteResponse {
                answer: cached.answer,
                context: if include_context {
                    Some(markdown.clone())
                } else {
                    None
                },
                markdown: Some(markdown),
                file_count: cached.file_count,
                token_count: cached.token_count,
                processing_time_ms,
                repo_name: cached.repo_name,
                llm_tool: cached.llm_tool,
            }));
        }

        // Use blocking task for file I/O and git operations
        let cache = self.cache.clone();
        let response = tokio::task::spawn_blocking(move || {
            super::handlers::process_remote_sync(request, start)
        })
        .await
        .map_err(|e| ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?
        .map_err(|e| ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?;

        // Cache the response
        let cache_value = crate::mcp_server::cache::ProcessRemoteCacheValue {
            answer: response.answer.clone(),
            markdown: response.markdown.clone().unwrap_or_default(),
            file_count: response.file_count,
            token_count: response.token_count,
            repo_name: response.repo_name.clone(),
            llm_tool: response.llm_tool.clone(),
        };
        cache.set_process_remote(cache_key, cache_value).await;

        Ok(Json(response))
    }

    #[tool(description = "Get metadata information about a specific file")]
    pub async fn file_metadata(
        &self,
        Parameters(request): Parameters<GetFileMetadataRequest>,
    ) -> Result<Json<GetFileMetadataResponse>, ErrorData> {
        tokio::task::spawn_blocking(move || super::handlers::get_file_metadata_sync(request))
            .await
            .map_err(|e| ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?
            .map(Json)
            .map_err(|e| ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))
    }

    #[tool(description = "Search for text patterns across the codebase")]
    pub async fn search(
        &self,
        Parameters(request): Parameters<SearchCodebaseRequest>,
    ) -> Result<Json<SearchCodebaseResponse>, ErrorData> {
        let start = Instant::now();
        tokio::task::spawn_blocking(move || super::handlers::search_codebase_sync(request, start))
            .await
            .map_err(|e| ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?
            .map(Json)
            .map_err(|e| ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))
    }

    #[tool(description = "Generate a diff between two files")]
    pub async fn diff(
        &self,
        Parameters(request): Parameters<DiffFilesRequest>,
    ) -> Result<Json<DiffFilesResponse>, ErrorData> {
        tokio::task::spawn_blocking(move || super::handlers::diff_files_sync(request))
            .await
            .map_err(|e| ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?
            .map(Json)
            .map_err(|e| ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))
    }

    #[tool(description = "Perform semantic search for code symbols (functions, types, imports)")]
    pub async fn semantic_search(
        &self,
        Parameters(request): Parameters<SemanticSearchRequest>,
    ) -> Result<Json<SemanticSearchResponse>, ErrorData> {
        let start = Instant::now();
        tokio::task::spawn_blocking(move || super::handlers::semantic_search_sync(request, start))
            .await
            .map_err(|e| ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?
            .map(Json)
            .map_err(|e| ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for ContextCreatorServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build()).with_instructions(
            concat!(
            "Context Creator MCP Server - Analyze codebases and answer questions about them.\n\n",
            "This server provides tools to:\n",
            "- Analyze local directories with 'analyze_local'\n",
            "- Analyze remote Git repositories with 'analyze_remote'\n",
            "- Explore declarations with 'repository_map', then page source with 'read_file'\n",
            "- Get file metadata with 'file_metadata'\n",
            "- Search codebases with 'search'\n",
            "- Generate diffs with 'diff'\n",
            "- Perform semantic searches with 'semantic_search'\n\n",
            "Analysis returns context directly when no prompt is supplied."
        ),
        )
    }
}

/// Start the RMCP server with specified transport
pub mod transport {
    use super::*;
    use rmcp::{transport::stdio, ServiceExt};

    /// Start server with stdio transport (for MCP CLI usage)
    pub async fn start_stdio() -> Result<()> {
        tracing::info!("Starting Context Creator MCP server (stdio mode)");

        let server = ContextCreatorServer::new();
        let service = server.serve(stdio()).await?;

        service.waiting().await?;
        Ok(())
    }

    /// Start Streamable HTTP at /mcp, with legacy protocol negotiation supported.
    pub async fn start_http(addr: &str) -> Result<()> {
        use rmcp::transport::streamable_http_server::{
            session::local::LocalSessionManager, StreamableHttpServerConfig, StreamableHttpService,
        };

        let listener = tokio::net::TcpListener::bind(addr).await?;
        let address = listener.local_addr()?;
        tracing::info!("Starting Context Creator MCP server at http://{address}/mcp");

        let config = StreamableHttpServerConfig::default()
            .with_json_response(true)
            .with_allowed_origins([
                format!("http://localhost:{}", address.port()),
                format!("http://127.0.0.1:{}", address.port()),
            ]);
        let cancellation = config.cancellation_token.clone();
        // Share caches across stateless requests as well as legacy sessions.
        let server = ContextCreatorServer::new();
        let service = StreamableHttpService::new(
            move || Ok(server.clone()),
            Arc::new(LocalSessionManager::default()),
            config,
        );
        let router = axum::Router::new().nest_service("/mcp", service);
        axum::serve(listener, router)
            .with_graceful_shutdown(async move {
                let _ = tokio::signal::ctrl_c().await;
                cancellation.cancel();
            })
            .await?;
        Ok(())
    }
}
