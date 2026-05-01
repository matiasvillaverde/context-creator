# AGENTS.md

Repository-level guidance for coding agents working on `context-creator`.

## Scope

- This file applies to the entire repository unless a more specific nested `AGENTS.md` is added later.
- Prefer current source code over older docs when they disagree. Some docs still show older API names, while the Rust modules are authoritative.
- Keep changes narrowly scoped. This project has broad CLI, semantic-analysis, MCP, and security coverage, so unrelated rewrites are expensive to validate.

## Project Overview

`context-creator` is a Rust CLI and library for turning codebases into LLM-friendly context. It walks files, applies ignore/include rules, optionally performs semantic dependency analysis with tree-sitter, prioritizes files under token budgets, formats output, and can run as an MCP server.

Main user-facing surfaces:

- CLI context generation from local paths or GitHub repositories.
- Subcommands: `search`, `diff`, `telemetry`, and `examples`.
- Output styles: markdown, XML, plain text, and paths-only.
- MCP server implementations: legacy jsonrpsee and RMCP.
- LLM tool integration for Gemini, Codex, Claude, and Ollama.

## Toolchain

- Rust edition: 2021.
- MSRV: 1.74.0, as configured in `clippy.toml`.
- CI uses stable Rust with `rustfmt` and `clippy`.
- Optional tools: `make`, `cargo-audit`, `cargo-tarpaulin`, `cargo-watch`, Node.js for the generated npm package workflow.

## High-Value Commands

Run targeted checks while developing, then broaden before finishing risky changes.

```bash
cargo check --all-targets
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --test lib
```

Make targets wrap the common flows:

```bash
make check
make fmt
make fmt-check
make lint
make test
make test-fast
make test-acceptance
make validate
```

Useful focused test forms:

```bash
cargo test --test lib cli_test::
cargo test --test lib semantic_include_types_test::
cargo test --test lib acceptance:: --no-fail-fast
cargo test --test rmcp_basic_test
cargo test --test mcp_server_test
```

Run the CLI locally:

```bash
cargo run -- --help
cargo run -- examples/sample-rust-project
cargo run -- search "AuthenticationService" src/
cargo run -- diff HEAD~1 HEAD
cargo run -- telemetry -t examples/telemetry/traces.json
cargo run -- --rmcp --rmcp-transport stdio
cargo run -- --rmcp --rmcp-transport http --mcp-port 9090
```

Notes:

- `make test` runs format, lint, `cargo build`, and the consolidated `tests/lib.rs` integration target.
- Use `cargo test` when changing standalone integration tests, MCP tests, or shared behavior not registered in `tests/lib.rs`.
- `cargo audit` may require installing `cargo-audit`; `.cargo/audit.toml` intentionally ignores `RUSTSEC-2025-0009` for the documented jsonrpsee/rustls/ring transitive dependency.

## Source Map

- `src/main.rs`: binary entry point, config loading, logging setup, stdin prompt handling, MCP server startup.
- `src/lib.rs`: library entry point and main processing pipeline.
- `src/cli.rs`: clap definitions, subcommands, validation, LLM tool behavior, token-limit precedence.
- `src/config.rs`: TOML config loading and application to CLI config.
- `src/commands/`: implementations for `search`, `diff`, and `telemetry`.
- `src/core/walker.rs`: path walking, ignore/include pattern handling, file metadata, priority calculation, binary filtering.
- `src/core/context_builder.rs`: context options and output generation.
- `src/core/prioritizer.rs`: token-aware priority selection.
- `src/core/token.rs`: token counting.
- `src/core/project_analyzer.rs`: single-pass project analysis reused by semantic features.
- `src/core/file_expander.rs`: semantic expansion for imports, callers, and types.
- `src/core/search.rs`: text search support.
- `src/core/telemetry/`: OpenTelemetry parsing, correlation, and enrichment.
- `src/core/semantic/`: tree-sitter semantic analysis, graph traversal, language analyzers, parser/cache infrastructure.
- `src/formatters/`: markdown, XML, plain text, and paths renderers behind `DigestFormatter`.
- `src/mcp_server/`: MCP request/response types, jsonrpsee handlers, RMCP server, and MCP cache.
- `src/remote.rs`: remote repository fetch support.
- `src/utils/`: file extension mapping, git utilities, shared error types.
- `tests/lib.rs`: consolidated integration test runner that includes most tests under `tests/modules/`.
- `tests/modules/acceptance/` and `tests/acceptance/`: acceptance coverage and telemetry enrichment fixtures.
- `examples/`: sample projects, configs, and telemetry data for manual testing.
- `.github/workflows/`: CI, release, npm publish, and crates.io publish workflows.

## Architecture Rules

- Preserve the CLI flow: parse `Config`, load config, validate, dispatch subcommands, then run the normal directory-processing pipeline.
- Use `Config` helper methods instead of reading raw fields when behavior depends on precedence or normalization:
  - `get_directories()`
  - `get_prompt()`
  - `get_include_patterns()`
  - `get_ignore_patterns()`
  - `get_effective_max_tokens()`
  - `get_effective_context_tokens()`
- Keep validation in `Config::validate()` for user-facing CLI combinations. Existing flexible combinations are intentional; do not re-add broad mutual exclusions without tests.
- Use `ContextOptions::from_config()` and `WalkOptions::from_config()` so token limits, binary filtering, ignore patterns, and priority rules stay consistent.
- Semantic features should use `ProjectAnalysis::analyze_project()` and `expand_file_list_with_context()` when full-project context is needed. Avoid adding extra full tree walks on hot paths.
- Keep deterministic ordering when selecting or rendering files: priority descending, then path ascending is the established pattern.
- For MCP handlers, keep blocking filesystem, git, AST, and LLM work inside `tokio::task::spawn_blocking`.
- Do not block the Tokio runtime directly in async MCP methods.
- Keep both MCP implementations in sync when changing shared request/response behavior.
- For output styles, add behavior through `DigestFormatter` and `create_formatter()` rather than branching throughout the pipeline.

## Code Style

- Use idiomatic Rust 2021 and keep `rustfmt` clean with `max_width = 100`.
- `cargo clippy --all-targets --all-features -- -D warnings` must pass.
- Prefer small, focused functions and modules that match the existing separation of CLI, core processing, formatting, MCP, and utilities.
- Use `anyhow::Result` at orchestration boundaries and `ContextCreatorError` for user-facing validation, path, security, token, clipboard, and tool errors.
- Avoid `unwrap()` and `expect()` in production code. They are allowed in tests by `clippy.toml`.
- Use `tracing` for diagnostics. Respect `quiet`, `progress`, `verbose`, `--log-format`, and `RUST_LOG`.
- Do not add noisy stdout output in library/core code. CLI stdout is part of the product output.
- Keep comments useful and sparse; explain non-obvious algorithms, security checks, and concurrency decisions.
- Avoid new dependencies unless they remove real complexity. Prefer existing crates already in `Cargo.toml`.

## Testing Guidance

- Add or update tests for behavior changes, especially CLI validation, ignore/include semantics, token limits, semantic expansion, MCP request handling, and security-sensitive paths.
- Put most integration coverage in `tests/modules/...` and register it in `tests/lib.rs`.
- Use standalone `tests/*.rs` files when the test target needs separate setup or is already organized that way, especially MCP tests.
- Use module-level unit tests for local parser/analyzer behavior.
- Use acceptance tests for realistic end-to-end behavior and fixture-based semantic/telemetry scenarios.
- Leave `*.disabled` stress/performance tests disabled unless the task explicitly asks to revive them.
- When adding tests that shell out to the binary, follow existing `assert_cmd` patterns.
- For semantic tests, prefer small fixture code that directly exercises the graph, resolver, or analyzer edge case.

## Semantic Analysis Guidance

- Language analyzers live in `src/core/semantic/languages/`.
- `get_analyzer_for_file()` controls which files get semantic extraction.
- `get_resolver_for_file()` currently provides module resolution for Rust, Python, JavaScript, and TypeScript.
- Adding or expanding language support usually requires:
  - Updating `src/utils/file_ext.rs` if file type or fence-language behavior changes.
  - Updating `src/core/semantic/languages/mod.rs` and the language file.
  - Adding tree-sitter dependencies in `Cargo.toml` when real parsing is needed.
  - Extending analyzer and resolver tests.
  - Adding integration coverage for `--trace-imports`, `--include-callers`, or `--include-types` if those features are affected.
- Keep path resolution constrained through the existing path validation helpers. Do not let import resolution escape the detected project root.
- Preserve cycle handling and depth limits. `semantic_depth` exists to bound traversal.

## Security And Safety

- Treat file paths, glob patterns, git refs, repository URLs, and MCP parameters as untrusted input.
- Preserve protections against path traversal, dangerous git-ref characters, overly long patterns, symlink traversal, and binary/large-file processing.
- Do not follow symlinks by default or include hidden files by default unless the change is explicit and tested.
- Be careful when adding logs: prompts, file contents, env files, and repository paths may contain sensitive data.
- `.contextignore` excludes secrets and build artifacts for context generation. Do not remove those defaults casually.
- Keep command execution through `std::process::Command` with argument arrays, not shell interpolation.
- If changing `cargo audit` behavior, update `.cargo/audit.toml` only with a clear reason.

## Configuration And Context Files

- Supported context files include `.contextignore`, `.contextkeep`, and `.context-creator.toml`.
- Custom priority rules are first-match-wins and are compiled in `WalkOptions::from_config()`.
- `ConfigFile::load_default()` currently looks for `.context-creator.toml`, `.contextrc.toml`, and a home config. Keep docs and tests aligned if this changes.
- Token precedence is subtle. Preserve the order implemented in `Config::get_effective_max_tokens()`:
  1. explicit CLI `--max-tokens`
  2. tool-specific config token limits when a prompt is present
  3. config default max tokens
  4. tool default when a prompt is present
  5. no automatic limit for non-prompt usage unless configured

## MCP Guidance

- Legacy server: jsonrpsee code in `src/mcp_server/mod.rs` and `src/mcp_server/handlers.rs`.
- Current RMCP server: `src/mcp_server/rmcp_server.rs` with helpers in `rmcp_handlers.rs`.
- The binary supports `--mcp` for legacy mode and `--rmcp` for RMCP mode.
- RMCP transports are `stdio` and `http`.
- Keep cache keys in `src/mcp_server/cache.rs` aligned with request fields.
- Validate paths before processing. Local path validation differs between legacy and RMCP code; update both intentionally if behavior changes.
- Use `test_rmcp_server.sh` for manual RMCP checks when relevant.

## Git, Release, And Packaging

- Conventional commit prefixes are documented in `CONTRIBUTING.md`: `feat:`, `fix:`, `docs:`, `test:`, `chore:`, `perf:`, `refactor:`.
- Git hooks are optional. Enable them with `git config core.hooksPath .githooks`.
- Version metadata lives in `Cargo.toml`.
- Release and publish workflows are tag-driven on `v*`.
- The npm package structure is generated in `.github/workflows/npm-publish.yml`; there is no root `package.json` to edit for normal Rust changes.
- Do not commit build outputs from `target/`, coverage output, generated docs, or temporary analysis files.

## Documentation

- Update `README.md` and `docs/` for user-facing CLI, configuration, MCP, or behavior changes.
- Update examples when changing flags, config schema, output styles, or semantic behavior.
- Prefer concise docs with executable commands.
- If docs disagree with source, fix docs in the same change when practical.

## Before Finishing

- Run the narrowest relevant tests first.
- For shared CLI/core changes, run at least `cargo fmt -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --test lib`.
- For MCP changes, also run the relevant standalone MCP integration tests.
- For docs-only changes, no Rust test is usually required, but verify links, paths, and command names against the repository.
