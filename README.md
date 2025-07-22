# context-creator
> Intelligent context engineering for LLM-powered development

[![CI](https://github.com/matiasvillaverde/context-creator/actions/workflows/ci.yml/badge.svg)](https://github.com/matiasvillaverde/context-creator/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=flat&logo=rust&logoColor=white)](https://www.rust-lang.org/)

`context-creator` transforms your codebase into intelligently curated LLM context. Unlike simple concatenation tools, it builds a dependency graph to create relevant, focused contexts that make your AI-powered development actually work.

## Why context-creator?

**🎯 Smart Context Engineering**  
Creates a dependency graph of your codebase. When you ask about authentication, it includes auth files, their dependencies, and related tests—nothing more, nothing less.

**⚡ Blazing Fast**  
Built in Rust with parallel processing. Handles massive codebases in seconds, not minutes.

**🧠 Intelligent Prioritization**  
When hitting token limits, it keeps the most important files based on Git history, dependencies, and your `.contextkeep` rules.

**🚀 Direct LLM Integration**  
Pipe directly to Gemini (or any LLM) for instant answers about your codebase.

## Quick Start

```bash
# Install
cargo install context-creator

# Ask Gemini about your codebase
context-creator --prompt "How can I add 2FA to the authentication system?"

# Analyze a specific feature area
context-creator --prompt "Find all performance bottlenecks in the API layer"

# Plan implementation work
context-creator --prompt "I need to add WebAuthn support. Which files need changes?"

# Architecture review
context-creator --prompt "Generate a dependency graph of the payment processing module"
```

## Real-World Examples

### 🔍 Feature Planning
```bash
context-creator --prompt "I want to implement rate limiting. Show me:
1. Current middleware architecture
2. Files I'll need to modify
3. Suggested implementation approach"
```

### 🐛 Performance Analysis
```bash
context-creator --prompt "Analyze database queries across the codebase. 
Find N+1 queries and suggest optimizations."
```

### 🏗️ Architecture Understanding
```bash
context-creator --prompt "Explain how user authentication flows through the system.
Include relevant files and create a sequence diagram."
```

### 🔒 Security Audit
```bash
context-creator --prompt "Review authentication and authorization code for vulnerabilities.
Focus on JWT handling and session management."
```

## How It Works

Unlike tools that simply concatenate files, `context-creator`:

1. **Builds a dependency graph** of your entire codebase
2. **Extracts relevant subgraphs** based on your query
3. **Prioritizes files** by importance (Git history, dependencies, explicit rules)
4. **Optimizes for token limits** by intelligently pruning less relevant files
5. **Streams to LLMs** with context-aware ordering (important files last)

## Advanced Context Building

### 🔗 Dependency Graph Features

**Note:** Dependency graph analysis currently supports **Python**, **TypeScript/JavaScript**, and **Rust**. For other languages, `context-creator` works as a fast, intelligent concatenation tool.

#### `--trace-imports` - Follow Import Chains
```bash
# Find all files that depend on your authentication module
context-creator --prompt "Show me everything that uses the auth module" --trace-imports

# Trace specific module dependencies
context-creator --trace-imports --include "**/auth.py"
```

#### `--include-callers` - Find Function Usage
```bash
# Find all places where login() is called
context-creator --prompt "Where is the login function used?" --include-callers

# Analyze payment processing call chain
context-creator --include-callers --include "**/payment.ts"
```

#### `--include-types` - Include Type Definitions
```bash
# Include all type definitions and interfaces
context-creator --prompt "Review the type system" --include-types

# Analyze data models
context-creator --include-types --include "**/models/**"
```

#### `--semantic-depth` - Control Traversal Depth
```bash
# Shallow analysis (direct dependencies only)
context-creator --prompt "Quick auth overview" --include-types --semantic-depth 1

# Deep analysis (up to 10 levels)
context-creator --prompt "Full dependency analysis" --include-types --semantic-depth 10
```

### 📊 Real-World Dependency Graph Example

When you run:
```bash
context-creator --prompt "How does the payment system work?" --include "src/PaymentService.rs" --trace-imports --include-callers --include-types
```

The tool:
1. Finds `PaymentService.rs` and related files
2. Traces all imports (Stripe SDK, database models, utility functions)
3. Finds all callers (checkout flow, refund handlers, admin tools)
4. Builds a complete context of how payments flow through your system

## Configuration

### `.contextkeep` - Prioritize Critical Files
```gitignore
# Always include these when relevant
src/auth/**
src/core/**
Cargo.toml
package.json
```

### `.contextignore` - Exclude Noise
```gitignore
# Never include
target/
node_modules/
*.log
.env
```

### `.context-creator.toml` - Advanced Config
```toml
[defaults]
max_tokens = 200000

# First-match-wins priority rules
[[priorities]]
pattern = "src/core/**"
weight = 100

[[priorities]]
pattern = "tests/**"
weight = 50

[[priorities]]
pattern = "docs/**"
weight = -10  # Negative weight = lower priority
```

## Installation

```bash
# Using Cargo
cargo install context-creator

# Prerequisites: Gemini CLI (for --prompt)
npm install -g @google/gemini-cli
gcloud auth application-default login
```

## Usage Examples

### Basic Usage
```bash
# Process current directory
context-creator

# Save to file instead of piping to LLM
context-creator -o context.md

# Process specific directories
context-creator src/ tests/ docs/
```

### Pattern Matching
```bash
# Include specific file types (quote to prevent shell expansion)
context-creator --include "**/*.py" --include "src/**/*.{rs,toml}"

# Exclude patterns
context-creator --ignore "**/*_test.py" --ignore "**/migrations/**"

# Combine includes and excludes
context-creator --include "**/*.ts" --ignore "node_modules/**" --ignore "**/*.test.ts"
```

### Remote Repositories
```bash
# Analyze any GitHub repository
context-creator --repo https://github.com/rust-lang/rust --prompt "How does the borrow checker work?"

# With specific patterns
context-creator --repo https://github.com/facebook/react --include "**/*.js" --prompt "Explain the reconciliation algorithm"
```

### Advanced Combinations
```bash
# Read prompt from stdin
echo "Find security vulnerabilities" | context-creator --stdin src/

# Copy output to clipboard (macOS)
context-creator --include "**/*.py" --copy

# Cap output to specific token limit
context-creator --max-tokens 100000 --prompt "Analyze the API endpoints"

# Enable verbose logging for debugging
context-creator -vv --prompt "Why is this slow?"
```

## Performance

Benchmarked on large codebases:

| Codebase | Files | context-creator | Alternative Tools |
|----------|-------|-----------------|-------------------|
| Next.js  | 5,000 | 3.2s           | 45s+             |
| Rust std | 8,000 | 5.1s           | 2min+            |
| Linux    | 70,000| 28s            | 10min+           |

## Token Management

When using `--prompt`, context-creator automatically:
- Measures prompt tokens
- Reserves space for LLM response
- Prioritizes files to fit within limits
- Removes least important files first

```bash
# With 2M token limit and 50-token prompt:
# Available for code: 2,000,000 - 50 - 1,000 = 1,998,950 tokens
context-creator --prompt "Analyze auth flow" --max-tokens 2000000
```

## Language Support

| Feature | Python | TypeScript/JavaScript | Rust | Other Languages |
|---------|--------|--------------------|------|-----------------|
| Basic concatenation | ✅ | ✅ | ✅ | ✅ |
| Import tracing | ✅ | ✅ | ✅ | ❌ |
| Caller detection | ✅ | ✅ | ✅ | ❌ |
| Type extraction | ✅ | ✅ | ✅ | ❌ |
| Dependency graph | ✅ | ✅ | ✅ | ❌ |

For unsupported languages, `context-creator` still provides intelligent file prioritization, Git-based importance scoring, and fast concatenation.
