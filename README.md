# context-creator

[![CI](https://github.com/matiasvillaverde/context-creator/actions/workflows/ci.yml/badge.svg)](https://github.com/matiasvillaverde/context-creator/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=flat&logo=rust&logoColor=white)](https://www.rust-lang.org/)

Code context building blocks for AI agents, available as a **Rust crate, CLI, and MCP server**.

Use context-creator to build tools that let agents explore repositories, inspect
source, and assemble dependency-aware context within a token budget. Embed the
crate in your application, wrap the CLI in your agent framework, or connect the
MCP server to an existing coding agent.

Your agent chooses the task, the files to investigate, and the model to use.
context-creator handles file discovery, filtering, semantic expansion, formatting,
and local token accounting. Context generation, repository maps, and source reads
work locally without an LLM call. Provider CLI handoff is optional.

## Build Tools for Agents

Choose the integration that fits your application:

| Integration | How to use it | What your agent receives |
| --- | --- | --- |
| [Rust crate](#rust-crate) | Call typed helpers in-process or compose the core modules. | Typed maps and source pages, or generated context artifacts. |
| [CLI](#cli) | Run a subprocess from Python, TypeScript, or another agent runtime. | Context on stdout or in a file, with an optional JSON accounting report. |
| [MCP server](#mcp-server) | Expose the existing tools through stdio or Streamable HTTP. | Maps, paginated source, search results, metadata, and context through tool calls. |

For example, you can combine these building blocks into your own tools:

| Tool you build | Building blocks |
| --- | --- |
| `explore_repository` | A bounded repository map, search, and selected source pages. |
| `review_change` | Git diffs and focused context expanded with imports, callers, and types. |
| `investigate_failure` | Source search, related files, and optional OpenTelemetry enrichment. |

A typical agent workflow is:

1. **Map:** request declarations and source locations under a small token budget.
2. **Read:** inspect the relevant files or line ranges; request the next page when needed.
3. **Expand:** add related imports, callers, or types, then return the context to
   your agent's model for reasoning, editing, or review.

For task-oriented tools, use the shared **`build_context`** API. It selects
complete functions or signatures, related types/tests/configuration, and static
dependencies, with reasons and known gaps. Revision-checked sessions return
changed or newly selected source. Recipes support debugging, explaining features,
and reviewing Git changes; contract hints connect literal HTTP calls, OpenAPI
JSON operations, and handlers across languages.

```bash
context-creator context --root . --task "Fix session expiration" \
  --recipe debug_failure --max-tokens 8000
context-creator context --root . --recipe review_change --base-ref HEAD
context-creator recipes
context-creator context --root . --seed src/main.zig --max-tokens 12000
```

These commands return bounded JSON and make no model calls. The same request and
response types are available through `agent::AgentService` and the MCP
`build_context`/`list_recipes` tools. See the [agent tools guide](docs/agent-tools.md)
for integration, custom recipes, session updates, evidence, and limits.

The `context` command honors top-level options and TOML filters, budgets,
tokenizer, and secret policy; options after `context` take precedence.

Seed one file to collect its transitive local import graph, complete declarations,
and nearby callers/tests. MCP `read_file` can also return this context alongside a
source page by setting `dependency_tokens`. Check omissions and gaps before
assuming all dependencies fit the response.

## Key Advantages

- **Dependency-aware**: Uses tree-sitter AST parsing to extract imports, declarations, calls, and types
- **Predictable budgets**: Fits the complete rendered output across all input directories
- **Focused selection**: Uses file filters, priority rules, and dependency relationships
- **Multi-language**: Real parsers for [29 code, query, infrastructure, and markup language families](docs/languages.md), including Zig, Bash, Dart, Lua, Scala, SQL, and Terraform; the support matrix distinguishes direct resolution from inferred relationships
- **Reusable interfaces**: The crate, CLI, and MCP server support custom agent tools

## Installation

For the native CLI, install from a checkout with Rust 1.89 or newer:

```bash
cargo install --path . --locked
```

See [Building from Source](#building-from-source) for cloning and build commands.
The examples below describe the APIs and flags in this checkout.

For MCP clients, the published npm wrapper is also available:

```bash
npm install -g context-creator-mcp@latest
```

The npm wrapper starts MCP mode automatically. Use the native `context-creator`
binary for CLI context generation. Published packages follow their release
versions; see the [Installation Guide](docs/installation.md) for client setup.

## Usage

### CLI

```bash
# Give an agent a compact overview of the repository
context-creator . --style map --max-tokens 4096 --secret-scan redact --quiet

# Build context around files selected by the agent
context-creator . --include "**/auth.py" \
  --trace-imports --include-callers --include-types \
  --max-tokens 16000 --secret-scan redact \
  --output agent-context.md --report agent-context.json --quiet

# Get changes for a review tool
context-creator diff HEAD~1 HEAD

# Enrich code with OpenTelemetry runtime metrics
context-creator telemetry -t traces.json
```

The first command returns a map on stdout. The second writes source context and
a JSON report with selected/omitted file counts, token counts, and timings. Your
tool can pass the artifact to any model or agent framework.

For example, a Python agent can wrap the CLI as a regular function:

```python
import subprocess


def repository_overview(repo: str) -> str:
    result = subprocess.run(
        [
            "context-creator", repo,
            "--style", "map", "--max-tokens", "4096",
            "--secret-scan", "redact", "--quiet",
        ],
        check=True,
        capture_output=True,
        text=True,
        timeout=60,
    )
    return result.stdout
```

Register the function as a tool using your framework's tool API. A failed command
raises an exception, so the agent can handle errors separately from context.

### Rust Crate

Embed the same typed map helper used by the MCP server in your own Rust tool.
This Git dependency follows the current repository's API:

```toml
[dependencies]
context-creator = { git = "https://github.com/matiasvillaverde/context-creator", branch = "main" }
anyhow = "1"
```

```rust
use context_creator::mcp_server::{
    progressive::repository_map_sync, RepositoryMapRequest, RepositoryMapResponse,
};
use std::path::PathBuf;

pub fn repository_overview(path: PathBuf) -> anyhow::Result<RepositoryMapResponse> {
    repository_map_sync(RepositoryMapRequest {
        path,
        include_patterns: vec![],
        ignore_patterns: vec![],
        max_tokens: Some(4096),
        tokenizer: None,
        secret_scan: None,
    })
}
```

The response contains `map`, `token_count`, `file_count`, and `omitted_files`.
`secret_scan: None` uses this helper's default redaction policy. Use
`read_file_sync(ReadFileRequest)` for paginated source, or `context_creator::run(Config)`
to generate full context artifacts. These are blocking functions; call them on a
blocking worker when integrating with an async agent. An MCP server does not need
to be running to use these helpers.

### Optional Provider CLI Handoff

You can also ask context-creator to send the generated context and a prompt to an
installed provider CLI:

```bash
context-creator --tool codex --model YOUR_MODEL --max-tokens 100000 \
  --output-reserve 8000 --prompt "Review this code"
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

To expose the existing tools directly, add the native binary to your MCP client
configuration. Set `command` to its full path if it is not on `PATH`:

```json
{
  "mcpServers": {
    "context-creator": {
      "command": "context-creator",
      "args": ["--rmcp"]
    }
  }
}
```

An agent can request a map, then read a selected source range. These are example
`tools/call` parameters:

```json
{
  "name": "repository_map",
  "arguments": { "path": "/workspace/repo", "max_tokens": 4096 }
}
```

```json
{
  "name": "read_file",
  "arguments": {
    "root": "/workspace/repo",
    "path": "src/auth.py",
    "start_line": 1,
    "max_lines": 120,
    "max_tokens": 2048
  }
}
```

Use the returned `next_line` as `start_line` to continue reading. The read budget
counts source content; your agent must also allow for the MCP response envelope
and its own instructions. Both tools default to redaction and make no LLM calls.
See the [MCP Server Guide](docs/mcp-server.md) for tool schemas, limits, HTTP setup,
and the alternative npm launcher.

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

- `build_context` - Task context, selection explanations, change impact, contract hints, and session deltas
- `list_recipes` - Discover built-in and supplied custom tool recipes
- `repository_map` - Get bounded declarations and source locations without an LLM call
- `read_file` - Page through source under line and token limits
- `analyze_local` - Build local code context, with optional provider CLI handoff
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

- [Agent Tools Guide](docs/agent-tools.md) - Task context, reusable recipes, sessions, and change impact
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
