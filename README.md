# code-digest 😋

[![CI](https://github.com/matiasvillaverde/code-digest/actions/workflows/ci.yml/badge.svg)](https://github.com/matiasvillaverde/code-digest/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=flat&logo=rust&logoColor=white)](https://www.rust-lang.org/)

Transform your entire codebase into a single, well-formatted Markdown file optimized for LLM context windows. Similar to [gitingest](https://gitingest.com/), but faster and with built-in Gemini CLI integration.

## Why code-digest?

**Leverage Gemini's massive context window** to understand your entire codebase at once. This tool gives AI assistants like Claude Code superpowers by enabling them to:

- 🏗️ Plan architectural changes with full visibility of your codebase
- 🔍 Answer complex questions about how different parts interact
- 📊 Analyze patterns and suggest improvements across your entire project
- 🚀 Make informed decisions when they need the big picture

Simply put: feed your entire repo to Gemini and have intelligent conversations about your code architecture.

## 🎯 Key Features

- **🚄 Blazing Fast**: Built in Rust with parallel processing
- **🤖 Gemini Integration**: Direct piping to [Gemini CLI](https://github.com/reugn/gemini-cli) for instant AI analysis
- **📊 Smart Token Management**: Accurate token counting using tiktoken
- **🎯 Intelligent Prioritization**: Automatically prioritizes important files when hitting token limits
- **🔍 Git-Aware**: Respects `.gitignore` and custom `.digestignore` patterns

## 📦 Installation

### Prerequisites

### Install code-digest

```bash
# Using Cargo
cargo install code-digest

# Or from source
git clone https://github.com/matiasvillaverde/code-digest.git
cd code-digest
cargo install --path .
```

Install Gemini CLI (optional):
```bash
npm install -g @google/gemini-cli
```

## 🚀 Quick Start

### Ask Questions About Your Codebase

```bash
# Analyze architecture
code-digest "What are the main architectural patterns used in this codebase?"

# Understand dependencies
code-digest "How does the authentication system interact with the database?"

# Find improvement opportunities
code-digest "What parts of this codebase could benefit from refactoring?"
```

### Generate Context Files

```bash
# Process current directory
code-digest

# Save to file for later use
code-digest -o context.md

# Process specific directory with token limit
code-digest -d /path/to/project --max-tokens 100000
```

## 📋 Configuration

### .digestignore

Exclude files from processing:

```gitignore
# Dependencies
node_modules/
target/
vendor/

# Build artifacts
dist/
build/
*.pyc

# Sensitive files
.env
secrets/
```

### .digestkeep

Prioritize important files:

```gitignore
# Core functionality
src/main.*
src/core/**/*.rs

# Important configs
Cargo.toml
package.json
```

### Configuration File (.code-digest.toml)

```toml
[defaults]
max_tokens = 150000
progress = true

[[priorities]]
pattern = "src/**/*.rs"
weight = 100

[[priorities]]
pattern = "tests/**/*.rs"
weight = 50
```

## 🔧 CLI Reference

```
code-digest [OPTIONS] [PROMPT]

Arguments:
  [PROMPT]  The prompt to send to Gemini

Options:
  -d, --directory <PATH>      Directory to process [default: .]
  -o, --output <FILE>         Output to file instead of stdout
      --max-tokens <N>        Maximum tokens for output
  -q, --quiet                 Suppress output except errors
  -v, --verbose               Enable verbose logging
  -c, --config <FILE>         Path to config file
      --progress              Show progress indicators
  -h, --help                  Print help
  -V, --version               Print version
```

## 🧪 Common Use Cases

### Architecture Review
```bash
code-digest "Create a high-level architecture diagram of this codebase"
```

### Security Audit
```bash
code-digest "Identify potential security vulnerabilities in this codebase"
```

### Documentation Generation
```bash
code-digest "Generate comprehensive API documentation for all public functions"
```

### Code Quality Analysis
```bash
code-digest "What code smells or anti-patterns exist in this project?"
```

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guidelines](CONTRIBUTING.md) for details.

### Development

```bash
# Run tests
make test

# Run all checks
make validate
```

## 🐛 Troubleshooting

**gemini not found**
- Ensure Gemini CLI is installed: `pip install gemini-cli`
- Verify it's in your PATH: `which gemini`

**Token count exceeded**
- Use `--max-tokens` to set a limit
- Configure file priorities in `.digestkeep`
- Exclude unnecessary files in `.digestignore`

## 🚧 Roadmap

- [ ] Support for more tokenizers (GPT-4, Claude, etc.)
- [ ] Custom output templates
- [ ] Integration with more LLM CLIs

---

<p align="center">Made with ❤️ and Rust</p>
