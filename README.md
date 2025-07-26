# context-creator
> Intelligent context engineering for LLM-powered development

[![CI](https://github.com/matiasvillaverde/context-creator/actions/workflows/ci.yml/badge.svg)](https://github.com/matiasvillaverde/context-creator/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=flat&logo=rust&logoColor=white)](https://www.rust-lang.org/)

`context-creator` transforms your codebase into intelligently curated LLM context. Unlike simple concatenation tools, it builds a dependency graph to create relevant, focused contexts that make your AI-powered development actually work.

## Why context-creator?

**🎯 Smart Context Engineering**  
Creates a dependency graph of your codebase. When you ask about authentication, it includes auth files, their dependencies, and related tests—nothing more, nothing less.

**⚡ Blazing Fast**  
Built in Rust with parallel processing. Handles massive codebases in seconds, not minutes.

**🧠 Intelligent Prioritization**  
When hitting token limits, it keeps the most important files based on Git history, dependencies, and your `.contextkeep` rules.

**🚀 Direct LLM Integration**  
Pipe directly to Gemini (or any LLM) for instant answers about your codebase.

## Quick Start

```bash
# Install
cargo install context-creator

# Ask Gemini about your codebase
context-creator --prompt "How can I add 2FA to the authentication system?"

# Analyze a specific feature area
context-creator --prompt "Find all performance bottlenecks in the API layer"

# Plan implementation work
context-creator --prompt "I need to add WebAuthn support. Which files need changes?"

# Architecture review
context-creator --prompt "Generate a dependency graph of the payment processing module"

# Analyze git changes
context-creator diff HEAD~1 HEAD
```

## Real-World Examples

### 🔍 Feature Planning
```bash
context-creator --prompt "I want to implement rate limiting. Show me:
1. Current middleware architecture
2. Files I'll need to modify
3. Suggested implementation approach"
```

### 🐛 Performance Analysis
```bash
context-creator --prompt "Analyze database queries across the codebase. 
Find N+1 queries and suggest optimizations."
```

### 🏗️ Architecture Understanding
```bash
context-creator --prompt "Explain how user authentication flows through the system.
Include relevant files and create a sequence diagram."
```

### 🔒 Security Audit
```bash
context-creator --prompt "Review authentication and authorization code for vulnerabilities.
Focus on JWT handling and session management."
```

### 📋 Change Analysis
```bash
context-creator diff HEAD~10 HEAD --prompt "Summarize all changes in the last 10 commits.
What are the main features added and potential risks introduced?"
```

## How It Works

Unlike tools that simply concatenate files, `context-creator`:

1. **Builds a dependency graph** of your entire codebase
2. **Extracts relevant subgraphs** based on your query
3. **Prioritizes files** by importance (Git history, dependencies, explicit rules)
4. **Optimizes for token limits** by intelligently pruning less relevant files
5. **Streams to LLMs** with context-aware ordering (important files last)

## Advanced Context Building

### 🔗 Dependency Graph Features

**Note:** Dependency graph analysis currently supports **Python**, **TypeScript/JavaScript**, and **Rust**. For other languages, `context-creator` works as a fast, intelligent concatenation tool.

#### `--trace-imports` - Follow Import Chains
```bash
# Find all files that depend on your authentication module
context-creator --prompt "Show me everything that uses the auth module" --trace-imports

# Trace specific module dependencies
context-creator --trace-imports --include "**/auth.py"
```

#### `--include-callers` - Find Function Usage
```bash
# Find all places where login() is called
context-creator --prompt "Where is the login function used?" --include-callers

# Analyze payment processing call chain
context-creator --include-callers --include "**/payment.ts"
```

#### `--include-types` - Include Type Definitions
```bash
# Include all type definitions and interfaces
context-creator --prompt "Review the type system" --include-types

# Analyze data models
context-creator --include-types --include "**/models/**"
```

#### `--semantic-depth` - Control Traversal Depth
```bash
# Shallow analysis (direct dependencies only)
context-creator --prompt "Quick auth overview" --include-types --semantic-depth 1

# Deep analysis (up to 10 levels)
context-creator --prompt "Full dependency analysis" --include-types --semantic-depth 10
```

#### `--git-context` - Include Git History in File Headers
```bash
# Include recent commit messages for each file
context-creator --prompt "Review recent changes" --git-context

# Combine with enhanced context for full metadata
context-creator --enhanced-context --git-context

# Useful for understanding code evolution
context-creator --include "src/auth/**" --git-context --prompt "How has authentication evolved?"
```

When enabled, adds git commit history to each file header:
```markdown
## src/auth/login.rs
Git history:
  - feat: add OAuth2 support by John Doe
  - fix: handle rate limiting in login flow by Jane Smith
  - refactor: extract validation logic by John Doe
```

### 📊 Real-World Dependency Graph Example

When you run:
```bash
context-creator --prompt "How does the payment system work?" --include "src/PaymentService.rs" --trace-imports --include-callers --include-types
```

The tool:
1. Finds `PaymentService.rs` and related files
2. Traces all imports (Stripe SDK, database models, utility functions)
3. Finds all callers (checkout flow, refund handlers, admin tools)
4. Builds a complete context of how payments flow through your system

### 🔍 Search Command

Search for specific terms across your codebase and automatically build comprehensive context:

```bash
# Search with automatic semantic analysis
context-creator search "AuthenticationService"

# Search without semantic analysis (faster, but less comprehensive)
context-creator search "TODO" --no-semantic

# Search in specific directories
context-creator search "database" src/ tests/
```

The search command:
- Uses parallel processing across all CPU cores
- Streams files line-by-line (memory efficient)
- Respects `.gitignore` and `.contextignore` patterns
- Automatically enables `--trace-imports`, `--include-callers`, and `--include-types` for comprehensive context

### 📈 Git Diff Command

Analyze changes between git references with intelligent context building:

```bash
# Compare current working directory with last commit
context-creator diff HEAD~1 HEAD

# Compare two branches
context-creator diff main feature-branch

# Compare with specific commit hash
context-creator diff a1b2c3d HEAD

# Save diff analysis to file
context-creator --output-file diff-analysis.md diff HEAD~1 HEAD

# Apply token limits to focus on most important changes
context-creator --max-tokens 50000 diff HEAD~5 HEAD

# Include semantic analysis of changed files
context-creator --trace-imports --include-callers --include-types diff main HEAD
```

The diff command:
- **Security hardened** - Validates git references to prevent command injection attacks
- **Markdown formatted** - Generates structured analysis with file contents and statistics
- **Token aware** - Respects token limits and prioritizes most important changed files
- **Semantic integration** - Optionally includes dependency analysis of changed files
- **Change statistics** - Shows files changed, lines added/removed, and estimated token usage

#### Git Diff Output Format

The generated analysis includes:
1. **Diff Statistics** - Summary of files changed, lines added/removed
2. **Changed Files List** - All modified files with relative paths
3. **File Contents** - Full content of changed files with syntax highlighting
4. **Context Statistics** - Token count and processing summary
5. **Semantic Analysis** - Optional dependency and caller information

Perfect for:
- **Code reviews** - Generate comprehensive change summaries
- **Feature documentation** - Document what changed in a feature branch
- **Impact analysis** - Understand scope of changes across git references
- **LLM analysis** - Feed git diffs to AI for automated review and suggestions

## Configuration

### `.contextkeep` - Prioritize Critical Files
```gitignore
# Always include these when relevant
src/auth/**
src/core/**
Cargo.toml
package.json
```

### `.contextignore` - Exclude Noise
```gitignore
# Never include
target/
node_modules/
*.log
.env
```

### `.context-creator.toml` - Advanced Config
```toml
[defaults]
max_tokens = 200000

# First-match-wins priority rules
[[priorities]]
pattern = "src/core/**"
weight = 100

[[priorities]]
pattern = "tests/**"
weight = 50

[[priorities]]
pattern = "docs/**"
weight = -10  # Negative weight = lower priority
```

## Installation

```bash
# Using Cargo
cargo install context-creator

# Prerequisites: Gemini CLI (for --prompt)
npm install -g @google/gemini-cli
gcloud auth application-default login
```

## MCP Server Mode

context-creator includes a built-in MCP (Model Context Protocol) server that allows AI assistants like Claude to analyze your codebases programmatically. The server provides powerful tools for code analysis, search, and understanding.

### Available MCP Tools

When connected to Claude, you'll have access to these tools:

- **`analyze_local`** - Analyze a local codebase directory and answer questions about it
- **`analyze_remote`** - Analyze a remote Git repository
- **`search`** - Search for text patterns across the codebase
- **`semantic_search`** - Find functions, types, imports, and symbols
- **`file_metadata`** - Get detailed information about specific files
- **`diff`** - Generate diffs between two files

### Setting up with Claude Desktop

1. **Build or install context-creator**:
   ```bash
   # Install from crates.io
   cargo install context-creator
   
   # Or build from source
   cargo build --release
   ```

2. **Edit Claude Desktop configuration**:
   
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

3. **Restart Claude Desktop** to load the new configuration

4. **Verify connection** - Claude should now have access to context-creator tools

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

### Using MCP Tools in Claude

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

### Advanced MCP Usage

#### Analyzing Local Projects
```
"Review the error handling patterns in src/"
"Find potential SQL injection vulnerabilities"
"Which files implement rate limiting?"
"Trace all imports of the database module"
```

#### Analyzing Remote Repositories
```
"Analyze the authentication in https://github.com/example/repo"
"How does Rust's borrow checker work?" (analyzes rust-lang/rust)
"Explain React's reconciliation algorithm" (analyzes facebook/react)
```

#### Code Search and Navigation
```
"Find all API endpoints in this codebase"
"Show me all TypeScript interfaces"
"Where is the UserService class defined?"
"Find all calls to deprecated functions"
```

### Troubleshooting MCP Connection

1. **Check server is running**:
   ```bash
   # Test standalone
   context-creator --rmcp
   # Should show: "Starting Context Creator MCP server (stdio mode)"
   ```

2. **Verify Claude configuration**:
   - Ensure path to context-creator is absolute
   - Check file has execute permissions
   - Verify `--rmcp` argument is included

3. **Check logs**:
   - Claude Desktop: Check developer console
   - Claude Code: Run with verbose flag `claude -v`

4. **Common issues**:
   - Path not found: Use full absolute path
   - Permission denied: `chmod +x /path/to/context-creator`
   - Already configured: Remove old config first

## Usage Examples

### Basic Usage
```bash
# Process current directory
context-creator

# Save to file instead of piping to LLM
context-creator -o context.md

# Process specific directories
context-creator src/ tests/ docs/
```

### Pattern Matching
```bash
# Include specific file types (quote to prevent shell expansion)
context-creator --include "**/*.py" --include "src/**/*.{rs,toml}"

# Exclude patterns
context-creator --ignore "**/*_test.py" --ignore "**/migrations/**"

# Combine includes and excludes
context-creator --include "**/*.ts" --ignore "node_modules/**" --ignore "**/*.test.ts"
```

### Remote Repositories
```bash
# Analyze any GitHub repository
context-creator --repo https://github.com/rust-lang/rust --prompt "How does the borrow checker work?"

# With specific patterns
context-creator --repo https://github.com/facebook/react --include "**/*.js" --prompt "Explain the reconciliation algorithm"
```

### Advanced Combinations
```bash
# Read prompt from stdin
echo "Find security vulnerabilities" | context-creator --stdin src/

# Copy output to clipboard (macOS)
context-creator --include "**/*.py" --copy

# Cap output to specific token limit
context-creator --max-tokens 100000 --prompt "Analyze the API endpoints"

# Enable verbose logging for debugging
context-creator -vv --prompt "Why is this slow?"
```

## Performance

Benchmarked on large codebases:

| Codebase | Files | context-creator | Alternative Tools |
|----------|-------|-----------------|-------------------|
| Next.js  | 5,000 | 3.2s           | 45s+             |
| Rust std | 8,000 | 5.1s           | 2min+            |
| Linux    | 70,000| 28s            | 10min+           |

## Token Management

When using `--prompt`, context-creator automatically:
- Measures prompt tokens
- Reserves space for LLM response
- Prioritizes files to fit within limits
- Removes least important files first

```bash
# With 2M token limit and 50-token prompt:
# Available for code: 2,000,000 - 50 - 1,000 = 1,998,950 tokens
context-creator --prompt "Analyze auth flow" --max-tokens 2000000
```

## Language Support

| Feature | Python | TypeScript/JavaScript | Rust | Other Languages |
|---------|--------|--------------------|------|-----------------|
| Basic concatenation | ✅ | ✅ | ✅ | ✅ |
| Import tracing | ✅ | ✅ | ✅ | ❌ |
| Caller detection | ✅ | ✅ | ✅ | ❌ |
| Type extraction | ✅ | ✅ | ✅ | ❌ |
| Dependency graph | ✅ | ✅ | ✅ | ❌ |

For unsupported languages, `context-creator` still provides intelligent file prioritization, Git-based importance scoring, and fast concatenation.
