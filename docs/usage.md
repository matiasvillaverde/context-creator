# Usage Guide

This guide covers all aspects of using context-creator effectively, from basic commands to advanced workflows.

## Basic Usage

### Simple Markdown Generation

```bash
# Generate markdown for current directory
context-creator

# Process specific directory
context-creator -d /path/to/project

# Save to file
context-creator -d /path/to/project -o project.md

# Process with progress indicators
context-creator -d /path/to/project -o project.md --progress
```

### LLM Integration

```bash
# Direct LLM interaction (requires gemini or codex)
context-creator -d /path/to/project "Explain the architecture"

# With specific LLM tool
context-creator -d /path/to/project --tool gemini "Review this code"
context-creator -d /path/to/project --tool codex "Find potential bugs"

# Analyze specific aspects
context-creator -d /path/to/project "What are the main security concerns?"
context-creator -d /path/to/project "How can we improve performance?"
```

## Command Line Options

### Core Options

```bash
# Directory to process (default: current directory)
context-creator -d /path/to/project
context-creator --directory /path/to/project

# Output file (default: stdout)
context-creator -o output.md
context-creator --output output.md

# Maximum tokens (for token-limited LLMs)
context-creator --max-tokens 50000
context-creator --max-tokens 100000

# LLM tool selection
context-creator -t gemini
context-creator --tool codex

# Configuration file
context-creator -c config.toml
context-creator --config /path/to/config.toml
```

### Verbosity and Output

```bash
# Quiet mode (suppress all output except errors)
context-creator -q
context-creator --quiet

# Verbose mode (DEBUG level logging)
context-creator -v
context-creator --verbose

# Trace mode (TRACE level logging)
context-creator -vv

# JSON-formatted logs (for log aggregation tools)
context-creator --log-format json

# Combined with verbose for structured JSON debug logs
context-creator -v --log-format json

# Progress indicators
context-creator --progress

# Combined options
context-creator -d /path/to/project -o output.md --verbose --progress
```

#### Advanced Logging

The new logging system supports:
- Multiple verbosity levels: `-v` for DEBUG, `-vv` for TRACE
- Log format options with `--log-format` (plain or json) for structured logging
- Environment variable control with `RUST_LOG`
- Module-specific filtering

```bash
# Enable debug logging for specific modules
RUST_LOG=context_creator::walker=debug context-creator

# Enable trace logging for semantic analysis
RUST_LOG=context_creator::semantic=trace context-creator

# JSON logs for processing with tools like jq
context-creator --log-format json -v | jq '.fields'
```

### Help and Information

```bash
# Show help
context-creator -h
context-creator --help

# Show version
context-creator --version

# List supported file types
context-creator --list-types

# Show configuration schema
context-creator --config-schema
```

## Token Management

### Understanding Token Limits

Token limits help optimize output for LLM context windows:

```bash
# GPT-3.5 Turbo (4K context)
context-creator --max-tokens 3000

# GPT-4 (8K context)
context-creator --max-tokens 7000

# GPT-4 Turbo (128K context)
context-creator --max-tokens 120000

# Claude-3 (200K context)
context-creator --max-tokens 180000
```

### Token Usage Examples

```bash
# Small project analysis
context-creator -d small-project --max-tokens 5000

# Medium project with prioritization
context-creator -d medium-project --max-tokens 25000 --verbose

# Large project with careful selection
context-creator -d large-project --max-tokens 100000 --progress

# No token limit (include everything)
context-creator -d project  # No --max-tokens
```

## File Filtering

### Using .contextignore

Create a `.contextignore` file in your project root:

```bash
# Example .contextignore
node_modules/
target/
.git/
*.log
*.tmp
dist/
build/
.DS_Store
__pycache__/
*.pyc
.env
secrets.txt
```

### Configuration-based Filtering

```toml
# In config.toml
ignore = [
    "tests/",
    "docs/",
    "*.md",
    "*.json"
]

include = [
    "src/**/*.rs",
    "lib/**/*.js",
    "*.toml"
]
```

## Configuration Files

### Basic Configuration

```toml
# ~/.config/context-creator/config.toml

[defaults]
max_tokens = 50000
progress = true
verbose = false
tool = "gemini"

ignore = [
    "node_modules/",
    "target/",
    ".git/"
]

[[priorities]]
pattern = "src/main.*"
weight = 200.0

[[priorities]]
pattern = "*.rs"
weight = 150.0

[[priorities]]
pattern = "*.toml"
weight = 100.0
```

### Project-specific Configuration

```bash
# Create project config
cat > .context-creator.toml << EOF
[defaults]
max_tokens = 25000
progress = true

ignore = ["tests/", "benches/"]

[[priorities]]
pattern = "src/core/*"
weight = 200.0
EOF

# Use project config
context-creator -c .context-creator.toml -d .
```

## Workflow Examples

### Code Review Workflow

```bash
# 1. Generate comprehensive review
context-creator -d feature-branch --max-tokens 50000 -o review.md

# 2. Focus on changes
git diff main..feature-branch --name-only | while read file; do
    echo "## $file" >> changes.md
    context-creator -d . --include "$file" >> changes.md
done

# 3. Interactive review with LLM
context-creator -d feature-branch "Review this code for:"
context-creator -d feature-branch "1. Security vulnerabilities"
context-creator -d feature-branch "2. Performance issues"
context-creator -d feature-branch "3. Code quality concerns"
```

### Documentation Generation

```bash
# Generate API documentation
context-creator -d src/ --max-tokens 30000 "Generate API documentation"

# Create onboarding guide
context-creator -d . "Create a guide for new developers"

# Architecture overview
context-creator -d . --max-tokens 20000 "Explain the system architecture"

# Technical debt analysis
context-creator -d . "Identify areas of technical debt"
```

### Migration Planning

```bash
# Analyze codebase for migration
context-creator -d legacy-app "Analyze for Python 2 to 3 migration"
context-creator -d js-app "Plan migration from JavaScript to TypeScript"
context-creator -d . "Identify dependencies for cloud migration"

# Generate migration checklist
context-creator -d . -o migration-analysis.md --max-tokens 40000
```

### Learning and Understanding

```bash
# Understand new codebase
context-creator -d unknown-project "Explain what this project does"

# Learn specific patterns
context-creator -d . "Show examples of the observer pattern"

# Understand architecture
context-creator -d . "Describe the microservices architecture"

# Find examples
context-creator -d . "Show how authentication is implemented"
```

## Advanced Usage

### Parallel Processing

```bash
# Control parallelism
export RAYON_NUM_THREADS=8
context-creator -d large-project

# Process multiple projects
parallel -j4 'context-creator -d {} -o {/.}.md' ::: project1 project2 project3 project4
```

### Batch Processing

```bash
#!/bin/bash
# Process multiple directories

for project in projects/*/; do
    echo "Processing $project..."
    context-creator -d "$project" -o "docs/$(basename "$project").md" --progress
done
```

### Custom Templates

```bash
# Using configuration templates
context-creator -c templates/api-docs.toml -d api/
context-creator -c templates/security-review.toml -d .
context-creator -c templates/performance.toml -d critical-path/
```

### Integration with CI/CD

```yaml
# .github/workflows/documentation.yml
name: Generate Documentation

on:
  push:
    branches: [main]

jobs:
  docs:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Install context-creator
        run: cargo install context-creator
      
      - name: Generate documentation
        run: |
          context-creator -d . -o docs/codebase.md --max-tokens 100000
          
      - name: Commit documentation
        run: |
          git config --local user.email "action@github.com"
          git config --local user.name "GitHub Action"
          git add docs/codebase.md
          git commit -m "Update codebase documentation" || exit 0
          git push
```

## Output Formats

### Standard Markdown

```bash
# Basic markdown output
context-creator -d project -o project.md

# With table of contents
context-creator -d project -o project.md  # TOC included by default

# File tree structure
context-creator -d project -o project.md  # Tree included by default
```

### Custom Formatting

```toml
# Custom format configuration
[format]
include_stats = true
include_tree = true
include_toc = true
group_by_type = false
sort_by_priority = true

file_header_template = "## 📁 {path}"
doc_header_template = "# 🚀 Code context: {directory}"
```

## Performance Optimization

### Large Projects

```bash
# Use token limits for large projects
context-creator -d huge-project --max-tokens 50000 --progress

# Exclude unnecessary files
context-creator -d huge-project -c minimal-config.toml

# Process in chunks
find huge-project -type d -maxdepth 1 | xargs -I {} context-creator -d {} -o {}.md
```

### Memory Management

```bash
# For memory-constrained environments
export CODE_context_CHUNK_SIZE=1000
context-creator -d project --max-tokens 10000

# Streaming mode for very large outputs
context-creator -d project | head -n 10000 > partial.md
```

### Caching

```bash
# Enable caching (experimental)
export CODE_context_CACHE_DIR=~/.cache/context-creator
context-creator -d project -o project.md

# Clear cache
rm -rf ~/.cache/context-creator
```

## Error Handling

### Common Issues and Solutions

```bash
# Permission denied
sudo chown -R $(whoami) /path/to/project
context-creator -d /path/to/project

# Out of memory
context-creator -d project --max-tokens 10000

# LLM tool not found
which gemini  # Check if installed
export PATH="$PATH:/path/to/llm/tools"
context-creator -d project --tool gemini "prompt"

# Configuration error
context-creator --config-schema > schema.json
context-creator -c config.toml --validate-config
```

### Debugging

```bash
# Debug mode
RUST_LOG=debug context-creator -d project --verbose

# Trace execution
RUST_LOG=trace context-creator -d project 2> debug.log

# Check configuration
context-creator -c config.toml --dry-run

# Validate before processing
context-creator -d project --validate-only
```

## Tips and Best Practices

### Performance Tips

1. **Use token limits** for large projects to avoid memory issues
2. **Configure .contextignore** to exclude unnecessary files
3. **Use --progress** for long-running operations
4. **Set appropriate parallelism** with RAYON_NUM_THREADS
5. **Cache frequently accessed projects** (when available)

### Quality Tips

1. **Use descriptive prompts** for LLM integration
2. **Configure priorities** for important files
3. **Review output** before using with LLMs
4. **Use project-specific configs** for consistency
5. **Validate configurations** before deployment

### Security Tips

1. **Review .contextignore** to exclude secrets
2. **Use configuration files** to avoid command-line exposure
3. **Limit token output** for sensitive codebases
4. **Check output files** for sensitive information
5. **Use secure LLM endpoints** for private code

## Next Steps

- Check out [Configuration Reference](configuration.md) for advanced setup
- See [Examples](examples.md) for real-world use cases
- Read [API Reference](api.md) for programmatic usage
- Visit [Troubleshooting](troubleshooting.md) for common issues