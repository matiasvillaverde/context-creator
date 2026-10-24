# Modernization research and implementation

Research date: 2026-10-24. Two research workstreams reviewed 50 Exa search
results, then checked maintainer documentation and source. Search results can
overlap; this is not a count of independent experiments. Source code in this
checkout is the authority for implemented behavior.

## Comparable tools

| Tool | Useful design | Implication for context-creator |
| --- | --- | --- |
| [Repomix](https://repomix.com/guide/command-line-options) | Configurable token encoding, token tree, compression, MCP, and a budget guard that fails after generating oversized output. [Secret scanning](https://repomix.com/guide/security) is enabled by default. | Make encoding explicit, distinguish trimming from a CI overflow guard, and investigate signatures as a smaller context format. |
| [Gitingest](https://github.com/coderamp-labs/gitingest) | Simple URL/local ingestion. Its [formatter](https://github.com/coderamp-labs/gitingest/blob/main/src/gitingest/output_formatter.py) uses `o200k_base` and treats token delimiters as ordinary text. | Keep local workflows simple and count arbitrary source text safely. |
| [Aider](https://aider.chat/docs/repomap.html) | A compact repository map ranks definitions and signatures using dependencies. Its [implementation](https://github.com/Aider-AI/aider/blob/main/aider/repomap.py) caches extracted tags and rendered maps. | A map followed by focused file reads is a useful direction for agents; reuse the existing project analysis. |
| [Code2prompt](https://github.com/mufeedvh/code2prompt) | Rust CLI/library, templates, structured output, and MCP. Its README describes parallel per-file token counts plus estimated template overhead. | State the accounting contract precisely and benchmark comparable operations. |

The sources establish feature contracts. They do not establish which tool is
fastest or which context selection produces the best answers. No independent,
comparable throughput or answer-quality benchmark was found in this review.

## Current provider and protocol requirements

- [Codex noninteractive mode](https://learn.chatgpt.com/docs/non-interactive-mode)
  uses `codex exec`; `-` reads task input from stdin. This checkout uses
  `exec --skip-git-repo-check -` so arbitrary source directories work.
- [Gemini automation](https://geminicli.com/docs/cli/tutorials/automation/) uses
  `--prompt` for headless operation, with piped source as context. The
  [installation package](https://geminicli.com/docs/get-started/installation/)
  is `@google/gemini-cli`.
- [Claude headless mode](https://code.claude.com/docs/en/headless) uses `-p`;
  [model selection](https://code.claude.com/docs/en/model-config) accepts aliases
  and model identifiers.
- Model identifiers and account access change independently of this tool. The
  optional `--model` and MCP `model` fields pass through to the installed CLI;
  omitting them preserves its configured default. This avoids an embedded list
  of model names going stale. Explicit budgets remain necessary.
- [OpenAI token counting](https://developers.openai.com/api/docs/guides/token-counting/),
  [Anthropic token counting](https://platform.claude.com/docs/en/build-with-claude/token-counting),
  and [Gemini token counting](https://ai.google.dev/gemini-api/docs/tokens)
  describe provider-specific request accounting. Local BPE counts cannot account
  exactly for every model, tool schema, instruction, attachment, and output.
- [MCP 2026-07-28](https://modelcontextprotocol.io/specification/2026-07-28/changelog)
  introduces stateless discovery and request metadata. The official
  [Rust SDK migration guide](https://github.com/modelcontextprotocol/rust-sdk/discussions/969)
  describes the 3.x changes and legacy compatibility. This checkout uses RMCP
  3.7 and Streamable HTTP at `/mcp`, with modern and legacy wire tests.

## Changes implemented in this checkout

1. Shared, lazily initialized tokenizer vocabularies. `o200k-base` is the default;
   `cl100k-base` remains available. Request text caches stay separate. Source
   containing literal special-token delimiters counts as ordinary text.
2. Explicit headless provider invocation and a shared CLI/MCP command policy.
   Model and tokenizer selection work through the CLI, TOML, and both MCP
   implementations. Cache keys distinguish model/encoding choices.
3. MCP prompt reservation happens once through `Config` helpers. File counts
   use the selected file list, rather than counting headings in rendered source.
4. RMCP 0.2 → 3.7, Streamable HTTP replacing the old HTTP+SSE endpoints, Host
   validation, explicit local Origin validation, and shared request caches.
5. `git2` 0.20 → 0.21 and removal of the unused `lru` dependency. The original
   resolved dependencies reported RMCP
   [RUSTSEC-2026-0189](https://rustsec.org/advisories/RUSTSEC-2026-0189.html),
   two git2 unsoundness advisories, two lru unsoundness advisories, and an
   unmaintained macro dependency. The updated graph reports no vulnerabilities
   or warnings under `cargo audit` on the research date. Existing audit policy
   was preserved.
6. A tracked lockfile and Rust 1.89 minimum, verified with that compiler. RMCP
   requires at least 1.88; the resolved UUID dependency requires 1.89. CI checks
   the declared minimum and runs the standalone MCP suites.
7. Real Java imports, methods, calls, types, wildcard/static imports, nested-class
   source resolution, and Maven/Gradle root detection. Resolution is confined to
   the project; symlink escapes are rejected. The parser uses the existing Java
   dependency. Java classpaths, dependency JARs, overload dispatch, and all
   compiler-specific source sets are not modeled.
8. A fixture matrix for all thirteen implemented language families, module
   variants, malformed source, local imports, callers/importing clients, types,
   aliases, and symlink confinement. Empty analyzer modules are not registered
   as supported semantic languages. Caller
   matching separates languages and Java packages. Graph edges use indexed,
   complete-path lookups instead of repeated scans and substring matches.
9. Markdown fences survive embedded backtick runs in both rendering paths. XML
   attributes are escaped and CDATA terminators are split without losing text.
10. MCP acceptance tests use Cargo's prebuilt binary instead of starting a
    release build inside every test, and select available ports. Clipboard
    coverage checks availability on headless machines. Explicit logging flags
    override configuration defaults, quiet mode suppresses cycle warnings, and
    RMCP startup ignores context-generation configuration. Configuration
    docs/examples now use the implemented schema.

11. Final rendered budgets across all styles and combined roots, configurable
    response reservation, and a compact dependency-ranked repository map with
    cached signatures and source locations.
12. Tree-sitter 0.26 and compatible current grammars, including actual TSX
    parsing. C/C++, C#, Kotlin, Ruby, and PHP now have real analyzers/local
    resolvers. Parser deadlines are cooperative; async parsing runs on blocking
    workers. Rust 1.89 compatibility constrains the resolved language ABI crate.
13. JSON generation reports, opt-in native counting for OpenAI/Anthropic/Gemini,
    and explicit `off`/`redact`/`error` content scanning. `.contextignore` is now
    actually loaded alongside the legacy ignore filename; `.contextkeep`
    prioritizes visible files without bypassing ignores.
14. Bounded `repository_map` and `read_file` tools in both MCP servers. File pages
    respect repository/global ignores and root/symlink/hidden/binary/size limits,
    preserve complete lines, and expose a forward cursor. New progressive tools
    default to redaction; existing analyze tools include policy in cache keys.
15. A versioned thirteen-language corpus and pinned comparison adapters, process
    CPU/memory/output accounting, fixed navigation tasks, and separate Criterion
    cold/warm declaration-cache measurements. See [the harness](../benchmarks/README.md).

## Historical tokenizer measurement

This foundation experiment predates the change to a single combined-root budget;
it describes the `dfc37fa` implementation, not the current final-budget contract.
A local debug-build experiment created four directories, each containing twenty
Rust files with thirty copies of `pub fn example() -> usize { 42 }`. Five fresh
CLI processes generated context for all four directories using a 10,000-token
budget per directory. Both versions used `cl100k_base` to isolate vocabulary
reuse from encoding changes.

| Measurement | Before | After |
| --- | ---: | ---: |
| Median wall time | 2.382 s | 0.326 s |
| Observed range | 2.278–2.603 s | 0.312–0.330 s |
| Relative improvement | — | 7.3× |

This measures a tokenizer-initialization-heavy workload on this machine. It is
not a release-build throughput claim, a cross-tool comparison, or a prediction
for every repository. One-directory runs and parsing-heavy work need separate
measurements. Run `cargo bench --bench benchmarks` for the repository's Criterion
workloads; token cache hits and first-time encoding are measured separately.

## Release execution plan

Execution plan for the next release (each item requires focused regressions,
the full Rust/MCP checks, and review before pushing to `main`):

- [x] Commit the verified model/MCP/tokenizer/Java foundation.
- [x] Enforce final rendered budgets across formats and input roots; expose output reservation.
- [x] Add a compact, dependency-ranked repository map with definitions and signatures.
- [x] Implement and test C, C++, C#, Kotlin, Ruby, and PHP extraction and local resolution.
- [x] Upgrade compatible parsers together and cover module variants and malformed syntax.
- [x] Add reproducible cold/warm performance and fixed-task relevance benchmarks.
- [x] Add opt-in provider-native counting, without credentials in ordinary tests.
- [x] Add explicit secret handling and bounded map/file reads to both MCP servers.
- [ ] Validate the release binary, push tested commits to `main`, and inspect CI.

| Priority | Work | Completion criterion |
| --- | --- | --- |
| 1 | Enforce budgets against the final rendered output and combined directories. | Every style includes metadata/escaping overhead; oversized outputs produce a clear result without corrupting source. Test zero/tiny budgets, multiple roots, semantic metadata, and prompt/output reservation. Current file selection remains an estimate. |
| 2 | Add a compact repository-map output through `DigestFormatter`. | Reuse `ProjectAnalysis`, retain definitions/signatures and file links, and compare navigation recall and token savings on fixed tasks. Live answer quality needs a separate model evaluation. |
| 3 | Improve actual language coverage: C/C++, C#, Kotlin, Ruby, PHP. | Add compatible real parsers/resolvers and import/caller/type fixtures; never advertise empty stubs. Thirteen language families are registered. |
| 4 | Modernize parser infrastructure and module variants. | Upgrade tree-sitter/grammars together; test TSX-specific parsing, `.mjs`/`.cjs`/`.mts`/`.cts`, malformed syntax, and mixed source roots. |
| 5 | Add reproducible cold/warm performance and relevance evaluation. | Record CPU, memory, files, output size, selected encoding, parsing versus cache time, and task outcomes. Compare release builds against the four tools above on identical pinned corpora. |
| 6 | Optional provider-native counting and explicit output reservation. | Model-aware counts are opt-in, authenticated, tested without live credentials by default, and reported separately from local estimates. |
| 7 | Evaluate secret-content scanning and progressive MCP reads. | Define scanning policy/false positives and bounded map → file → expansion workflows; preserve ignore/path protections. |

The implemented scope is tracked above. Passing the current suites does not prove
the repository has no bugs, and language fixtures do not prove compiler-level
semantic equivalence. Provider invocation is verified with executable stubs;
account-specific live model access is outside those tests.
