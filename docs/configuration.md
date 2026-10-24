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
output_reserve = 1000         # reserved with a prompt or native counting
secret_scan = "off"           # off, redact, error
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
5. A 4,096-token default for maps; no automatic limit for other context-only styles.

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

## Accounting reports

```bash
context-creator src/ tests/ --output context.md --report context-report.json
```

`--report PATH` writes schema version 1 JSON containing selected/omitted file
counts, exact local context/input counts, encoding, model, source-scanning policy,
output bytes, response reservation, optional native counts, and stage timings.
Timings cover scanning/analysis, selection, rendering, native counting, and reporting counts;
generation time excludes output I/O and LLM execution. Existing report/context
artifacts are excluded from later runs. Use distinct paths with existing parent
directories. Reports do not contain prompts or file contents.

## Provider-native counting

```bash
# Set MODEL_ID to a full model ID supported by your API account.
# OPENAI_API_KEY is read only when native counting is requested.
context-creator --tool codex --model "$MODEL_ID" --native-token-count \
  --max-tokens 100000 --output-reserve 8000 \
  --output context.md --report context-report.json
```

This explicit flag sends the rendered source plus any prompt to the selected
provider's token-count endpoint. Codex uses `OPENAI_API_KEY`; Claude uses
`ANTHROPIC_API_KEY`; Gemini uses `GOOGLE_API_KEY` or `GEMINI_API_KEY` (Google's
variable takes precedence). Installed CLI login sessions are separate from API
credentials. Ollama native counting is unsupported. Use full API model IDs;
provider CLI aliases may not be accepted by an API.

Local and native counts are reported separately. A native input count plus the
response reservation must fit the declared budget before any output or LLM call.
Native counting does not rerun automatic selection or retry paid API requests.
It covers the supplied user text, excluding instructions, history, attachments,
and tools added later by an installed CLI. It is not a forecast of the CLI's
complete request or generated response. Ordinary tests use local HTTP fixtures;
live account access is not part of the test suite.

Sources: [OpenAI input counting](https://developers.openai.com/api/docs/guides/token-counting/),
[Claude count endpoint](https://platform.claude.com/docs/en/api/messages/count_tokens),
[Gemini counting](https://ai.google.dev/gemini-api/docs/tokens).

## Secret-content policy

`--secret-scan off|redact|error` applies to source contents during normal context
generation. The default is `off`; `[defaults].secret_scan` can set a default,
and an explicit CLI value wins. `redact` replaces recognized values before
semantic parsing, rendering, token accounting, native counting, or an LLM call.
`error` fails before writing output when a scanned eligible file matches.
Ignored files stay excluded; semantic expansion cannot reintroduce ignored
source contents. Prompt values are not written to verbose logs.

Rules recognize private-key blocks and distinctive OpenAI, Anthropic, GitHub,
Google, and AWS credential forms. They do not perform entropy detection, verify
credentials, or recognize every secret format. Generated examples can match
these forms too. Errors contain rule IDs and file/line locations without matched
values. Redaction preserves line counts for maps and paginated reads. Use ignore
rules to exclude sensitive files even when their contents do not match a rule.

Reporting, native counting, and source scanning flags apply to normal context
generation; subcommands do not silently accept them. MCP tools have their own
request fields, described in the [server guide](mcp-server.md).

## File selection

`.gitignore`, `.contextignore`, the legacy `.context-creator-ignore`, and CLI
include/ignore patterns affect file selection. `.contextkeep` adds a 100-point
priority boost to matching visible files; explicit custom priority rules take
precedence and ignore rules still apply. Hidden files and symlinks are excluded by default.
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
