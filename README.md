# code-digest 

Turn your codebase into context. 

[](https://github.com/matiasvillaverde/code-digest/actions/workflows/ci.yml)
[](https://opensource.org/licenses/MIT)
[](https://www.rust-lang.org/)


`code-digest` transforms your entire git repository into a single, LLM-optimized Markdown file—intelligently compressed and prioritized—so you can feed it directly into Gemini’s(or any LLM) long-context window for fast, high-level insights and deep architectural understanding.

-----

## Quick Start

```bash
# Ask about improving your local project
code-digest --prompt "Based on this codebase, suggest three ways I can improve performance."

# Ask about architectural patterns in a remote repo
code-digest --repo https://github.com/coderamp-labs/gitingest --prompt "What are the main architectural patterns here? Compare them to common Python best practices."

# Save the formatted Markdown to a file
code-digest -o context.md
```

### Common Use Cases

```bash
# Architecture Review
`code-digest --prompt "Generate a mermaid diagram representing the high-level architecture."`

# Onboarding New Developers
`code-digest --prompt "I'm a new developer. Give me a tour of this codebase, explaining the purpose of the top 5 most important files."`

# Implementing Features
`code-digest --prompt "I need to add a new authentication method using Passkeys. Which files will I need to modify? Provide a step-by-step plan."`

# Security Audit
`code-digest --prompt "Analyze these dependencies and the core logic for potential security vulnerabilities."`
```

### Why `code-digest`?

 #### Gemini’s Long Context
> Stop pasting small snippets. Feed your entire codebase to Gemini in one go and ask **big-picture questions**.

#### Blazing Fast
> Built in Rust with parallel processing, `code-digest` is **dramatically faster** at digesting large repositories.

#### Smart Token Management
> It’s more than a `cat`. `code-digest` respects `.gitignore`, prioritizes critical files via `.digestkeep`, and trims intelligently based on token budgets.

#### Give Claude Code Superpowers
> Claude already excels at precise work. Teach it to use `code-digest`, and it gains a **satellite-level** view of your entire codebase—unlocking deeper context, better answers, and faster development cycles.


## Installation

#### 1\. Install `code-digest`

```bash
# Using Cargo
cargo install code-digest
```

**Prerequisite:** Ensure you have the [Gemini CLI](https://github.com/google/gemini-cli) or [Codex](https://github.com/openai/codex) installed and configured.

#### 2\. Install Gemini CLI (Required for piping)

```bash
npm install -g @google/gemini-cli
gcloud auth application-default login
```

## Configuration

Fine-tune how `code-digest` processes your repository.

  * **`.digestignore`:** Exclude non-essential files and folders (e.g., `node_modules/`, `target/`).
  * **`.digestkeep`:** Prioritize critical files (e.g., `src/main.rs`, `Cargo.toml`). This ensures the most important code is included when you're near the token limit.
  * **`.code-digest.toml`:** For advanced configuration like setting default token limits and priority weights.
## Configuration

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
