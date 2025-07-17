# context-creator 
> Turn your entire codebase into context. 

[![CI](https://github.com/matiasvillaverde/context-creator/actions/workflows/ci.yml/badge.svg)](https://github.com/matiasvillaverde/context-creator/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=flat&logo=rust&logoColor=white)](https://www.rust-lang.org/)

`context-creator` transforms your entire git repository into a single, LLM-optimized Markdown file—intelligently compressed and prioritized—so you can feed it directly into Gemini’s(or any LLM) long-context window for fast, high-level insights and deep architectural understanding.

-----

## Quick Start

```bash
# Ask about improving your local project
context-creator --prompt "Based on this codebase, suggest three ways I can improve performance."

# Ask about architectural patterns in a remote repo
context-creator --repo https://github.com/coderamp-labs/gitingest --prompt "What are the main architectural patterns here? Compare them to common Python best practices."

# Save the formatted Markdown to a file
context-creator -o context.md
```

### Common Use Cases

```bash
# Architecture Review
`context-creator --prompt "Generate a mermaid diagram representing the high-level architecture."`

# Onboarding New Developers
`context-creator --prompt "I'm a new developer. Give me a tour of this codebase, explaining the purpose of the top 5 most important files."`

# Implementing Features
`context-creator --prompt "I need to add a new authentication method using Passkeys. Which files will I need to modify? Provide a step-by-step plan."`

# Security Audit
`context-creator --prompt "Analyze these dependencies and the core logic for potential security vulnerabilities."`
```

### Why `context-creator`?

 #### Gemini’s Long Context
> Stop pasting small snippets. Feed your entire codebase to Gemini in one go and ask **big-picture questions**.

#### Blazing Fast
> Built in Rust with parallel processing, `context-creator` is **dramatically faster** at digesting large repositories.

#### Smart Token Management
> It’s more than a `cat`. `context-creator` respects `.gitignore`, prioritizes critical files via `.digestkeep`, and trims intelligently based on token budgets.

#### Give Claude Code Superpowers
> Claude already excels at precise work. Teach it to use `context-creator`, and it gains a **satellite-level** view of your entire codebase—unlocking deeper context, better answers, and faster development cycles.


## Installation

#### 1\. Install `context-creator`

```bash
# Using Cargo
cargo install context-creator
```

**Prerequisite:** Ensure you have the [Gemini CLI](https://github.com/google/gemini-cli) or [Codex](https://github.com/openai/codex) installed and configured.

#### 2\. Install Gemini CLI (Required for piping)

```bash
npm install -g @google/gemini-cli
gcloud auth application-default login
```

### More Usage Examples

```bash
# Save to file for later use
context-creator -o context.md

# Process specific directories (positional arguments)
context-creator src/ tests/ docs/

# Process specific directories (explicit include flags)
context-creator --include src/ --include tests/ --include docs/

# Process files matching glob patterns (QUOTE patterns to prevent shell expansion)
context-creator --include "**/*.py" --include "src/**/*.{rs,toml}"

# Process specific file types across all directories
context-creator --include "**/*repository*.py" --include "**/test[0-9].py"

# Combine prompt with include patterns for targeted analysis
context-creator --prompt "Review security" --include "src/auth/**" --include "src/security/**"

# Use ignore patterns to exclude unwanted files
context-creator --include "**/*.rs" --ignore "target/**" --ignore "**/*_test.rs"

# Combine prompt with ignore patterns
context-creator --prompt "Analyze core logic" --ignore "tests/**" --ignore "docs/**"

# Process with token limit
context-creator --include src/ --max-tokens 100000
```

## Glob Patterns

Both `--include` and `--ignore` flags support powerful glob patterns for precise file filtering:

### Supported Pattern Syntax

- `*` - matches any characters except `/`
- `?` - matches any single character except `/`
- `**` - recursive directory matching
- `[abc]` - character sets and ranges `[a-z]`
- `{a,b}` - brace expansion (alternatives)
- `[!abc]` - negated character sets

### Pattern Examples

```bash
# Include patterns
context-creator --include "*.py"                              # All Python files
context-creator --include "**/*.rs"                           # All Rust files recursively
context-creator --include "src/**/*.{py,js,ts}"              # Multiple file types in src
context-creator --include "**/*{repository,service,model}*.py" # Specific patterns
context-creator --include "**/test[0-9].py"                  # Numbered test files
context-creator --include "**/db/**"                         # All files in database directories

# Ignore patterns
context-creator --ignore "target/**"                         # Ignore Rust build artifacts
context-creator --ignore "node_modules/**"                   # Ignore Node.js dependencies
context-creator --ignore "**/*_test.rs"                      # Ignore test files
context-creator --ignore "*.{log,tmp,bak}"                   # Ignore temporary files
context-creator --ignore "docs/**"                           # Ignore documentation

# Combined patterns
context-creator --include "**/*.rs" --ignore "target/**" --ignore "**/*_test.rs"
```

### ⚠️ Important: Shell Expansion Prevention

**Always quote your glob patterns** to prevent shell expansion:

```bash
# ✅ CORRECT - quoted pattern
context-creator --include "**/*.py"

# ❌ WRONG - shell may expand before reaching application
context-creator --include **/*.py
```

## Configuration

Fine-tune how `context-creator` processes your repository:

  * **`.digestignore`:** Exclude non-essential files and folders (e.g., `node_modules/`, `target/`).
  * **`.digestkeep`:** Prioritize critical files (e.g., `src/main.rs`, `Cargo.toml`). This ensures the most important code is included when you're near the token limit.
  * **`.context-creator.toml`:** For advanced configuration like setting default token limits and priority weights.

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

### Configuration File (.context-creator.toml)

```toml
[defaults]
max_tokens = 150000
progress = true

[tokens]
gemini = 2000000
codex = 1500000

[[priorities]]
pattern = "src/**/*.rs"
weight = 100

[[priorities]]
pattern = "tests/**/*.rs"
weight = 50
```

### Token Limits Configuration

`context-creator` now supports LLM-specific token limits via the `[tokens]` section in your configuration file. This feature provides smart defaults when using the `--prompt` flag:

**Token Limit Precedence:**
1. **Explicit CLI** (`--max-tokens 500000`) - Always takes precedence
2. **Config Token Limits** (`[tokens]` section) - Used when prompts are provided
3. **Config Defaults** (`[defaults].max_tokens`) - Used for non-prompt operations
4. **Hard-coded Defaults** (1,000,000 tokens) - Fallback for prompts when no config

```toml
# Example: Configure different limits per LLM
[tokens]
gemini = 2000000    # 2M tokens for Gemini
codex = 1500000     # 1.5M tokens for Codex

[defaults]
max_tokens = 150000  # For non-prompt operations
```

**Usage Examples:**
```bash
# Uses config token limits (2M for Gemini, with prompt reservation)
context-creator --prompt "Analyze this codebase" --tool gemini

# Explicit override (500K total, with prompt reservation)
context-creator --prompt "Quick review" --max-tokens 500000

# Uses config defaults (150K for file output, no reservation needed)
context-creator --output-file context.md
```

### Smart Prompt Token Reservation

When using prompts, `context-creator` automatically reserves tokens for:
- **Prompt tokens** (measured using tiktoken)
- **Safety buffer** (1000 tokens for LLM response)

This ensures the total input (prompt + codebase context) fits within the LLM's context window:

```bash
# Example: 2M token limit with 50-token prompt
# Available for codebase: 2,000,000 - 50 - 1,000 = 1,998,950 tokens
context-creator --prompt "Analyze this code" --tool gemini
```
# Security fixes applied
