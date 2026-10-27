# Installation Guide

This guide covers the actual installation methods supported by context-creator.

## Prerequisites

- **For npm package**: Node.js >= 16.0.0
- **For building the current source**: Rust >= 1.89.0, Git, and a C/C++ build toolchain

## Quick Install

### Native CLI and crate examples on main

The README's task-context and crate examples target the current `main` branch.
Install it directly from Git when you do not need a local checkout:

```bash
cargo install --git https://github.com/matiasvillaverde/context-creator --branch main --locked
context-creator --version
```

To keep a checkout and try the bundled examples, use the
[source build instructions](#building-from-source) below.

### NPM package for MCP

```bash
# Install globally
npm install -g context-creator-mcp@latest

# Or use with npx (no installation required)
npx -y context-creator-mcp@latest
```

The npm launcher starts MCP mode automatically. Use the native `context-creator`
binary for CLI context generation. Published package versions can have fewer
features than `main`.

### Published Cargo release

```bash
# Install from crates.io
cargo install context-creator --locked

# Verify installation
context-creator --version
```

### Prebuilt release binaries

Download the asset for your operating system and architecture from
[GitHub Releases](https://github.com/matiasvillaverde/context-creator/releases).
Place the executable on `PATH`; on Unix systems, make it executable first.
Released binaries follow their release's feature set. Build `main` to use the
current README's task-context and crate APIs.

## MCP Client Setup

context-creator can be used as an MCP server with various AI assistants. Here's how to configure it:

### Basic Configuration

For clients using the `mcpServers` format, the published npm launcher can be
configured as follows:

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

To use the native binary from the current source, set `command` to
`context-creator` and `args` to `["--rmcp"]`. Use a full binary path when the
client cannot find it on `PATH`. Client configuration formats differ; the native
stdio server command is always `context-creator --rmcp`.

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

For other MCP clients, use your client's documented stdio configuration format.
Run either `context-creator --rmcp` for the native binary or
`npx -y context-creator-mcp@latest` for the published launcher.
</details>

## Building from Source

### Clone and Build

```bash
# Clone repository
git clone https://github.com/matiasvillaverde/context-creator.git
cd context-creator

# Build release version
cargo build --release --locked

# Install globally
cargo install --path . --locked

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
