# context-creator

[![CI](https://github.com/matiasvillaverde/context-creator/actions/workflows/ci.yml/badge.svg)](https://github.com/matiasvillaverde/context-creator/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=flat&logo=rust&logoColor=white)](https://www.rust-lang.org/)

High-performance CLI for building quality context windows that make AI assistants actually understand your codebase.

## The Problem

AI coding assistants are only as good as the context you provide. Most tools simply concatenate files, leading to:

- **Irrelevant files** cluttering the context window
- **Missing dependencies** that are crucial for understanding
- **Token limits** wasted on unimportant code
- **No understanding** of how your code actually connects

## The Solution

context-creator uses tree-sitter to build a dependency graph of your codebase, selecting only the files relevant to your task. It's like repomix, but faster and smarter.

### Without context-creator
```bash
# Generic context that includes everything
cat src/**/*.ts > context.txt  # 500K tokens of mostly noise
```

### With context-creator
```bash
# Intelligent context that follows your code's actual dependencies
context-creator --prompt "How does the authentication work?"
# Returns: auth files + their actual dependencies + related tests = 50K relevant tokens
```

## Key Advantages

- **Dependency-aware**: Uses tree-sitter AST parsing to understand imports, not just file names
- **Fast**: Rust-powered parallel processing handles massive codebases in seconds
- **Smart selection**: Includes only files connected to your query through the dependency graph
- **Multi-language**: Semantic analysis for Python, TypeScript, JavaScript, and Rust
- **MCP integration**: Works as a server for AI assistants to query your codebase programmatically

## Installation

```bash
npm install -g context-creator-mcp@latest
```

For platform-specific MCP client setup, see [Installation Guide](docs/installation.md).

## Usage

### CLI

```bash
# Analyze current directory
context-creator

# Build focused context for specific task
context-creator --prompt "Find security vulnerabilities in the auth system"

# Trace dependencies of specific files
context-creator --trace-imports --include "**/auth.py"

# Compare changes with dependency context
context-creator diff HEAD~1 HEAD
```

### MCP Server

Add to your MCP client configuration:

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

Then in your AI assistant:
```
"Explain how the payment system works" # AI will use analyze_local to build relevant context
"Find all SQL injection vulnerabilities" # Searches with full dependency understanding
```

## Features

- Tree-sitter AST parsing for true code understanding
- Import tracing and dependency resolution
- Parallel processing with Rayon
- Token budget management
- Git history integration
- MCP server with programmatic access

## MCP Tools

- `analyze_local` - Analyze local codebases with dependency awareness
- `analyze_remote` - Analyze Git repositories
- `search` - Text pattern search
- `semantic_search` - AST-based code search
- `file_metadata` - File information
- `diff` - File comparison

## Configuration

### .contextignore

```gitignore
node_modules/
target/
*.log
.env
```

### .contextkeep

```gitignore
src/core/**
src/api/**
```

### .context-creator.toml

```toml
[defaults]
max_tokens = 200000

[[priorities]]
pattern = "src/core/**"
weight = 100
```

## Documentation

- [Installation Guide](docs/installation.md) - Detailed setup instructions
- [Usage Examples](docs/usage.md) - CLI commands and workflows
- [Configuration](docs/configuration.md) - Advanced configuration
- [MCP Server Guide](docs/mcp-server.md) - MCP integration details
- [Architecture](docs/architecture.md) - Technical implementation

## Requirements

- Node.js >= v18.0.0 (for npm package)
- or Rust >= 1.70.0 (for building from source)

## Building from Source

```bash
git clone https://github.com/matiasvillaverde/context-creator
cd context-creator
cargo build --release
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

MIT