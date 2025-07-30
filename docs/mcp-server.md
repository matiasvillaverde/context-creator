# MCP Server Guide

## Overview

context-creator includes a built-in MCP (Model Context Protocol) server that allows AI assistants like Claude to analyze your codebases programmatically. The server provides powerful tools for code analysis, search, and understanding.

## Available MCP Tools

When connected to an MCP client, you'll have access to these tools:

- **`analyze_local`** - Analyze a local codebase directory and answer questions about it
- **`analyze_remote`** - Analyze a remote Git repository
- **`search`** - Search for text patterns across the codebase
- **`semantic_search`** - Find functions, types, imports, and symbols
- **`file_metadata`** - Get detailed information about specific files
- **`diff`** - Generate diffs between two files

## Installation and Setup

### Building context-creator

First, you need to build or install context-creator:

```bash
# Install from crates.io
cargo install context-creator

# Or build from source
git clone https://github.com/matiasvillaverde/context-creator
cd context-creator
cargo build --release
```

### Setting up with Claude Desktop

1. **Edit Claude Desktop configuration**:
   
   On macOS: `~/Library/Application Support/Claude/claude_desktop_config.json`
   On Windows: `%APPDATA%\Claude\claude_desktop_config.json`
   
   Add the MCP server configuration:
   ```json
   {
     "mcpServers": {
       "context-creator": {
         "command": "/path/to/context-creator",
         "args": ["--rmcp"],
         "env": {}
       }
     }
   }
   ```
   
   Replace `/path/to/context-creator` with:
   - Installed version: `~/.cargo/bin/context-creator`
   - Built from source: `/path/to/project/target/release/context-creator`

2. **Restart Claude Desktop** to load the new configuration

3. **Verify connection** - Claude should now have access to context-creator tools

### Setting up with Claude Code (CLI)

1. **Project-level configuration** (recommended for team projects):
   
   Create `.mcp.json` in your project root:
   ```json
   {
     "mcpServers": {
       "context-creator": {
         "command": "./target/release/context-creator",
         "args": ["--rmcp"],
         "env": {}
       }
     }
   }
   ```

2. **Or add to user-level configuration**:
   ```bash
   # Add server
   claude mcp add context-creator /path/to/context-creator --arg="--rmcp"
   
   # Verify connection
   claude mcp list
   
   # Should show:
   # context-creator ✓ Connected
   ```

3. **Remove old configurations if needed**:
   ```bash
   claude mcp remove context-creator
   ```

## Using MCP Tools in Claude

Once connected, you can ask Claude to analyze your codebase:

```
"Analyze the authentication system in this codebase"
→ Claude will use analyze_local tool

"Search for all TODO comments"
→ Claude will use search tool

"Find all functions that call the login() method"
→ Claude will use semantic_search tool

"What's the difference between old_auth.py and new_auth.py?"
→ Claude will use diff tool

"Analyze the React hooks in facebook/react repository"
→ Claude will use analyze_remote tool
```

## Tool Descriptions

### analyze_local

Analyzes a local codebase directory and answers questions about it.

**Parameters:**
- `path` - The directory path to analyze
- `question` - The question to answer about the codebase

**Example usage:**
```
"Review the error handling patterns in src/"
"Find potential SQL injection vulnerabilities"
"Which files implement rate limiting?"
"Trace all imports of the database module"
```

### analyze_remote

Analyzes a remote Git repository without cloning it locally.

**Parameters:**
- `repo_url` - The repository URL (GitHub, GitLab, etc.)
- `question` - The question to answer about the repository

**Example usage:**
```
"Analyze the authentication in https://github.com/example/repo"
"How does Rust's borrow checker work?" (analyzes rust-lang/rust)
"Explain React's reconciliation algorithm" (analyzes facebook/react)
```

### search

Searches for text patterns across the codebase.

**Parameters:**
- `path` - The directory to search in
- `query` - The search term or pattern
- `case_sensitive` - Whether the search is case-sensitive (optional)
- `file_pattern` - File pattern to limit search (optional)

**Example usage:**
```
"Find all API endpoints in this codebase"
"Search for hardcoded credentials"
"Find all references to deprecated functions"
```

### semantic_search

Performs semantic code search using AST analysis.

**Parameters:**
- `path` - The directory to search in
- `query` - The symbol or pattern to search for
- `search_type` - Type of search: "functions", "types", "imports", or "all"

**Example usage:**
```
"Show me all TypeScript interfaces"
"Where is the UserService class defined?"
"Find all async functions"
"List all imported external libraries"
```

### file_metadata

Gets detailed information about a specific file.

**Parameters:**
- `file_path` - The path to the file

**Returns:**
- File size, modification time, language, and other metadata

### diff

Generates a diff between two files.

**Parameters:**
- `file1_path` - Path to the first file
- `file2_path` - Path to the second file

**Returns:**
- Unified diff showing changes between files

## Advanced MCP Usage

### Complex Analysis Tasks

```
"Create a dependency graph of the authentication module"
"Find all code that needs updating for the new API version"
"Identify potential performance bottlenecks in the data processing pipeline"
"Generate a security audit report for the application"
```

### Cross-Repository Analysis

```
"Compare the error handling approaches in repo A vs repo B"
"Find similar implementations across multiple repositories"
"Analyze how different projects structure their authentication"
```

### Refactoring Support

```
"Find all places where we could use the new async/await syntax"
"Identify duplicate code that could be extracted into utilities"
"Show me all the places affected if I rename this function"
```

## Troubleshooting MCP Connection

### Check server is running

```bash
# Test standalone
context-creator --rmcp
# Should show: "Starting Context Creator MCP server (stdio mode)"
```

### Verify Claude configuration

- Ensure path to context-creator is absolute
- Check file has execute permissions
- Verify `--rmcp` argument is included

### Check logs

- Claude Desktop: Check developer console
- Claude Code: Run with verbose flag `claude -v`

### Common issues

- **Path not found**: Use full absolute path
- **Permission denied**: `chmod +x /path/to/context-creator`
- **Already configured**: Remove old config first
- **Server not starting**: Check for port conflicts if using HTTP mode

## Performance Considerations

1. **Caching**: The MCP server caches analysis results for better performance
2. **Parallel Processing**: File analysis uses all available CPU cores
3. **Memory Management**: Large repositories are processed incrementally
4. **Token Limits**: Responses are automatically truncated to fit context windows

## Security Features

1. **Path Validation**: All file paths are validated to prevent directory traversal
2. **Repository Validation**: Only valid Git URLs are accepted
3. **Sandboxing**: Remote repositories are analyzed in isolated environments
4. **Resource Limits**: CPU and memory usage are bounded

## Integration Tips

### For Development Teams

1. Add `.mcp.json` to your repository for consistent setup
2. Configure `.contextignore` to exclude sensitive files
3. Use `.contextkeep` to prioritize important files
4. Document MCP usage in your team's README

### For AI-Assisted Development

1. Use specific, targeted questions for better results
2. Combine multiple tools for comprehensive analysis
3. Iterate on queries based on initial results
4. Save useful queries as documentation

## Future Enhancements

- WebSocket support for real-time updates
- Integration with more AI platforms
- Custom analysis plugins
- Team collaboration features
- Performance profiling tools