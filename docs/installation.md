# Installation Guide

This guide covers the actual installation methods supported by context-creator.

## Prerequisites

- **For npm package**: Node.js >= 16.0.0
- **For building from source**: Rust >= 1.70.0

## Quick Install

### NPM Package (Recommended for MCP)

```bash
# Install globally
npm install -g context-creator-mcp@latest

# Or use with npx (no installation required)
npx -y context-creator-mcp@latest
```

### Cargo (Building from Source)

```bash
# Install from crates.io
cargo install context-creator

# Verify installation
context-creator --version
```

## MCP Client Setup

context-creator can be used as an MCP server with various AI assistants. Here's how to configure it:

### Basic Configuration

All MCP clients use the same basic configuration pattern:

```json
{
  "mcpServers": {
    "context-creator": {
      "command": "npx",
      "args": ["-y", "context-creator-mcp@latest"]
    }
  }
}
```

### Platform-Specific Setup

<details>
<summary>Claude Desktop</summary>

1. Edit configuration file:
   - macOS: `~/Library/Application Support/Claude/claude_desktop_config.json`
   - Windows: `%APPDATA%\Claude\claude_desktop_config.json`

2. Add the configuration above

3. Restart Claude Desktop
</details>

<details>
<summary>Claude Code</summary>

Option 1 - Project configuration:
```json
// .mcp.json in project root
{
  "mcpServers": {
    "context-creator": {
      "command": "npx",
      "args": ["-y", "context-creator-mcp@latest"]
    }
  }
}
```

Option 2 - Global configuration:
```bash
claude mcp add context-creator -- npx -y context-creator-mcp@latest
```
</details>

<details>
<summary>Cursor</summary>

1. Open Settings → Extensions → MCP
2. Add the basic configuration
</details>

<details>
<summary>Other MCP Clients</summary>

For other MCP clients (VS Code, Windsurf, Zed, Cline, etc.), refer to your client's MCP documentation. The configuration pattern is the same - use `npx` with `context-creator-mcp@latest`.
</details>

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
# Install development dependencies
cargo install cargo-watch cargo-tarpaulin

# Build and test
make test

# Run in development mode
cargo run -- --help
```

## Verification

After installation, verify everything works:

### NPM Package
```bash
# Test MCP server mode
npx -y context-creator-mcp@latest --version

# Should automatically add --rmcp flag for MCP mode
```

### Cargo Installation
```bash
# Check version
context-creator --version

# Test basic functionality
context-creator --help
```

## Environment Variables

```bash
# Optional: Set log level
export RUST_LOG=info

# Optional: Set performance tuning
export RAYON_NUM_THREADS=8
```

## Troubleshooting

### NPM Issues

- **Node.js version**: Requires Node.js >= 16.0.0
- **Permission denied**: May need sudo on Linux/macOS for global install
- **Binary not found**: The npm package includes platform-specific binaries

### Cargo Issues

- **Rust not installed**: Install from https://rustup.rs
- **Build failures**: Update Rust with `rustup update stable`
- **OpenSSL errors**: Install OpenSSL development headers

### MCP Connection Issues

- **Server not starting**: Check Node.js is installed
- **Configuration not working**: Ensure using exact JSON format
- **Already configured**: Remove old configurations first

## Supported Platforms

The npm package includes pre-built binaries for:
- macOS (x64, arm64)
- Linux (x64, arm64)
- Windows (x64)

For other platforms, build from source using Cargo.