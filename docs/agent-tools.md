# Building tools for agents

`build_context` assembles local code for a task, with source locations, selection
reasons, omissions, static relationships, and revision-checked session updates.
It is available through `agent::AgentService`, the `context` CLI subcommand, and
both MCP implementations. It makes no LLM or embedding API calls.

## Task context

```bash
context-creator context --root . --task "Fix session expiration" \
  --recipe debug_failure --max-tokens 8000
```

The command writes compact JSON to stdout. `--task` ranks paths, identifiers, and
source using normalized lexical terms. This is deterministic local retrieval,
not an embedding search or an automatically generated implementation plan. Use
`--seed src/auth.py` (repeatable) to anchor a task to known files or directories.
Imports, static dependents, matching tests/configuration, and supported contracts
expand those anchors. `--include` narrows initial task matches; dependencies can
still be included. `--ignore` and repository ignore rules exclude files throughout.

Functions are returned whole. When a function does not fit, its declaration
signature can be returned instead. Types containing nested methods use signatures
plus separately selected methods. Imports and other top-level source appear as
`file_context` snippets. Unsupported source formats use whole-file candidates.
No function body is cut off halfway to meet a budget.

`max_tokens` bounds the compact serialization of the **complete ContextResponse**,
including content, reasons, revision fields, and metadata. `token_count` measures
that same JSON with `tokenizer` (`o200k-base` by default; `cl100k-base` is optional).
A budget too small for metadata is an error. The outer MCP/JSON-RPC envelope,
client instructions, and model response require additional room in your agent.
Pretty-printing the JSON changes token usage.

## Crate integration

```rust
use context_creator::agent::{AgentService, ContextRequest};

fn main() -> anyhow::Result<()> {
    let tools = AgentService::new();
    let request = ContextRequest::new(".", "Fix session expiration");
    let context = tools.build_context(request)?;
    for snippet in context.snippets {
        println!("{}:{} {:?}", snippet.path, snippet.start_line, snippet.reasons);
    }
    Ok(())
}
```

Keep one service per application/workspace workflow to reuse analysis and
sessions. Calls are blocking; use a blocking worker inside async applications.
The service serializes its builds so a shared session cannot accept two updates
from the same cursor. Source files are walked and fingerprinted on each call;
unchanged requests reuse declarations, dependency analysis, and contract links.
After edits, changed declarations are re-extracted and project relationships are
rebuilt. This is an in-process cache, not a persistent database or file watcher.

## Recipes

```bash
context-creator recipes
context-creator context --task "Explain refresh tokens" --recipe explain_feature
context-creator context --recipe review_change --base-ref HEAD
```

| Recipe | Behavior |
| --- | --- |
| `explain_feature` | Task/seed selection, tests, configuration, contracts, two expansion steps. |
| `debug_failure` | The same building blocks with three expansion steps. |
| `review_change` | Requires `base_ref`; prioritizes changed declarations and related static impact, with two expansion steps. |

Custom recipes extend a built-in and set bounded options. For example, save this
as `agent-recipes.toml`:

```toml
[recipes.auth_review]
extends = "debug_failure"
include_patterns = ["src/auth/**", "tests/auth/**"]
ignore_patterns = ["vendor/**"]
include_tests = true
include_config = true
follow_contracts = true
depth = 3
max_snippets = 32
```

```bash
context-creator recipes --recipe-file agent-recipes.toml
context-creator context --task "Fix session expiration" \
  --recipe auth_review --recipe-file agent-recipes.toml
```

Use `RecipeBook`/`RecipeDefinition` in the crate. MCP `list_recipes` accepts
`{"recipes":{...}}`; pass the same definitions as `custom_recipes` to
`build_context`. Recipe definitions are supplied per request, without server-side
configuration writes. There are at most 32 custom recipes, 64 patterns per recipe,
five expansion steps, and 128 selected snippets. Explicit seeds and changed files
can still select tests/configuration when automatic inclusion is disabled.

## Incremental sessions

For the first request, provide `session_id`. Apply its full response, retain the
returned `revision`, then send that cursor as `expected_revision` on the next
request. `source_revision` fingerprints the visible source snapshot; `revision`
identifies a delivered context transition. A changed snapshot sets `stale: true`,
including when an edit was outside the selected snippets.

Apply responses in this order:

1. If `reset` is true, clear the prior context. Otherwise, verify
   `previous_revision` matches your last cursor and remove IDs listed in `removed`.
2. Store/update each snippet by `id`. A snippet with `content` replaces its old
   content. An absent `content` means the exact `content_hash` was already sent;
   preserve that content while updating locations and reasons.
3. Save `revision` as the next cursor. A smaller budget or different task can
   remove snippets that are no longer selected.

The server rejects out-of-order cursors. If a response was lost or your agent
forgot its local context, send `reset_session: true` with the session ID and omit
`expected_revision` to obtain a full replacement. If removal metadata cannot fit,
the service also returns an explicit full replacement rather than an incomplete
delta. Sessions are isolated by repository, tokenizer, secret policy, and ignore
scope, and are limited to 128 per service. At most two workspace snapshots are
cached per service. A server restart clears in-memory state.

Short-lived CLI processes can use an explicit checkpoint **outside the source
root**:

```bash
context-creator context --root /workspace/repo --task "Fix session expiration" \
  --session-id auth --session-file /tmp/auth-context-session.json

# Use the revision from the previous response:
context-creator context --root /workspace/repo --task "Fix session expiration" \
  --session-id auth --session-file /tmp/auth-context-session.json \
  --expected-revision PREVIOUS_REVISION
```

Checkpoint files store fingerprints and session metadata, never source content.
They are written atomically and guarded by a `.lock` file against concurrent CLI
updates. After an interrupted process, remove a stale lock only once the process
has stopped. The library exposes `build_context_with_checkpoint` for trusted local
paths; arbitrary checkpoint paths are deliberately absent from MCP requests.
Checkpoint identity must match the repository, content policy, and session ID.

## Change impact

`base_ref` compares a Git commit/tree with the current index and working tree,
including staged edits, unstaged edits, untracked files, deletions, and detected
renames. The response describes statuses, new-source hunk ranges, and removed
symbol names. Current source is returned, not the historical version at the base.

Resolved local imports, static dependents, text references to removed symbols,
related tests, and matching contracts help collect impact context. A changed
range is known Git evidence; inferred callers, name matches, and runtime effects
are labeled accordingly. This does not prove which tests cover a change or which
runtime behaviors will break. Ignored and hidden changes are not exposed.

## Connections across services and languages

The contract index links matching literal HTTP methods and normalized relative
paths across clients, route handlers, tests, and OpenAPI **JSON** operations.
Local `#/components/schemas/Name` references can link to source types named
`Name`, including generated client types. These are `http_contract` and
`schema_type` relationships with `inferred` confidence and concrete evidence.

For example, a TypeScript `fetch('/api/session/refresh', {method: 'POST'})` can
link to an OpenAPI operation and a Python `@app.post('/api/session/refresh')`
handler. Python decorators, common JavaScript/TypeScript client/router calls,
Go method calls, Rust route attributes, and basic literal route-builder calls
are recognized. This is a bounded syntax-based recognizer, not complete support
for every framework. Computed URLs/options, service origins, route prefixes,
OpenAPI YAML/external references, arbitrary RPC/message protocols, and runtime
routing need additional adapters. Matching handler names alone is insufficient.
Multiple possible handlers produce an ambiguity gap.

## Explanations, limits, and safety

Each snippet reports its path, symbol when present, original source range,
`full`/`signature` representation, content fingerprint, and selection reasons.
`relations` includes evidence and `resolved`/`inferred` confidence. `gaps` reports
malformed syntax, unresolved local imports/types, ambiguous or dynamic contracts,
and static-analysis limits. `omissions` explains unselected candidates;
`omitted_snippets` counts candidates omitted or covered by enclosing snippets.
`truncated_metadata` counts explanations/relationships/change entries omitted to
fit. Neither this response nor the file list is a completeness guarantee.

Agent context defaults to `secret_scan: "redact"` in all interfaces. `error` fails
before context is returned; `off` is an explicit opt-out. Scanning is heuristic.
Paths remain under the supplied root; explicit seeds cannot bypass ignores,
hidden-file, binary, or symlink filtering. Requests are limited to 5,000 visible
files, 2 MiB per source file, 64 MiB total source, 64 seeds, and a 16 KiB task.
The selection considers up to 128 ranked files, 256 declarations per file, and
512 snippet candidates; expansion and contract limits are reported as gaps.
Large repositories should use narrower roots or ignore patterns.

## Validation and measurements

```bash
cargo test --locked --test lib agent_context_test::
cargo test --locked --test progressive_mcp_test --test rmcp_integration_test
cargo run --release --locked --example agent_workflow
```

The workflow experiment creates a fixed expiration task with 120 unrelated files.
It checks five expected source/test/configuration paths, exact response budgets,
zero repeated source on an unchanged session, and a single-file edit. It reports
cold/warm wall time, response tokens, delivered source bytes, and index reuse.
This measures retrieval and incremental behavior; live model task-completion
quality requires a separate evaluation.
