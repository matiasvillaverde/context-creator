# Installation Guide

This guide covers various methods to install code-digest on different platforms.

## Prerequisites

- **Operating System**: Linux, macOS, or Windows
- **Rust** (if building from source): 1.70.0 or later
- **Memory**: Minimum 512MB RAM, 2GB+ recommended for large projects
- **Storage**: 50MB for installation, additional space for generated files

## Quick Install (Recommended)

### Using Cargo (All Platforms)

```bash
# Install from crates.io
cargo install code-digest

# Verify installation
code-digest --version
```

### Using Pre-built Binaries

Download the latest release for your platform:

```bash
# Linux x86_64
curl -L https://github.com/matiasvillaverde/code-digest/releases/latest/download/code-digest-linux-x86_64.tar.gz | tar xz
sudo mv code-digest /usr/local/bin/

# macOS (Intel)
curl -L https://github.com/matiasvillaverde/code-digest/releases/latest/download/code-digest-macos-x86_64.tar.gz | tar xz
sudo mv code-digest /usr/local/bin/

# macOS (Apple Silicon)
curl -L https://github.com/matiasvillaverde/code-digest/releases/latest/download/code-digest-macos-aarch64.tar.gz | tar xz
sudo mv code-digest /usr/local/bin/

# Windows
# Download code-digest-windows.zip from releases page
# Extract and add to PATH
```

## Package Managers

### Homebrew (macOS and Linux)

```bash
# Add tap
brew tap matiasvillaverde/tap

# Install
brew install code-digest

# Update
brew upgrade code-digest
```

### Arch Linux (AUR)

```bash
# Using yay
yay -S code-digest

# Using paru
paru -S code-digest

# Manual installation
git clone https://aur.archlinux.org/code-digest.git
cd code-digest
makepkg -si
```

### Debian/Ubuntu

```bash
# Add repository
curl -fsSL https://raw.githubusercontent.com/matiasvillaverde/code-digest/main/scripts/install-deb.sh | sudo bash

# Install
sudo apt update
sudo apt install code-digest

# Update
sudo apt upgrade code-digest
```

### RPM-based (RHEL, CentOS, Fedora)

```bash
# Add repository
sudo curl -o /etc/yum.repos.d/code-digest.repo https://raw.githubusercontent.com/matiasvillaverde/code-digest/main/scripts/code-digest.repo

# Install (DNF)
sudo dnf install code-digest

# Install (YUM)
sudo yum install code-digest

# Update
sudo dnf upgrade code-digest
```

### Windows Package Managers

#### Chocolatey

```powershell
# Install
choco install code-digest

# Update
choco upgrade code-digest
```

#### Scoop

```powershell
# Add bucket
scoop bucket add matiasvillaverde https://github.com/matiasvillaverde/scoop-bucket

# Install
scoop install code-digest

# Update
scoop update code-digest
```

#### WinGet

```powershell
# Install
winget install matiasvillaverde.code-digest

# Update
winget upgrade matiasvillaverde.code-digest
```

## Building from Source

### Clone and Build

```bash
# Clone repository
git clone https://github.com/matiasvillaverde/code-digest.git
cd code-digest

# Build release version
cargo build --release

# Install globally
cargo install --path .

# Or run directly
./target/release/code-digest --version
```

### Development Build

```bash
# Clone with development tools
git clone https://github.com/matiasvillaverde/code-digest.git
cd code-digest

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
code-digest --version

# Test basic functionality
cd /tmp
mkdir test-project
echo "fn main() { println!(\"Hello!\"); }" > test-project/main.rs
code-digest -d test-project

# Test with configuration
code-digest --help
```

Expected output:
```
code-digest 0.1.0
High-performance CLI tool to convert codebases to Markdown for LLM context

USAGE:
    code-digest [OPTIONS] [PROMPT]

ARGS:
    <PROMPT>    The prompt to send to the LLM...
```

## Environment Setup

### Shell Completion

#### Bash

```bash
# Generate completion script
code-digest --generate-completion bash > ~/.local/share/bash-completion/completions/code-digest

# Or add to .bashrc
echo 'eval "$(code-digest --generate-completion bash)"' >> ~/.bashrc
```

#### Zsh

```bash
# Generate completion script
code-digest --generate-completion zsh > ~/.local/share/zsh/site-functions/_code-digest

# Or add to .zshrc
echo 'eval "$(code-digest --generate-completion zsh)"' >> ~/.zshrc
```

#### Fish

```bash
# Generate completion script
code-digest --generate-completion fish > ~/.config/fish/completions/code-digest.fish
```

#### PowerShell

```powershell
# Generate completion script
code-digest --generate-completion powershell | Out-String | Invoke-Expression

# Add to profile
Add-Content $PROFILE 'code-digest --generate-completion powershell | Out-String | Invoke-Expression'
```

### Configuration Directory

Create default configuration directory:

```bash
# Linux/macOS
mkdir -p ~/.config/code-digest
mkdir -p ~/.local/share/code-digest

# Windows
mkdir %APPDATA%\code-digest
mkdir %LOCALAPPDATA%\code-digest
```

### Environment Variables

```bash
# Optional: Set default configuration file
export CODE_DIGEST_CONFIG="$HOME/.config/code-digest/config.toml"

# Optional: Set default cache directory
export CODE_DIGEST_CACHE_DIR="$HOME/.cache/code-digest"

# Optional: Set log level
export CODE_DIGEST_LOG_LEVEL="info"

# Optional: Set performance tuning
export CODE_DIGEST_PARALLEL_JOBS="8"
```

## Troubleshooting

### Common Issues

#### Permission Denied

```bash
# Linux/macOS: Fix permissions
sudo chown -R $(whoami) /usr/local/bin/code-digest
chmod +x /usr/local/bin/code-digest

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
- Join our [Community](https://github.com/matiasvillaverde/code-digest/discussions)

## Uninstallation

### Cargo Installation

```bash
cargo uninstall code-digest
```

### Package Managers

```bash
# Homebrew
brew uninstall code-digest

# APT
sudo apt remove code-digest

# DNF/YUM
sudo dnf remove code-digest

# Chocolatey
choco uninstall code-digest

# Scoop
scoop uninstall code-digest
```

### Manual Installation

```bash
# Remove binary
sudo rm /usr/local/bin/code-digest

# Remove configuration
rm -rf ~/.config/code-digest
rm -rf ~/.local/share/code-digest
```