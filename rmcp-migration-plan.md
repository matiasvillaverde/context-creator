# RMCP Migration Plan

## Overview
This document outlines the migration from jsonrpsee to rmcp for the MCP server implementation in context-creator.

## Current Architecture Analysis

### jsonrpsee Usage
1. **Server Setup** (`src/mcp_server/mod.rs`):
   - Uses `jsonrpsee::server::Server` for HTTP JSON-RPC
   - Port-based server with `start_server(addr: &str)`
   - RPC traits defined with `#[rpc(server)]` macro
   
2. **RPC Methods**:
   - Health check endpoint
   - Codebase processing endpoints (local and remote)
   - File metadata, search, diff, and semantic search endpoints
   
3. **Request/Response Types**:
   - Strongly typed Rust structs with serde
   - Async handlers returning `RpcResult<T>`

4. **Testing**:
   - HTTP client-based integration tests
   - Process spawning for server testing
   - Mock tests for isolated testing

## RMCP Architecture

### Key Differences
1. **Tool-based API**: RMCP uses "tools" instead of RPC methods
2. **Multiple Transports**: stdio, HTTP/SSE, streamable HTTP
3. **ServerHandler trait**: Central trait for implementing servers
4. **Built-in MCP Protocol**: Native MCP protocol support

## Migration Strategy

### Phase 1: Infrastructure Setup
1. Update Cargo.toml dependencies
2. Create new module structure for RMCP server
3. Keep existing jsonrpsee server in parallel initially

### Phase 2: Core Implementation
1. **ServerHandler Implementation**:
   ```rust
   #[derive(Clone, Debug)]
   pub struct ContextCreatorServer {
       cache: Arc<McpCache>,
       tool_router: ToolRouter<Self>,
   }
   ```

2. **Tool Conversion Map**:
   - `health_check` → Built-in server info
   - `process_local_codebase` → `analyze_local` tool
   - `process_remote_repo` → `analyze_remote` tool
   - `get_file_metadata` → `file_metadata` tool
   - `search_codebase` → `search` tool
   - `diff_files` → `diff` tool
   - `semantic_search` → `semantic_search` tool

3. **Request/Response Adaptation**:
   - Keep existing types but wrap in RMCP's `Parameters<T>` for input
   - Use `Content::text()` or `Json<T>` for output

### Phase 3: Transport Implementation
1. **stdio mode** (primary for MCP):
   ```rust
   let service = server.serve(stdio()).await?;
   service.waiting().await?;
   ```

2. **HTTP/SSE mode** (for web clients):
   ```rust
   let ct = SseServer::serve(addr).await?
       .with_service_directly(|| Ok(ContextCreatorServer::new()));
   ```

### Phase 4: Testing Strategy

#### Unit Tests
- Test individual tool handlers
- Mock the `RequestContext` for isolated testing

#### Integration Tests
1. **stdio tests**:
   ```rust
   use rmcp::transport::stdio::test_utils::*;
   let (client, server) = create_test_pair().await?;
   ```

2. **HTTP tests**:
   - Use rmcp's HTTP client capabilities
   - Test SSE streaming for long operations

3. **Protocol Compliance**:
   - Use MCP test suite if available
   - Verify tool discovery (`list_tools`)
   - Test error handling

#### Migration Tests
- Run both servers in parallel
- Compare outputs for same inputs
- Ensure backward compatibility

### Phase 5: Cutover Plan
1. Feature flag for server selection
2. Gradual rollout with monitoring
3. Remove jsonrpsee after validation

## Implementation Details

### Tool Implementation Pattern
```rust
#[tool_router]
impl ContextCreatorServer {
    #[tool(description = "Analyze a local codebase directory")]
    pub async fn analyze_local(
        &self,
        Parameters(request): Parameters<ProcessLocalRequest>,
    ) -> Result<Json<ProcessLocalResponse>, ErrorData> {
        // Validate and process
        // Return Json response
    }
}
```

### Error Handling
```rust
fn to_error_data(e: anyhow::Error) -> ErrorData {
    ErrorData::new(
        ErrorCode::INTERNAL_ERROR,
        e.to_string(),
        None,
    )
}
```

### Caching Integration
- Keep existing cache structure
- Adapt cache keys for tool names

## Testing Checklist

### Functional Tests
- [ ] Health/info endpoint works
- [ ] Local codebase analysis with all parameters
- [ ] Remote repo analysis
- [ ] File metadata retrieval
- [ ] Search functionality
- [ ] Diff generation
- [ ] Semantic search
- [ ] Caching works correctly
- [ ] LLM integration functions

### Performance Tests
- [ ] Response times comparable to jsonrpsee
- [ ] Memory usage acceptable
- [ ] Concurrent request handling

### Protocol Tests
- [ ] MCP client compatibility
- [ ] Tool discovery
- [ ] Error format compliance
- [ ] Streaming for large responses

### Edge Cases
- [ ] Large codebases
- [ ] Invalid paths/security
- [ ] Network failures
- [ ] Timeout handling
- [ ] Cache invalidation

## Rollback Plan
1. Keep jsonrpsee code in separate module
2. Environment variable to select server
3. Quick switch capability
4. Monitoring for issues

## Success Criteria
1. All existing functionality works
2. MCP protocol compliance verified
3. Performance within 10% of current
4. All tests passing
5. No regression in user experience