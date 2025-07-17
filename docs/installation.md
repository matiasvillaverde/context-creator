# Installation Guide

This guide covers various methods to install context-creator on different platforms.

## Prerequisites

- **Operating System**: Linux, macOS, or Windows
- **Rust** (if building from source): 1.70.0 or later
- **Memory**: Minimum 512MB RAM, 2GB+ recommended for large projects
- **Storage**: 50MB for installation, additional space for generated files

## Quick Install (Recommended)

### Using Cargo (All Platforms)

```bash
# Install from crates.io
cargo install context-creator

# Verify installation
context-creator --version
```

### Using Pre-built Binaries

Download the latest release for your platform:

```bash
# Linux x86_64
curl -L https://github.com/matiasvillaverde/context-creator/releases/latest/download/context-creator-linux-x86_64.tar.gz | tar xz
sudo mv context-creator /usr/local/bin/

# macOS (Intel)
curl -L https://github.com/matiasvillaverde/context-creator/releases/latest/download/context-creator-macos-x86_64.tar.gz | tar xz
sudo mv context-creator /usr/local/bin/

# macOS (Apple Silicon)
curl -L https://github.com/matiasvillaverde/context-creator/releases/latest/download/context-creator-macos-aarch64.tar.gz | tar xz
sudo mv context-creator /usr/local/bin/

# Windows
# Download context-creator-windows.zip from releases page
# Extract and add to PATH
```

## Package Managers

### Homebrew (macOS and Linux)

```bash
# Add tap
brew tap matiasvillaverde/tap

# Install
brew install context-creator

# Update
brew upgrade context-creator
```

### Arch Linux (AUR)

```bash
# Using yay
yay -S context-creator

# Using paru
paru -S context-creator

# Manual installation
git clone https://aur.archlinux.org/context-creator.git
cd context-creator
makepkg -si
```

### Debian/Ubuntu

```bash
# Add repository
curl -fsSL https://raw.githubusercontent.com/matiasvillaverde/context-creator/main/scripts/install-deb.sh | sudo bash

# Install
sudo apt update
sudo apt install context-creator

# Update
sudo apt upgrade context-creator
```

### RPM-based (RHEL, CentOS, Fedora)

```bash
# Add repository
sudo curl -o /etc/yum.repos.d/context-creator.repo https://raw.githubusercontent.com/matiasvillaverde/context-creator/main/scripts/context-creator.repo

# Install (DNF)
sudo dnf install context-creator

# Install (YUM)
sudo yum install context-creator

# Update
sudo dnf upgrade context-creator
```

### Windows Package Managers

#### Chocolatey

```powershell
# Install
choco install context-creator

# Update
choco upgrade context-creator
```

#### Scoop

```powershell
# Add bucket
scoop bucket add matiasvillaverde https://github.com/matiasvillaverde/scoop-bucket

# Install
scoop install context-creator

# Update
scoop update context-creator
```

#### WinGet

```powershell
# Install
winget install matiasvillaverde.context-creator

# Update
winget upgrade matiasvillaverde.context-creator
```

## Building from Source

### Clone and Build

```bash
# Clone repository
git clone https://github.com/matiasvillaverde/context-creator.git
cd context-creator

# Build release version
cargo build --release

# Install globally
cargo install --path .

# Or run directly
./target/release/context-creator --version
```

### Development Build

```bash
# Clone with development tools
git clone https://github.com/matiasvillaverde/context-creator.git
cd context-creator

# Install development dependencies
cargo install cargo-watch cargo-tarpaulin

# Build and test
make test
make bench

# Run in development mode
cargo run -- --help
```

### Custom Features

```bash
# Build with specific features
cargo build --release --features "custom-tokenizer,extended-formats"

# Build minimal version
cargo build --release --no-default-features
```

## LLM CLI Tools (Optional)

For direct LLM integration, install one or more LLM CLI tools:

### Gemini CLI

```bash
# Python/pip installation
pip install gemini

# Verify
gemini --version
```

### OpenAI Codex CLI

```bash
# Install from GitHub
npm install -g @openai/codex-cli

# Configure API key
export OPENAI_API_KEY="your-api-key"

# Verify
codex --version
```

### Anthropic Claude CLI

```bash
# Install from pip
pip install anthropic-cli

# Configure
export ANTHROPIC_API_KEY="your-api-key"

# Verify
claude --version
```

## Verification

After installation, verify everything works:

```bash
# Check version
context-creator --version

# Test basic functionality
cd /tmp
mkdir test-project
echo "fn main() { println!(\"Hello!\"); }" > test-project/main.rs
context-creator -d test-project

# Test with configuration
context-creator --help
```

Expected output:
```
context-creator 0.1.0
High-performance CLI tool to convert codebases to Markdown for LLM context

USAGE:
    context-creator [OPTIONS] [PROMPT]

ARGS:
    <PROMPT>    The prompt to send to the LLM...
```

## Environment Setup

### Shell Completion

#### Bash

```bash
# Generate completion script
context-creator --generate-completion bash > ~/.local/share/bash-completion/completions/context-creator

# Or add to .bashrc
echo 'eval "$(context-creator --generate-completion bash)"' >> ~/.bashrc
```

#### Zsh

```bash
# Generate completion script
context-creator --generate-completion zsh > ~/.local/share/zsh/site-functions/_context-creator

# Or add to .zshrc
echo 'eval "$(context-creator --generate-completion zsh)"' >> ~/.zshrc
```

#### Fish

```bash
# Generate completion script
context-creator --generate-completion fish > ~/.config/fish/completions/context-creator.fish
```

#### PowerShell

```powershell
# Generate completion script
context-creator --generate-completion powershell | Out-String | Invoke-Expression

# Add to profile
Add-Content $PROFILE 'context-creator --generate-completion powershell | Out-String | Invoke-Expression'
```

### Configuration Directory

Create default configuration directory:

```bash
# Linux/macOS
mkdir -p ~/.config/context-creator
mkdir -p ~/.local/share/context-creator

# Windows
mkdir %APPDATA%\context-creator
mkdir %LOCALAPPDATA%\context-creator
```

### Environment Variables

```bash
# Optional: Set default configuration file
export CODE_context_CONFIG="$HOME/.config/context-creator/config.toml"

# Optional: Set default cache directory
export CODE_context_CACHE_DIR="$HOME/.cache/context-creator"

# Optional: Set log level
export CODE_context_LOG_LEVEL="info"

# Optional: Set performance tuning
export CODE_context_PARALLEL_JOBS="8"
```

## Troubleshooting

### Common Issues

#### Permission Denied

```bash
# Linux/macOS: Fix permissions
sudo chown -R $(whoami) /usr/local/bin/context-creator
chmod +x /usr/local/bin/context-creator

# Windows: Run as Administrator or add to user PATH
```

#### Rust Not Found

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Update PATH
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc
```

#### Build Failures

```bash
# Update Rust
rustup update stable

# Clear cache
cargo clean

# Check dependencies
cargo check

# Specific error fixes
cargo update
```

#### Memory Issues

```bash
# Increase build parallelism
export CARGO_BUILD_JOBS=2

# Use less memory
export CARGO_PROFILE_RELEASE_LTO=thin
```

### Performance Tuning

```bash
# Enable all CPU cores
export RAYON_NUM_THREADS=$(nproc)

# Optimize for current CPU
export RUSTFLAGS="-C target-cpu=native"

# Use faster linker (Linux)
sudo apt install lld
export RUSTFLAGS="-C link-arg=-fuse-ld=lld"
```

## Next Steps

- Read the [Usage Guide](usage.md) for basic operations
- See [Configuration Reference](configuration.md) for advanced setup
- Check [Examples](examples.md) for common use cases
- Join our [Community](https://github.com/matiasvillaverde/context-creator/discussions)

## Uninstallation

### Cargo Installation

```bash
cargo uninstall context-creator
```

### Package Managers

```bash
# Homebrew
brew uninstall context-creator

# APT
sudo apt remove context-creator

# DNF/YUM
sudo dnf remove context-creator

# Chocolatey
choco uninstall context-creator

# Scoop
scoop uninstall context-creator
```

### Manual Installation

```bash
# Remove binary
sudo rm /usr/local/bin/context-creator

# Remove configuration
rm -rf ~/.config/context-creator
rm -rf ~/.local/share/context-creator
```