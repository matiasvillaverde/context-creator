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

context-creator combines file filters, priorities, token budgets, and optional tree-sitter dependency expansion to build focused context. You choose the seed files; import, caller, and type analysis can add related source.

### Without context-creator
```bash
# Generic context that includes everything
cat src/**/*.ts > context.txt  # 500K tokens of mostly noise
```

### With context-creator
```bash
# Start with authentication files and include their local dependencies
context-creator --include "**/auth.py" --trace-imports --max-tokens 50000
```

## Key Advantages

- **Dependency-aware**: Uses tree-sitter AST parsing to understand imports, not just file names
- **Predictable budgets**: Fits the complete rendered output across all input directories
- **Focused selection**: Uses file filters, priority rules, and dependency relationships
- **Multi-language**: Semantic extraction and local resolution for [13 language families](docs/languages.md), including C/C++, C#, Kotlin, Ruby, and PHP
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

# Select a model available in your installed provider CLI
context-creator --tool codex --model YOUR_MODEL --max-tokens 100000 --prompt "Explain the architecture"

# Select the local tokenizer (o200k-base is the default)
context-creator --tokenizer cl100k-base --max-tokens 50000

# Navigate declarations without implementation bodies
context-creator --style map --max-tokens 4096

# Reserve room for a response
context-creator --output-reserve 8000 --max-tokens 100000 --prompt "Review this code"

# Produce a checked artifact and machine-readable accounting
context-creator --secret-scan redact --output context.md --report context-report.json

# Trace dependencies of specific files
context-creator --trace-imports --include "**/auth.py"

# Compare changes with dependency context
context-creator diff HEAD~1 HEAD

# Enrich code with OpenTelemetry runtime metrics
context-creator telemetry -t traces.json
```

Model selection is passed through to Gemini CLI, Codex, Claude Code, or Ollama. Omit
`--model` to use the provider CLI's configured default. `--ollama-model` remains
supported. Set a budget appropriate to your model; tool defaults are working
budgets, not discovered model limits. Local token counts are estimates for Claude
and Gemini. Final rendered output is checked against the selected local encoding;
multiple input directories share one budget. Whole files are retained, and a
budget too small for the format header returns an error before output is written.
Repository maps default to 4,096 tokens and show up to 64 declarations per file.
`--report` records actual local counts, file selection, output bytes, and timings.
`--native-token-count` optionally checks the supplied text with the selected
provider API; see [configuration](docs/configuration.md#provider-native-counting).
Secret scanning is explicit (`off`, `redact`, or `error`) for CLI context generation.
The two progressive MCP tools default to redaction.

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

- Tree-sitter imports, declarations, calls, and type references
- Import tracing and dependency resolution
- Parallel processing with Rayon
- Token budget management
- Git history integration
- MCP server with programmatic access
- OpenTelemetry integration for runtime metrics
- Shared tokenizer vocabularies with selectable `o200k-base` and `cl100k-base`
- RMCP 3.7 with stdio and Streamable HTTP at `/mcp`

## MCP Tools

- `repository_map` - Get bounded declarations and source locations without an LLM call
- `read_file` - Page through source under line and token limits
- `analyze_local` - Analyze local codebases with dependency awareness
- `analyze_remote` - Analyze Git repositories
- `search` - Text pattern search
- `semantic_search` - AST-based code search
- `file_metadata` - File information
- `diff` - File comparison

See [reproducible benchmarks](benchmarks/README.md) for performance and navigation-task measurements.

## Configuration

### .contextignore

```gitignore
node_modules/
target/
*.log
.env
```

### .contextkeep

Add a priority boost to matching visible files. Ignore rules still apply.

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
- [Modernization research and roadmap](docs/modernization.md) - Implementation plan, tool comparisons, and validation

## Requirements

- Node.js >= v18.0.0 (for npm package)
- or Rust >= 1.89.0 (for building from source)

## Building from Source

```bash
git clone https://github.com/matiasvillaverde/context-creator
cd context-creator
cargo build --release --locked
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

MIT
