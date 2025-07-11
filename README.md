# code-digest 🚀

[![CI](https://github.com/matiasvillaverde/code-digest/actions/workflows/ci.yml/badge.svg)](https://github.com/matiasvillaverde/code-digest/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=flat&logo=rust&logoColor=white)](https://www.rust-lang.org/)

A high-performance CLI tool that transforms your codebase into a single, well-formatted Markdown file optimized for Large Language Model (LLM) context windows.

## 🎯 Features

- **🚄 Blazing Fast**: Built in Rust with parallel processing for maximum performance
- **🎯 Smart Prioritization**: Intelligently prioritizes files when token limits are reached
- **🔍 Git-Aware**: Respects `.gitignore` and custom `.digestignore` patterns
- **📊 Token Counting**: Accurate token counting using tiktoken for optimal LLM usage
- **🔗 Direct Integration**: Seamlessly pipes output to `gemini` or saves to file
- **⚙️ Highly Configurable**: Flexible configuration via CLI args or config files

## 📦 Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/matiasvillaverde/code-digest.git
cd code-digest

# Install with cargo
cargo install --path .
```

### Using Cargo

```bash
cargo install code-digest
```

## 🚀 Quick Start

### Basic Usage

Generate a markdown file from your codebase:

```bash
# Process current directory
code-digest

# Process specific directory
code-digest -d /path/to/your/project

# Save to file
code-digest -o output.md
```

### With Gemini Integration

Ask questions about your codebase directly:

```bash
code-digest "How does the authentication system work in this codebase?"
```

### Advanced Usage

```bash
# Limit token count
code-digest --max-tokens 100000 -o context.md

# Use custom configuration
code-digest -c my-config.toml

# Enable progress indicators
code-digest --progress -d ./src
```

## 📋 Configuration

### .digestignore

Create a `.digestignore` file in your project root to exclude files:

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

Prioritize important files with `.digestkeep`:

```gitignore
# Core functionality
src/main.*
src/core/**/*.rs
src/api/**

# Important configs
Cargo.toml
package.json
```

### Configuration File

Create `.code-digest.toml` for project-specific settings:

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

[[priorities]]
pattern = "docs/**/*.md"
weight = 30
```

## 🏗️ Architecture

```
code-digest/
├── src/
│   ├── main.rs       # CLI entry point
│   ├── lib.rs        # Public API
│   ├── cli.rs        # CLI argument parsing
│   ├── core/         # Core functionality
│   │   ├── walker.rs     # Directory traversal
│   │   ├── digest.rs     # Markdown generation
│   │   ├── token.rs      # Token counting
│   │   └── prioritizer.rs # File prioritization
│   └── utils/        # Utilities
│       ├── error.rs      # Error types
│       └── file_ext.rs   # File extension mapping
```

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guidelines](CONTRIBUTING.md) for details.

### Development Setup

```bash
# Clone the repo
git clone https://github.com/matiasvillaverde/code-digest.git
cd code-digest

# Run tests
make test

# Run lints
make lint

# Format code
make fmt

# Run all checks
make validate
```

## 📊 Performance

`code-digest` is designed for performance:

- Parallel file processing using Rayon
- Efficient memory usage with streaming
- Smart caching of token counts
- Optimized release builds with LTO

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

## 🧪 Examples

### Analyze a Rust Project

```bash
code-digest -d ~/my-rust-project "What are the main architectural patterns used?"
```

### Create Context for Code Review

```bash
code-digest --max-tokens 50000 -o review-context.md
```

### Process Multiple Repositories

```bash
for repo in repo1 repo2 repo3; do
  code-digest -d $repo -o $repo-context.md
done
```

## 📝 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- Built with [Rust](https://www.rust-lang.org/)
- Uses [tiktoken-rs](https://github.com/zurawiki/tiktoken-rs) for token counting
- Inspired by the need for better LLM context generation

## 🐛 Troubleshooting

### Common Issues

**gemini not found**
- Ensure `gemini` is installed and in your PATH
- Install with: `pip install gemini`

**Token count exceeded**
- Use `--max-tokens` to set a limit
- Configure file priorities in `.digestkeep`
- Exclude unnecessary files in `.digestignore`

**Performance issues**
- Use `--verbose` to identify bottlenecks
- Consider using `.digestignore` to skip large directories
- Ensure you're using a release build

## 🚧 Roadmap

- [ ] Support for more tokenizers (GPT-4, Claude, etc.)
- [ ] Custom output templates
- [ ] Integration with more LLM CLIs
- [ ] Web UI for configuration
- [ ] Plugin system for custom processors

---

<p align="center">Made with ❤️ and Rust</p>