# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

context-creator is a high-performance Rust CLI tool that converts entire codebases into LLM-optimized Markdown for consumption by models like Gemini. It processes git repositories by intelligently filtering, prioritizing, and formatting code into a single cohesive document.

## Key Commands

### Development Commands
```bash
# Build and run validations (format check + lint)
make build

# Run tests with validation
make test

# Run in development mode with example
make dev

# Install locally
make install

# Generate documentation
make doc
```

### Quality & CI Commands
```bash
# Format code
make fmt

# Check formatting (CI)
make fmt-check

# Lint with clippy
make lint

# Run all validation checks
make validate

# Generate test coverage
make coverage
```

### Testing Commands
```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run tests with specific features
cargo test --all-targets

# Run benchmarks
cargo bench
```

## Project Architecture

### Core Components

1. **CLI Layer (`src/cli.rs`)**
   - Command-line argument parsing using clap
   - Configuration validation and loading
   - Supports multiple input sources: directories, glob patterns, GitHub repos

2. **Core Processing (`src/core/`)**
   - `walker.rs`: Directory traversal with .gitignore support and custom filtering
   - `context_builder.rs`: Markdown generation with token management
   - `prioritizer.rs`: File prioritization based on importance and token limits
   - `semantic/`: Language-aware semantic analysis for imports and dependencies
   - `cache.rs`: File caching for performance optimization

3. **Configuration System (`src/config.rs`)**
   - TOML-based configuration with `.context-creator.toml`
   - Support for custom priorities, token limits, and ignore patterns
   - Hierarchical configuration loading (CLI > config file > defaults)

4. **Semantic Analysis (`src/core/semantic/`)**
   - Multi-language AST parsing using tree-sitter
   - Import tracing and dependency resolution
   - Language-specific analyzers in `languages/` directory
   - Supports 20+ programming languages

### Key Data Flow

1. **Input Processing**: CLI args → Config validation → Directory resolution
2. **File Discovery**: Walker scans directories → Applies ignore patterns → Filters by include patterns
3. **Semantic Analysis**: Optional import tracing → Dependency resolution → Enhanced file relationships
4. **Prioritization**: File importance scoring → Token budget allocation → Selection for output
5. **Output Generation**: Markdown formatting → Token counting → Final document assembly

### Configuration Files

- `.contextignore`: Exclude files/directories (gitignore syntax)
- `.contextkeep`: Prioritize important files
- `.context-creator.toml`: Advanced configuration (priorities, token limits, defaults)

### Token Management

The tool implements sophisticated token budget management:
- Automatic token counting using tiktoken-rs
- Prompt token reservation for LLM interactions
- Intelligent file truncation and selection
- Per-LLM token limit configuration

### Semantic Features

- **Import Tracing**: Follow import chains across files
- **Caller Analysis**: Find functions that call specific modules
- **Type Dependencies**: Include type definitions and interfaces
- **Multi-language Support**: Unified analysis across different languages

## Important Implementation Details

### Error Handling
- Uses `anyhow` for error propagation
- Custom error types in `src/utils/error.rs`
- Comprehensive validation at multiple levels

### Performance Optimizations
- Parallel processing with rayon
- File caching to avoid re-reading
- Tree-sitter parser pooling
- LRU caching for AST parsing

### Testing Strategy
- Unit tests for core functionality
- Integration tests for CLI combinations
- Performance benchmarks
- Semantic analysis stress tests
- Security vulnerability tests

## Development Notes

### Adding New Languages
Language support is added in `src/core/semantic/languages/`:
1. Implement language-specific analyzer
2. Add tree-sitter grammar dependency
3. Update language registry in `mod.rs`

### Configuration Precedence
1. Explicit CLI arguments (highest priority)
2. Config file token limits (for prompts)
3. Config file defaults
4. Hard-coded defaults (lowest priority)

### File Prioritization
Base file type priorities can be customized via config file using glob patterns. Uses "first-match-wins" evaluation order.