# context-creator

[![CI](https://github.com/matiasvillaverde/context-creator/actions/workflows/ci.yml/badge.svg)](https://github.com/matiasvillaverde/context-creator/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=flat&logo=rust&logoColor=white)](https://www.rust-lang.org/)

> Transform your codebase into intelligent LLM context with MCP (Model Context Protocol) integration

`context-creator` is a high-performance MCP server that analyzes codebases and answers questions about them. Built in Rust, it creates dependency graphs and semantic analysis to provide relevant, focused contexts for AI assistants.

## 🚀 Key Benefits

- **MCP Server Integration** - Works seamlessly with Claude Desktop, Cursor, and other MCP clients
- **Intelligent Analysis** - Builds dependency graphs and traces imports across your codebase
- **Blazing Fast** - Rust-powered parallel processing handles massive codebases in seconds
- **Multi-Language Support** - Semantic analysis for Python, TypeScript, JavaScript, and Rust

## 🛠️ Installation

### Requirements
- Node.js >= v18.0.0
- Cursor, Windsurf, Claude Desktop or another MCP Client

### Installing via Smithery

To install context-creator for Claude Desktop automatically via [Smithery](https://smithery.ai/protocol/context-creator):

```bash
npx -y @smithery/cli install context-creator --client claude
```

<details>
<summary>▶️ Install in Cursor</summary>

1. Open the **Cursor IDE**
2. Click **Settings** → **Extensions** → **MCP**
3. Add the following configuration:

```json
{
  "mcpServers": {
    "context-creator": {
      "command": "npx",
      "args": ["-y", "@matiasvillaverde/context-creator-mcp"]
    }
  }
}
```
</details>

<details>
<summary>▶️ Install in Windsurf</summary>

1. Open **Windsurf Settings** (⌘/Ctrl + ,)
2. Navigate to **MCP Servers**
3. Click **+ Add Server** and enter:

```json
{
  "id": "context-creator",
  "name": "Context Creator",
  "command": "npx",
  "args": ["-y", "@matiasvillaverde/context-creator-mcp"]
}
```
</details>

<details>
<summary>▶️ Install in Trae</summary>

1. Open Trae's MCP configuration panel
2. Add new server with:

```json
{
  "servers": {
    "context-creator": {
      "command": "npx",
      "args": ["-y", "@matiasvillaverde/context-creator-mcp"]
    }
  }
}
```
</details>

<details>
<summary>▶️ Install in VS Code</summary>

1. Install the MCP extension for VS Code
2. Open Command Palette (⌘/Ctrl + Shift + P)
3. Run "MCP: Add Server" and configure:

```json
{
  "context-creator": {
    "command": "npx",
    "args": ["-y", "@matiasvillaverde/context-creator-mcp"]
  }
}
```
</details>

<details>
<summary>▶️ Install in Visual Studio 2022</summary>

1. Open Visual Studio 2022
2. Navigate to Tools → Options → MCP Settings
3. Add server configuration:

```json
{
  "servers": [
    {
      "name": "context-creator",
      "command": "npx",
      "args": ["-y", "@matiasvillaverde/context-creator-mcp"]
    }
  ]
}
```
</details>

<details>
<summary>▶️ Install in Zed</summary>

1. Open Zed settings (`~/.config/zed/settings.json`)
2. Add to the MCP section:

```json
{
  "mcp": {
    "servers": {
      "context-creator": {
        "command": "npx",
        "args": ["-y", "@matiasvillaverde/context-creator-mcp"]
      }
    }
  }
}
```
</details>

<details>
<summary>▶️ Install in Gemini CLI</summary>

```bash
# Add to your Gemini CLI configuration
gemini mcp add context-creator "npx -y @matiasvillaverde/context-creator-mcp"

# Verify installation
gemini mcp list
```
</details>

<details>
<summary>▶️ Install in Claude Code</summary>

1. Create `.mcp.json` in your project root:

```json
{
  "mcpServers": {
    "context-creator": {
      "command": "npx",
      "args": ["-y", "@matiasvillaverde/context-creator-mcp"]
    }
  }
}
```

2. Or add globally:

```bash
claude mcp add context-creator "npx -y @matiasvillaverde/context-creator-mcp"
```
</details>

<details>
<summary>▶️ Install in Claude Desktop</summary>

1. Open Claude Desktop settings
2. Navigate to MCP Servers
3. Add configuration:

```json
{
  "mcpServers": {
    "context-creator": {
      "command": "npx",
      "args": ["-y", "@matiasvillaverde/context-creator-mcp"]
    }
  }
}
```
</details>

<details>
<summary>▶️ Install in Cline</summary>

1. Open Cline configuration
2. Add to MCP servers:

```json
{
  "mcp_servers": [
    {
      "name": "context-creator",
      "command": "npx",
      "args": ["-y", "@matiasvillaverde/context-creator-mcp"]
    }
  ]
}
```
</details>

<details>
<summary>▶️ Install in BoltAI</summary>

1. Open BoltAI preferences
2. Go to MCP Servers tab
3. Click "Add Server" and configure:

```json
{
  "name": "context-creator",
  "command": "npx",
  "args": ["-y", "@matiasvillaverde/context-creator-mcp"]
}
```
</details>

<details>
<summary>▶️ Using Docker</summary>

```bash
# Run with Docker
docker run -v $(pwd):/workspace matiasvillaverde/context-creator-mcp

# Or add to docker-compose.yml
services:
  context-creator:
    image: matiasvillaverde/context-creator-mcp
    volumes:
      - .:/workspace
```
</details>

<details>
<summary>▶️ Install in Windows</summary>

1. Open PowerShell as Administrator
2. Install globally:

```powershell
npm install -g @matiasvillaverde/context-creator-mcp

# Add to your MCP client configuration:
{
  "command": "context-creator-mcp"
}
```
</details>

<details>
<summary>▶️ Install in Augment Code</summary>

1. Open Augment Code settings
2. Navigate to Extensions → MCP
3. Add server:

```json
{
  "context-creator": {
    "command": "npx",
    "args": ["-y", "@matiasvillaverde/context-creator-mcp"]
  }
}
```
</details>

<details>
<summary>▶️ Install in Roo Code</summary>

1. Access Roo Code MCP settings
2. Add new server configuration:

```json
{
  "servers": {
    "context-creator": {
      "command": "npx",
      "args": ["-y", "@matiasvillaverde/context-creator-mcp"]
    }
  }
}
```
</details>

<details>
<summary>▶️ Install in Zencoder</summary>

1. Open Zencoder preferences
2. Go to MCP Configuration
3. Add:

```json
{
  "mcp_servers": [
    {
      "id": "context-creator",
      "command": "npx",
      "args": ["-y", "@matiasvillaverde/context-creator-mcp"]
    }
  ]
}
```
</details>

<details>
<summary>▶️ Install in Amazon Q Developer CLI</summary>

```bash
# Configure Q Developer CLI
q configure mcp add --name context-creator --command "npx -y @matiasvillaverde/context-creator-mcp"

# Verify
q configure mcp list
```
</details>

<details>
<summary>▶️ Install in Qodo Gen</summary>

1. Open Qodo Gen settings
2. Navigate to AI Providers → MCP
3. Add configuration:

```json
{
  "providers": {
    "context-creator": {
      "type": "mcp",
      "command": "npx",
      "args": ["-y", "@matiasvillaverde/context-creator-mcp"]
    }
  }
}
```
</details>

<details>
<summary>▶️ Install in JetBrains AI Assistant</summary>

1. Open IntelliJ IDEA / WebStorm / PyCharm
2. Go to Settings → Tools → AI Assistant → MCP
3. Click "+" to add server:

```json
{
  "name": "context-creator",
  "command": "npx",
  "args": ["-y", "@matiasvillaverde/context-creator-mcp"]
}
```
</details>

<details>
<summary>▶️ Install in Warp</summary>

1. Open Warp settings
2. Navigate to AI → MCP Servers
3. Add configuration:

```json
{
  "servers": [
    {
      "id": "context-creator",
      "command": "npx",
      "args": ["-y", "@matiasvillaverde/context-creator-mcp"]
    }
  ]
}
```
</details>

<details>
<summary>▶️ Install in Opencode</summary>

1. Access Opencode MCP settings
2. Add new server:

```json
{
  "mcp": {
    "context-creator": {
      "command": "npx",
      "args": ["-y", "@matiasvillaverde/context-creator-mcp"]
    }
  }
}
```
</details>

<details>
<summary>▶️ Install in Copilot Coding Agent</summary>

1. Open Copilot settings
2. Navigate to Extensions → MCP Servers
3. Configure:

```json
{
  "mcpServers": [
    {
      "name": "context-creator",
      "command": "npx",
      "args": ["-y", "@matiasvillaverde/context-creator-mcp"]
    }
  ]
}
```
</details>

<details>
<summary>▶️ Install in Kiro</summary>

See [Kiro Model Context Protocol Documentation](https://docs.kiro.ai/mcp) for details.

1. Navigate `Kiro > MCP Servers`
2. Add a new MCP server by clicking the `+ Add` button
3. Paste the configuration given below:

```json
{
  "mcpServers": {
    "context-creator": {
      "command": "npx",
      "args": ["-y", "@matiasvillaverde/context-creator-mcp"]
    }
  }
}
```
</details>

## 🎯 MCP Server Capabilities

Once connected, context-creator provides these powerful tools:

- **`analyze_local`** - Analyze a local codebase directory and answer questions about it
- **`analyze_remote`** - Analyze a remote Git repository (GitHub, GitLab, etc.)
- **`search`** - Search for text patterns across the codebase
- **`semantic_search`** - Find functions, types, imports, and symbols using AST analysis
- **`file_metadata`** - Get detailed information about specific files
- **`diff`** - Generate diffs between two files

### Usage Examples

```
"Analyze the authentication system in this codebase"
"Search for all TODO comments"
"Find all functions that call the login() method"
"What's the difference between old_auth.py and new_auth.py?"
"Analyze the React hooks in facebook/react repository"
```

## ⚙️ Configuration

### `.contextignore` - Exclude Files

Create a `.contextignore` file in your project root to exclude files and directories:

```gitignore
# Dependencies
node_modules/
target/
venv/

# Build outputs
dist/
build/
*.pyc

# Sensitive files
.env
*.key
secrets/
```

### `.contextkeep` - Prioritize Important Files

Create a `.contextkeep` file to ensure critical files are always included:

```gitignore
# Core application files
src/auth/**
src/api/routes.ts
src/models/**

# Configuration
package.json
tsconfig.json
.env.example
```

### `.context-creator.toml` - Advanced Settings

For fine-grained control, create `.context-creator.toml`:

```toml
[defaults]
max_tokens = 200000
include_git_context = true

# File priority rules (first match wins)
[[priorities]]
pattern = "src/core/**"
weight = 100

[[priorities]]
pattern = "tests/**"
weight = 50

[[priorities]]
pattern = "docs/**"
weight = -10  # Lower priority
```

## 📚 Documentation

For detailed documentation, see:
- [Installation Guide](docs/installation.md) - Detailed installation instructions
- [Configuration Guide](docs/configuration.md) - Configuration files and options
- [Usage Examples](docs/usage.md) - CLI usage and examples
- [MCP Server Guide](docs/mcp-server.md) - Advanced MCP server setup
- [Architecture](docs/architecture.md) - Technical architecture details

## 🤝 Contributing

Contributions welcome! Please read [CONTRIBUTING.md](CONTRIBUTING.md) before submitting PRs.

## 📄 License

MIT License - see [LICENSE](LICENSE) for details.
