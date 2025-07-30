# Usage Guide

This guide covers comprehensive usage of context-creator, including CLI commands, semantic analysis features, and advanced options.

## Basic Commands

### Process Current Directory
```bash
context-creator
```

### Save to File
```bash
context-creator -o context.md
```

### Process Specific Directories
```bash
context-creator src/ tests/ docs/
```

## LLM Tool Integration

### Using with Gemini (default)
```bash
context-creator --prompt "Analyze the codebase"
```

### Using with Claude Code
```bash
context-creator --tool claude --prompt "Find security vulnerabilities"
```

### Using with Ollama (local models)
```bash
context-creator --tool ollama --ollama-model llama3 --prompt "Explain this code"
context-creator --tool ollama --ollama-model codellama --prompt "Optimize performance"
```

### Using with Codex
```bash
context-creator --tool codex --prompt "Generate documentation"
```

## Pattern Matching

### Include Patterns
```bash
# Include specific file types (quote to prevent shell expansion)
context-creator --include "**/*.py" --include "src/**/*.{rs,toml}"
```

### Exclude Patterns
```bash
context-creator --ignore "**/*_test.py" --ignore "**/migrations/**"
```

### Combine Includes and Excludes
```bash
context-creator --include "**/*.ts" --ignore "node_modules/**" --ignore "**/*.test.ts"
```

## Semantic Analysis Features

### Dependency Graph Analysis
**Note:** Currently supports Python, TypeScript/JavaScript, and Rust. For other languages, context-creator works as a fast, intelligent concatenation tool.

### Trace Imports
Follow import chains across your codebase:
```bash
# Find all files that depend on your authentication module
context-creator --prompt "Show me everything that uses the auth module" --trace-imports

# Trace specific module dependencies
context-creator --trace-imports --include "**/auth.py"
```

### Include Callers
Find where functions are called:
```bash
# Find all places where login() is called
context-creator --prompt "Where is the login function used?" --include-callers

# Analyze payment processing call chain
context-creator --include-callers --include "**/payment.ts"
```

### Include Type Definitions
Include type definitions and interfaces:
```bash
# Include all type definitions and interfaces
context-creator --prompt "Review the type system" --include-types

# Analyze data models
context-creator --include-types --include "**/models/**"
```

### Control Semantic Depth
Control how deep the dependency graph traversal goes:
```bash
# Shallow analysis (direct dependencies only)
context-creator --prompt "Quick auth overview" --include-types --semantic-depth 1

# Deep analysis (up to 10 levels)
context-creator --prompt "Full dependency analysis" --include-types --semantic-depth 10
```

### Include Git Context
Add git commit history to file headers:
```bash
# Include recent commit messages for each file
context-creator --prompt "Review recent changes" --git-context

# Combine with enhanced context for full metadata
context-creator --enhanced-context --git-context

# Useful for understanding code evolution
context-creator --include "src/auth/**" --git-context --prompt "How has authentication evolved?"
```

When enabled, adds git commit history to each file header:
```markdown
## src/auth/login.rs
Git history:
  - feat: add OAuth2 support by John Doe
  - fix: handle rate limiting in login flow by Jane Smith
  - refactor: extract validation logic by John Doe
```

## Search Command

Search for specific terms across your codebase:

```bash
# Search with automatic semantic analysis
context-creator search "AuthenticationService"

# Search without semantic analysis (faster, but less comprehensive)
context-creator search "TODO" --no-semantic

# Search in specific directories
context-creator search "database" src/ tests/
```

The search command:
- Uses parallel processing across all CPU cores
- Streams files line-by-line (memory efficient)
- Respects `.gitignore` and `.contextignore` patterns
- Automatically enables `--trace-imports`, `--include-callers`, and `--include-types`

## Git Diff Command

Analyze changes between git references:

```bash
# Compare current working directory with last commit
context-creator diff HEAD~1 HEAD

# Compare two branches
context-creator diff main feature-branch

# Compare with specific commit hash
context-creator diff a1b2c3d HEAD

# Save diff analysis to file
context-creator --output-file diff-analysis.md diff HEAD~1 HEAD

# Apply token limits to focus on most important changes
context-creator --max-tokens 50000 diff HEAD~5 HEAD

# Include semantic analysis of changed files
context-creator --trace-imports --include-callers --include-types diff main HEAD
```

The diff command features:
- Security hardened against command injection
- Markdown formatted output
- Token-aware prioritization
- Optional semantic analysis
- Change statistics

## Remote Repositories

```bash
# Analyze any GitHub repository
context-creator --repo https://github.com/rust-lang/rust --prompt "How does the borrow checker work?"

# With specific patterns
context-creator --repo https://github.com/facebook/react --include "**/*.js" --prompt "Explain the reconciliation algorithm"
```

## Advanced Options

### Read Prompt from Stdin
```bash
echo "Find security vulnerabilities" | context-creator --stdin src/
```

### Copy Output to Clipboard (macOS)
```bash
context-creator --include "**/*.py" --copy
```

### Token Limits
```bash
# Cap output to specific token limit
context-creator --max-tokens 100000 --prompt "Analyze the API endpoints"
```

### Verbose Logging
```bash
context-creator -v   # Info level
context-creator -vv  # Debug level
```

## Real-World Examples

### Feature Planning
```bash
context-creator --prompt "I want to implement rate limiting. Show me:
1. Current middleware architecture
2. Files I'll need to modify
3. Suggested implementation approach"
```

### Performance Analysis
```bash
context-creator --prompt "Analyze database queries across the codebase. 
Find N+1 queries and suggest optimizations."
```

### Architecture Understanding
```bash
context-creator --prompt "Explain how user authentication flows through the system.
Include relevant files and create a sequence diagram."
```

### Security Audit
```bash
context-creator --prompt "Review authentication and authorization code for vulnerabilities.
Focus on JWT handling and session management."
```

### Change Analysis
```bash
context-creator diff HEAD~10 HEAD --prompt "Summarize all changes in the last 10 commits.
What are the main features added and potential risks introduced?"
```

## Command-Line Options Reference

### Core Options
- `-d, --directory <PATH>` - Directory to process (default: current directory)
- `-o, --output <FILE>` - Output file (default: stdout)
- `--max-tokens <N>` - Maximum tokens for output
- `-t, --tool <TOOL>` - LLM tool to use (gemini, claude, ollama, codex)
- `-c, --config <FILE>` - Configuration file path
- `--prompt <TEXT>` - Direct prompt for LLM analysis

### Semantic Analysis Options
- `--trace-imports` - Follow import chains
- `--include-callers` - Find function callers
- `--include-types` - Include type definitions
- `--semantic-depth <N>` - Control traversal depth
- `--git-context` - Include git history
- `--no-semantic` - Disable semantic analysis

### Pattern Matching
- `--include <PATTERN>` - Include file patterns
- `--ignore <PATTERN>` - Exclude file patterns

### Output Options
- `-v, --verbose` - Enable debug logging
- `-vv` - Enable trace logging
- `-q, --quiet` - Suppress output except errors
- `--log-format <FORMAT>` - Log format (plain, json)
- `--progress` - Show progress indicators
- `--copy` - Copy output to clipboard

### Repository Options
- `--repo <URL>` - Analyze remote repository
- `--stdin` - Read prompt from stdin

## Performance Tips

1. **Use token limits** for large projects to avoid memory issues
2. **Configure .contextignore** to exclude unnecessary files
3. **Use --progress** for long-running operations
4. **Set appropriate parallelism** with RAYON_NUM_THREADS
5. **Use specific include patterns** to reduce processing time

## Next Steps

- Check out [Configuration Guide](configuration.md) for detailed configuration options
- See [MCP Server Guide](mcp-server.md) for MCP integration details
- Read [Architecture](architecture.md) for technical details
- Visit [Troubleshooting](troubleshooting.md) for common issues