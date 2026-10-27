# context-creator

[![CI](https://github.com/matiasvillaverde/context-creator/actions/workflows/ci.yml/badge.svg)](https://github.com/matiasvillaverde/context-creator/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

**Give your agent relevant code within a token budget.**

context-creator turns a task, a file, or Git changes into source context with
locations, selection reasons, and dependency relationships. Use the **Rust crate,
CLI, or MCP server** to build tools for code review, debugging, and repository
exploration.

Context building, repository maps, and source reads run locally without an LLM
call or model API key. Your agent uses the returned code with the model you choose.

[Quickstart](#quickstart) · [Agent integrations](#agent-integrations) ·
[Languages and limits](#languages-and-limits) · [Documentation](#documentation)

## Quickstart

Requires **Rust 1.89+**, Git, and a C/C++ build toolchain. Install from the current
source so the examples below match your binary:

```bash
git clone https://github.com/matiasvillaverde/context-creator.git
cd context-creator
cargo install --path . --locked
```

Try the included session-expiration example; no Zig compiler is needed:

```bash
context-creator context --root examples/sample-zig-project \
  --task "Fix session expiration" --recipe debug_failure \
  --max-tokens 8000 --secret-scan redact
```

The command prints **JSON to stdout**: selected source, source locations, reasons,
relationships, and known gaps. A few selected symbols from this example are:

| Source | Symbol | Why it was included |
| --- | --- | --- |
| `src/session.zig:3` | `Session` | Task terms match the source |
| `src/handler.zig:4` | `authorize` | Task terms match the source |
| `src/policy.zig:3` | `elapsed` | Resolved local import from `src/session.zig:1` |

The response includes complete functions/types when they fit, or signatures when
bodies do not fit. `omissions`, `gaps`, and `truncated_metadata` tell your agent
what needs another read. In this example, Zig's external `std` imports are
reported as gaps rather than supplied as local source.

For your own code, change `--root` to your repository. Use `--seed` when you
already know the entry file:

```bash
context-creator context --root examples/sample-zig-project \
  --seed src/main.zig --max-tokens 8000 --secret-scan redact
```

[Other installation methods](docs/installation.md) include installing directly
from Git, released binaries, and the npm MCP launcher. Published releases may
have fewer features than the current source.

## Build tools for agents

| Tool you build | What context-creator provides |
| --- | --- |
| Debug a failure | Task matches, related declarations, tests, configuration, and static dependencies |
| Review a change | Changed symbols, likely dependents, related tests, and evidence for each selection |
| Explore a repository | Compact declaration maps and paginated source reads |
| Keep context current | Revision-checked sessions that deliver changed or newly selected code |

The built-in `debug_failure`, `review_change`, and `explain_feature` recipes share
parameters across the crate, CLI, and MCP. Sessions track delivered snippets;
callers retain a revision cursor and apply deltas or explicit resets. See the
[agent tools guide](docs/agent-tools.md) for recipes, sessions, contract hints,
and the response schema.

### More CLI workflows

Run these inside the repository you want to inspect:

```bash
# Start with declarations and source locations
context-creator . --style map --max-tokens 4096 --secret-scan redact --quiet

# Collect current changes and their static impact relative to a Git commit
context-creator context --root . --recipe review_change --base-ref HEAD \
  --max-tokens 8000 --secret-scan redact

# Write artifacts outside the source root
context-creator . --max-tokens 16000 --secret-scan redact \
  --output ../agent-context.md --report ../agent-context.json --quiet

# Discover the built-in recipes
context-creator recipes
```

Maps go to stdout. Artifact generation writes the requested files outside the
source root to keep generated context out of future walks. Its report includes
selected/omitted file counts, token usage, and timings. Artifact formats
also include XML, plain text, and paths. Optional
[provider CLI handoff](docs/configuration.md#token-budgets) can send context and a
prompt to an installed Gemini, Codex, Claude, or Ollama CLI.

## Agent integrations

| Integration | Use it when |
| --- | --- |
| CLI | Your agent runtime can launch a subprocess and parse stdout |
| [MCP](#mcp) | Your agent already supports MCP tools |
| [Rust crate](#rust-crate) | You want typed requests and reusable in-process analysis |

### MCP

The native binary starts a stdio server with `context-creator --rmcp`.
For clients using the `mcpServers` configuration format:

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

Use the binary's full path if it is not on the client's `PATH`. Other clients
have their own configuration format; the server command stays the same.

Your agent can call `build_context` with parameters such as:

```json
{
  "root": "/absolute/path/to/repo",
  "task": "Fix session expiration",
  "recipe": "debug_failure",
  "max_tokens": 8000
}
```

For progressive exploration, use `repository_map`, then `read_file` for selected
line ranges. `read_file` can also include transitive dependency context with
`dependency_tokens`. See the [MCP guide](docs/mcp-server.md) for tool schemas,
HTTP transport, and client setup.

### Rust crate

These examples use the API on `main`:

```toml
[dependencies]
context-creator = { git = "https://github.com/matiasvillaverde/context-creator", branch = "main" }
anyhow = "1"
```

```rust
use context_creator::agent::{AgentService, ContextRequest};

fn main() -> anyhow::Result<()> {
    let service = AgentService::new();
    let request = ContextRequest::new(".", "Fix session expiration");
    let context = service.build_context(request)?;
    for snippet in context.snippets {
        println!("{}:{} {:?}", snippet.path, snippet.start_line, snippet.representation);
    }
    Ok(())
}
```

Keep one `AgentService` to reuse analysis and session state. Calls are blocking;
use a blocking worker in async applications. No MCP server is needed for crate
integration. See the [crate integration guide](docs/agent-tools.md#crate-integration).

## Filtering and budgets

Use `.contextignore` to exclude files, `.contextkeep` to boost visible files, and
`.context-creator.toml` for CLI defaults. Ignore rules still apply to kept files.
For example:

```gitignore
# .contextignore
.env
node_modules/
target/
```

Task context defaults to secret redaction. CLI examples above request it
explicitly. Scanning is heuristic; choose which files your agent may read through
filters. See [configuration](docs/configuration.md) for precedence and policies.

The task budget covers the **complete compact JSON response**, including its
metadata. Reserve additional space for your instructions, the MCP envelope, and
the model's reply. Token counts use a selected local encoding and can differ
from a provider's tokenizer.

## Languages and limits

Parsers cover [29 language families](docs/languages.md), including Rust, Python,
JavaScript, TypeScript, Go, Java, C/C++, C#, and Zig. Feature coverage varies by
language and syntax form.

[Workspace context](docs/workspaces.md) can follow local Cargo, Python, and
npm/Yarn/pnpm package bindings while preserving the selected root and ignore rules.

The [real-project audit](docs/oss-language-audit.md) checks 87 randomly selected
files from 29 public projects. It found unresolved JS/TS parent imports, missing
imports/declarations/calls/types, and Java/Kotlin analysis timeouts. These issues
remain open. The audit records both successful checks and failures.

Task matching uses lexical terms and static relationships. Inferred callers and
contract links do not establish runtime behavior or complete compiler bindings.
Dependency traversal, candidate selection, and response size have limits; inspect
reported gaps and omissions. Large roots can be slow, so use focused roots and
ignore patterns. See [analysis limits](docs/agent-tools.md#explanations-limits-and-safety)
and [reproducible measurements](benchmarks/README.md) for details.

External type references can include [declared dependency requirements and lockfile versions](docs/dependency-versions.md).

## Documentation

- [Agent tools](docs/agent-tools.md) — Task context, recipes, sessions, and change impact
- [Installation](docs/installation.md) — Native builds, released packages, and MCP launchers
- [MCP server](docs/mcp-server.md) — Tools, transports, and client configuration
- [Configuration](docs/configuration.md) — Filtering, token accounting, and secret policy
- [Language support](docs/languages.md) — Extraction and resolution coverage
- [Documentation index](docs/README.md) — Guides, source entry points, and research

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for setup and checks. Report bugs and feature
requests in [GitHub Issues](https://github.com/matiasvillaverde/context-creator/issues).

## License

[MIT](LICENSE)
