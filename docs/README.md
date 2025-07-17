# Code context Documentation

Welcome to the context-creator documentation! This high-performance CLI tool converts codebases to Markdown format optimized for Large Language Model (LLM) consumption.

## Quick Links

- [Installation Guide](installation.md)
- [Usage Guide](usage.md)
- [Configuration Reference](configuration.md)
- [API Reference](api.md)
- [Examples](examples.md)
- [Troubleshooting](troubleshooting.md)
- [Contributing](../CONTRIBUTING.md)

## What is Code context?

Code context is a Rust-based CLI tool that:

- **Converts** entire codebases to structured Markdown
- **Prioritizes** files based on importance and token limits
- **Optimizes** output for LLM context windows
- **Supports** 20+ programming languages
- **Integrates** with LLM CLI tools (gemini, codex)
- **Processes** projects in parallel for maximum performance

## Key Features

### 🚀 **High Performance**
- Parallel file processing with Rayon
- Intelligent token counting with tiktoken-rs
- Memory-efficient streaming for large projects
- Benchmark: 2.4K files/sec end-to-end processing

### 🎯 **Smart Prioritization**
- File importance scoring based on type and location
- Token limit enforcement with optimal file selection
- Configurable priority weights and patterns
- Automatic structure overhead calculation

### ⚙️ **Flexible Configuration**
- TOML configuration files with inheritance
- CLI argument overrides
- .contextignore support (like .gitignore)
- Environment variable integration

### 🔧 **LLM Integration**
- Direct integration with gemini and codex
- Optimized token usage for context windows
- Structured output with table of contents
- File tree visualization

### 🧪 **Production Ready**
- Comprehensive test suite (77 tests)
- CI/CD with GitHub Actions
- Release automation
- Performance benchmarks

## Quick Start

```bash
# Install
cargo install context-creator

# Basic usage
context-creator -d /path/to/project -o project.md

# With token limits
context-creator -d /path/to/project --max-tokens 50000 -o project.md

# Direct LLM integration
context-creator -d /path/to/project "Explain the architecture of this codebase"

# With configuration
context-creator -d /path/to/project -c config.toml -o project.md
```

## Use Cases

### 📋 **Code Review & Analysis**
- Generate comprehensive project overviews
- Create documentation for legacy codebases
- Prepare code for AI-assisted reviews
- Export codebases for external analysis

### 🤖 **LLM Context Preparation**
- Convert projects for ChatGPT/GPT-4 analysis
- Prepare context for code generation tasks
- Create training data for custom models
- Generate structured prompts for AI tools

### 📚 **Documentation & Knowledge Transfer**
- Create onboarding materials for new developers
- Generate technical documentation automatically
- Export codebases for architecture discussions
- Prepare materials for technical interviews

### 🔍 **Project Understanding**
- Quickly understand unfamiliar codebases
- Generate project summaries and insights
- Identify key components and dependencies
- Analyze code patterns and structures

## Architecture Overview

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   CLI Parser    │───▶│  Configuration   │───▶│ Directory Walker│
│   (clap)        │    │  (TOML + Args)   │    │   (walkdir)     │
└─────────────────┘    └──────────────────┘    └─────────────────┘
                                                         │
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│  LLM Integration│◀───│   Markdown Gen   │◀───│ File Prioritizer│
│ (gemini/codex)  │    │   (templates)    │    │ (tiktoken-rs)   │
└─────────────────┘    └──────────────────┘    └─────────────────┘
```

## Performance Characteristics

| Metric | Performance |
|--------|-------------|
| Directory Walking | 160-276K files/sec |
| Token Counting | 680MB/s - 1.6GB/s |
| File Prioritization | 10K files/sec |
| Markdown Generation | 80K files/sec |
| End-to-End | 2.4K files/sec |
| Parallel Speedup | ~40% improvement |

## Supported Languages

| Language | Extension | Priority | Notes |
|----------|-----------|----------|--------|
| Rust | `.rs` | High | Native optimization |
| Python | `.py` | High | Complete support |
| JavaScript | `.js` | High | ES6+ features |
| TypeScript | `.ts`, `.tsx` | High | Full type support |
| Go | `.go` | Medium | Standard library aware |
| Java | `.java` | Medium | Package structure |
| C++ | `.cpp`, `.hpp` | Medium | Header handling |
| C | `.c`, `.h` | Medium | Include processing |
| C# | `.cs` | Medium | Namespace support |
| Ruby | `.rb` | Medium | Gem structure |
| PHP | `.php` | Medium | Framework aware |
| Swift | `.swift` | Medium | iOS/macOS focus |
| Kotlin | `.kt` | Medium | Android support |
| Scala | `.scala` | Medium | JVM integration |
| Haskell | `.hs` | Medium | Functional focus |
| Markdown | `.md` | Low | Documentation |
| JSON | `.json` | Low | Configuration |
| YAML | `.yml`, `.yaml` | Low | Configuration |
| TOML | `.toml` | Low | Configuration |
| XML | `.xml` | Low | Data format |
| HTML | `.html` | Low | Web content |
| CSS | `.css` | Low | Styling |

## Project Status

- ✅ **Core Features**: Complete
- ✅ **Testing**: 77 tests, 100% critical path coverage
- ✅ **Performance**: Optimized and benchmarked
- ✅ **Documentation**: Comprehensive guides
- ✅ **CI/CD**: GitHub Actions pipeline
- 🚧 **Examples**: In progress
- 🚧 **Release**: Preparing v1.0.0

## Community & Support

- **Issues**: [GitHub Issues](https://github.com/matiasvillaverde/context-creator/issues)
- **Discussions**: [GitHub Discussions](https://github.com/matiasvillaverde/context-creator/discussions)
- **Contributing**: See [CONTRIBUTING.md](../CONTRIBUTING.md)
- **License**: MIT License

## What's Next?

- 📦 Package distribution (Homebrew, apt, etc.)
- 🔌 Plugin system for custom processors
- 🎨 Template system for custom output formats
- 🔄 Watch mode for continuous processing
- 🌐 Web interface for team collaboration
- 📊 Analytics and usage insights