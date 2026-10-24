# Configuration reference

Use `context-creator --config PATH` for an explicit TOML file. Otherwise, the
first existing file in this list is loaded:

1. `.context-creator.toml` in the current working directory.
2. `.contextrc.toml` in the current working directory.
3. `~/.context-creator.toml`.

These files are alternatives, rather than layers merged together. There is no
system config, automatic XDG config, or environment-based config override.
`RUST_LOG` controls logging; `RAYON_NUM_THREADS` controls the worker pool.

## Supported TOML fields

```toml
# Arrays belong at the top level, before any [table].
ignore = ["*.log", "generated/**"]
include = ["src/**", "Cargo.toml"]

[defaults]
max_tokens = 100000
llm_tool = "codex"            # gemini, codex, claude, ollama
model = "YOUR_MODEL"          # optional model name or provider alias
tokenizer = "o200k-base"      # o200k-base (default), cl100k-base
output_reserve = 1000         # reserved when a prompt is present
progress = false
verbose = false               # enables -v
quiet = false
directory = "."
# output_file = "context.md"

[tokens]
gemini = 100000
codex = 100000
claude = 100000
ollama = 8192

[[priorities]]
pattern = "src/core/**"
weight = 100.0

[[priorities]]
pattern = "tests/**"
weight = 50.0
```

Only the fields above are implemented. Output format, semantic expansion, and
git options are CLI flags; `[format]`, `[processing]`, `[cache]`, `[output]`, and
provider-specific tables are not supported configuration sections.

Explicit CLI `--model`, `--tokenizer`, `--max-tokens`, include/ignore patterns,
and output paths take precedence over their configuration defaults. Existing
tool defaults fill unset or default-valued CLI settings; explicitly
selecting the built-in default tool does not distinguish it from an unset tool.
Explicit `--quiet` and `--verbose` override conflicting logging defaults.

Custom priority rules are first-match-wins. Give narrow patterns before broader
ones. Files are selected by descending priority, then ascending path.

## Token budgets

The effective budget follows this order:

1. Explicit CLI `--max-tokens`.
2. `[tokens]` for the selected tool when a prompt is present.
3. `[defaults].max_tokens`.
4. Built-in tool budget when a prompt is present.
5. No automatic limit for context-only CLI usage.

With a prompt, its local token count and the response allowance (`--output-reserve`,
default 1,000) are subtracted once. MCP context-only requests use the tool budget if `max_tokens`
is omitted. Budgets are working defaults, not a lookup of the chosen model's
context window. Use explicit values appropriate to your model and output needs.

`o200k-base` is the default local encoding; `cl100k-base` preserves the previous
vocabulary. Literal token delimiters in source count as ordinary text. Neither
encoding is an exact Claude/Gemini tokenizer. File selection estimates formatting
overhead for initial selection, then verifies the complete final document. All
input directories share one budget, including metadata, escaping, and headers.
Whole files are removed if necessary; source files are never cut mid-content.
A budget too small for the format header fails clearly before output is written.
The provider CLI's additional instructions and tool schemas are outside this
local text budget. `--style map` defaults to 4,096 tokens when no budget is set.

```bash
context-creator --tool codex --model YOUR_MODEL --max-tokens 100000 --prompt "Explain this code"
context-creator --tokenizer cl100k-base --max-tokens 50000
context-creator --tool ollama --model YOUR_LOCAL_MODEL --prompt "Review this code"
```

`--ollama-model` remains supported. If supplied alongside `--model`, both values
must agree for Ollama. Omitting a model for other providers preserves their CLI
configuration and account-specific defaults.

## File selection

`.gitignore`, `.contextignore`, `.contextkeep`, and include/ignore patterns affect
which files are available. Hidden files and symlinks are excluded by default.
Semantic resolution stays inside the detected project root and respects the
configured traversal depth.

```gitignore
# .contextignore
node_modules/
target/
*.log
.env
```

See [usage](usage.md), [MCP setup](mcp-server.md), and the
[tested example configuration](../examples/config-examples/comprehensive.toml).
