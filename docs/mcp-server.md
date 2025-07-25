# MCP Server Documentation

## Overview

The context-creator MCP (Model Context Protocol) server provides a JSON-RPC API for AI agents to analyze codebases programmatically. It enables remote codebase analysis, file searching, semantic code understanding, and more.

## Starting the Server

```bash
# Start MCP server on default port (8080)
cargo run -- --mcp

# Start on custom port
cargo run -- --mcp --mcp-port 9090
```

## API Endpoints

### 1. Health Check

Check if the server is running and healthy.

**Method:** `health_check`

**Request:**
```json
{
  "jsonrpc": "2.0",
  "method": "health_check",
  "params": [],
  "id": 1
}
```

**Response:**
```json
{
  "status": "healthy",
  "timestamp": 1234567890,
  "version": "1.2.0"
}
```

### 2. Process Local Codebase

Analyze a local directory and convert it to LLM-optimized Markdown.

**Method:** `process_local_codebase`

**Request:**
```json
{
  "jsonrpc": "2.0",
  "method": "process_local_codebase",
  "params": [{
    "path": "/path/to/project",
    "include_patterns": ["*.rs", "*.py"],
    "ignore_patterns": ["target/*", "*.pyc"],
    "include_imports": true,
    "max_tokens": 10000
  }],
  "id": 2
}
```

**Response:**
```json
{
  "markdown": "# Project Context\n\n## File: main.rs\n...",
  "file_count": 25,
  "token_count": 8500,
  "processing_time_ms": 150
}
```

### 3. Process Remote Repository

Clone and analyze a remote Git repository.

**Method:** `process_remote_repo`

**Request:**
```json
{
  "jsonrpc": "2.0",
  "method": "process_remote_repo",
  "params": [{
    "repo_url": "https://github.com/user/repo",
    "include_patterns": ["*.js"],
    "ignore_patterns": ["node_modules/*"],
    "include_imports": false,
    "max_tokens": 20000
  }],
  "id": 3
}
```

**Response:**
```json
{
  "markdown": "# Repository: repo\n\n## File: index.js\n...",
  "file_count": 15,
  "token_count": 12000,
  "processing_time_ms": 2500,
  "repo_name": "repo"
}
```

### 4. Get File Metadata

Retrieve metadata about a specific file.

**Method:** `get_file_metadata`

**Request:**
```json
{
  "jsonrpc": "2.0",
  "method": "get_file_metadata",
  "params": [{
    "file_path": "/path/to/file.rs"
  }],
  "id": 4
}
```

**Response:**
```json
{
  "path": "/path/to/file.rs",
  "size": 2048,
  "modified": 1234567890,
  "is_symlink": false,
  "language": "rust"
}
```

### 5. Search Codebase

Search for text patterns across files.

**Method:** `search_codebase`

**Request:**
```json
{
  "jsonrpc": "2.0",
  "method": "search_codebase",
  "params": [{
    "path": "/path/to/project",
    "query": "TODO",
    "max_results": 10,
    "file_pattern": "*.rs"
  }],
  "id": 5
}
```

**Response:**
```json
{
  "results": [
    {
      "file_path": "/path/to/main.rs",
      "line_number": 42,
      "line_content": "    // TODO: Implement error handling",
      "match_context": "...Implement error handling..."
    }
  ],
  "total_matches": 25,
  "files_searched": 100,
  "search_time_ms": 50
}
```

### 6. Semantic Search

Search for code elements using semantic understanding.

**Method:** `semantic_search`

**Request:**
```json
{
  "jsonrpc": "2.0",
  "method": "semantic_search",
  "params": [{
    "path": "/path/to/project",
    "query": "parse",
    "search_type": "functions",
    "max_results": 20
  }],
  "id": 6
}
```

**Search Types:**
- `functions` - Search function/method definitions
- `types` - Search type definitions and usage
- `imports` - Search import statements
- `references` - Search for references to symbols

**Response:**
```json
{
  "results": [
    {
      "file_path": "/path/to/parser.rs",
      "symbol_name": "parse_config",
      "symbol_type": "function",
      "line_number": 15,
      "context": "pub function parse_config"
    }
  ],
  "total_matches": 8,
  "files_analyzed": 50,
  "search_time_ms": 120
}
```

### 7. Diff Files

Compare two files and get a unified diff.

**Method:** `diff_files`

**Request:**
```json
{
  "jsonrpc": "2.0",
  "method": "diff_files",
  "params": [{
    "file1_path": "/path/to/old.rs",
    "file2_path": "/path/to/new.rs",
    "context_lines": 3
  }],
  "id": 7
}
```

**Response:**
```json
{
  "file1_path": "/path/to/old.rs",
  "file2_path": "/path/to/new.rs",
  "hunks": [
    {
      "old_start": 10,
      "old_lines": 5,
      "new_start": 10,
      "new_lines": 7,
      "content": "@@ -10,5 +10,7 @@\n fn main() {\n-    println!(\"Hello\");\n+    println!(\"Hello, world!\");\n+    // New comment\n }"
    }
  ],
  "added_lines": 2,
  "removed_lines": 1,
  "is_binary": false
}
```

## Error Handling

The server uses standard JSON-RPC error codes:

- `-32700`: Parse error
- `-32600`: Invalid request
- `-32601`: Method not found
- `-32602`: Invalid params
- `-32603`: Internal error

Example error response:
```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32602,
    "message": "Invalid path: potential security risk",
    "data": null
  },
  "id": 1
}
```

## Security Considerations

1. **Path Validation**: The server validates all file paths to prevent directory traversal attacks.
2. **URL Validation**: Remote repository URLs are validated before cloning.
3. **Resource Limits**: Token limits prevent excessive memory usage.
4. **Timeout Protection**: Long-running operations have timeouts.

## Performance Features

1. **Caching**: Results are cached for 5 minutes to improve response times.
2. **Parallel Processing**: File analysis uses parallel processing.
3. **Thread Pool Optimization**: Rayon thread pool is configured to avoid competing with Tokio.
4. **Async I/O**: All I/O operations are non-blocking.

## Example Client

See `examples/mcp_client.rs` for a complete example of using all API endpoints.

```rust
use jsonrpsee::{core::client::ClientT, http_client::HttpClientBuilder, rpc_params};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<()> {
    let client = HttpClientBuilder::default()
        .build("http://127.0.0.1:8080")?;

    // Health check
    let health: serde_json::Value = client
        .request("health_check", rpc_params![])
        .await?;
    
    println!("Server status: {}", health["status"]);
    Ok(())
}
```

## Integration with AI Agents

The MCP server is designed to be used by AI agents for codebase understanding tasks:

1. **Code Review**: Use semantic search to find relevant functions and analyze their implementation.
2. **Debugging**: Search for error messages or specific patterns across the codebase.
3. **Documentation**: Process entire codebases to generate comprehensive documentation.
4. **Refactoring**: Use diff functionality to preview changes before applying them.
5. **Learning**: Analyze code structure and dependencies to understand project architecture.

## Monitoring

The server logs important events to stderr:
- Server startup and shutdown
- Request processing times
- Error conditions
- Cache hits/misses

Use standard logging environment variables to control log levels:
```bash
RUST_LOG=debug cargo run -- --mcp
```

## Future Enhancements

- WebSocket support for streaming responses
- Authentication and authorization
- Rate limiting per client
- Metrics endpoint for Prometheus
- GraphQL API alternative
- Plugin system for custom analyzers