# MCP Server Guide

## Overview

context-creator includes a built-in MCP (Model Context Protocol) server that allows AI assistants like Claude to analyze your codebases programmatically. The server provides powerful tools for code analysis, search, and understanding.

## Available MCP Tools

When connected to an MCP client, you'll have access to these tools:

- **`build_context`** - Bounded task context with explanations, Git impact, contract hints, and revision-checked sessions
- **`list_recipes`** - Built-in and supplied reusable agent-tool recipes
- **`repository_map`** - Bounded declarations/signatures and source locations, without a model call
- **`read_file`** - Root-confined, paginated source reads, without a model call
- **`analyze_local`** - Analyze a local codebase directory and answer questions about it
- **`analyze_remote`** - Analyze a remote Git repository
- **`search`** - Search for text patterns across the codebase
- **`semantic_search`** - Find functions, types, imports, and symbols
- **`file_metadata`** - Get detailed information about specific files
- **`diff`** - Generate diffs between two files

## Progressive exploration

For a complete task request, call `build_context` with arguments such as
`{"root":"/path/to/repo","task":"Fix session expiration","recipe":"debug_failure","max_tokens":8000}`.
The compact JSON response fits this budget; the outer protocol envelope is extra.
This tool never invokes an LLM. `list_recipes` accepts `{}` or a `recipes` object.
Both method names are shared by RMCP and legacy JSON-RPC. See the
[agent tools guide](agent-tools.md) for schemas, recipes, and session update rules.

Start with `repository_map` and then request only the files/ranges needed for a
task. Both tools work through RMCP stdio/Streamable HTTP and the legacy JSON-RPC
server under the same method names. They never invoke an LLM.

```json
{"path":"/path/to/repo","max_tokens":4096,"secret_scan":"redact"}
```

`repository_map` returns `map`, `token_count`, `file_count`, and `omitted_files`.
Optional fields are `include_patterns`, `ignore_patterns`, `tokenizer`, and
`secret_scan`. It reuses the source walk and dependency analysis, caches
extracted declarations during fitting, and trims whole file entries. The default
budget is 4,096 tokens; the maximum is 32,768. Narrow inputs exceeding 10,000
eligible files before parsing. Files show up to 64 declarations each.

```json
{"root":"/path/to/repo","path":"src/main.rs","start_line":1,"max_lines":200,"max_tokens":4096}
```

`read_file` returns `content`, `start_line`, inclusive `end_line`, optional
`next_line`, `truncated`, and `token_count`. Pass `next_line` as `start_line` to
continue. The budget counts content, excluding the MCP JSON envelope. It fits
whole lines and reports an error when the first requested line cannot fit.
The default page is 200 lines/4,096 tokens, with limits of 1,000 lines/32,768
tokens. Input files are limited to 10 MiB. Empty or past-EOF ranges have no
`end_line` or cursor.

Paths are relative to `root`. Traversal, hidden components, symlinks, binary
files, and repository-ignored files are denied. Ignore matching examines only
ancestor directories, avoiding a full tree walk for each page. New calls read
current content rather than serving stale file pages. Both progressive tools
support `tokenizer` and `secret_scan`, defaulting to `o200k-base` and `redact`.
Redaction is heuristic; see the [scanning policy](configuration.md#secret-content-policy).
Existing analyze tools preserve the default `off` policy unless requested and
local analysis regenerates responses on every call, including secret checks.
All filesystem and parser work runs
on blocking workers.

## Installation and Setup

### Local binary and Streamable HTTP

```bash
# Stdio: configure the client to launch this command
context-creator --rmcp

# HTTP: configure the client with http://127.0.0.1:9090/mcp
context-creator --rmcp --rmcp-transport http --mcp-port 9090
```

RMCP 3.7 supports MCP `2026-07-28` discovery and stateless requests, alongside
older protocol negotiation. HTTP uses `/mcp`; the former `/sse` and message
endpoints have been replaced. The HTTP server binds to loopback, validates Host,
and accepts browser Origins only from `localhost` or `127.0.0.1` on its own port.
Stdio remains the simplest local client configuration.

Both `analyze_local` and `analyze_remote` accept optional `model` and `tokenizer`
fields, plus `output_reserve` and `secret_scan`. Model names pass through to the selected CLI; `tokenizer` accepts
`o200k-base` (default) or `cl100k-base`. Remote-response cache keys include both fields. For Ollama,
provide a model when sending a prompt. Omit `prompt` to build context without an
LLM call. `token_count` measures the returned context with the selected local
encoding, excluding provider instructions, tool schemas, and response tokens.

`analyze_local` (legacy: `process_local_codebase`) reads current files and ignore
rules on every call. Whole local responses and provider answers are not reused;
repeated calls with a prompt invoke the selected provider again. This avoids
returning pre-edit source or bypassing a newly detected secret. Use
`build_context` for revision-checked source-analysis reuse and incremental sessions.

Existing npm releases follow their packaged version; build this checkout to use
these changes before the next release.

### Quick Install via NPM

The easiest way to use context-creator as an MCP server is through npm:

```bash
# Using npx (no installation required)
npx -y context-creator-mcp@latest

# Or install globally
npm install -g context-creator-mcp@latest
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
         "command": "npx",
         "args": ["-y", "context-creator-mcp@latest"]
       }
     }
   }
   ```

2. **Restart Claude Desktop** to load the new configuration

3. **Verify connection** - Claude should now have access to context-creator tools

### Setting up with Claude Code (CLI)

1. **Project-level configuration** (recommended for team projects):
   
   Create `.mcp.json` in your project root:
   ```json
   {
     "mcpServers": {
       "context-creator": {
         "command": "npx",
         "args": ["-y", "context-creator-mcp@latest"]
       }
     }
   }
   ```

2. **Or add to global configuration**:
   ```bash
   # Add server
   claude mcp add context-creator -- npx -y context-creator-mcp@latest
   
   # Verify connection
   claude mcp list
   
   # Should show:
   # context-creator ✓ Connected
   ```

3. **Remove old configurations if needed**:
   ```bash
   claude mcp remove context-creator
   ```

### Building from Source (Advanced)

If you need to build from source:

```bash
# Clone and build
git clone https://github.com/matiasvillaverde/context-creator
cd context-creator
cargo build --release

# The binary will be at ./target/release/context-creator
# You can then configure your MCP client to use this path with --rmcp flag
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

### Check if context-creator is available

```bash
# Test with npx
npx -y context-creator-mcp@latest --version

# Or if installed globally
context-creator-mcp --version
```

### Verify Claude configuration

- Ensure you're using `npx` with the correct package name
- Check that Node.js v18+ is installed
- Verify the configuration is saved correctly

### Check logs

- Claude Desktop: Check developer console
- Claude Code: Run with verbose flag `claude -v`

### Common issues

- **Command not found**: Install Node.js v18 or later
- **Package not found**: Check internet connection and npm registry access
- **Permission denied**: May need to run with elevated permissions on Windows
- **Already configured**: Remove old config first with `claude mcp remove context-creator`

## Performance Considerations

1. **Caching**: Tokenizers and compiled queries are shared; `build_context` reuses unchanged source analysis. RMCP remote responses have a ten-minute cache; local analyze responses are regenerated.
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
