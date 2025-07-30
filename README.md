# context-creator

[![CI](https://github.com/matiasvillaverde/context-creator/actions/workflows/ci.yml/badge.svg)](https://github.com/matiasvillaverde/context-creator/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=flat&logo=rust&logoColor=white)](https://www.rust-lang.org/)

CLI tool and MCP server for analyzing codebases and providing context to LLMs.

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

# Analyze with prompt
context-creator --prompt "Find security vulnerabilities"

# Search codebase
context-creator search "TODO" --no-semantic

# Compare git changes
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

## Features

- Multi-language semantic analysis (Python, TypeScript, JavaScript, Rust)
- AST-based import tracing and dependency resolution
- Parallel processing with Rayon
- Token budget management for LLM context windows
- Git history integration
- MCP server with programmatic access

## MCP Tools

- `analyze_local` - Analyze local codebases
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