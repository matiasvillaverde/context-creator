# Usage Guide

This guide covers all aspects of using code-digest effectively, from basic commands to advanced workflows.

## Basic Usage

### Simple Markdown Generation

```bash
# Generate markdown for current directory
code-digest

# Process specific directory
code-digest -d /path/to/project

# Save to file
code-digest -d /path/to/project -o project.md

# Process with progress indicators
code-digest -d /path/to/project -o project.md --progress
```

### LLM Integration

```bash
# Direct LLM interaction (requires gemini-cli or codex)
code-digest -d /path/to/project "Explain the architecture"

# With specific LLM tool
code-digest -d /path/to/project --tool gemini-cli "Review this code"
code-digest -d /path/to/project --tool codex "Find potential bugs"

# Analyze specific aspects
code-digest -d /path/to/project "What are the main security concerns?"
code-digest -d /path/to/project "How can we improve performance?"
```

## Command Line Options

### Core Options

```bash
# Directory to process (default: current directory)
code-digest -d /path/to/project
code-digest --directory /path/to/project

# Output file (default: stdout)
code-digest -o output.md
code-digest --output output.md

# Maximum tokens (for token-limited LLMs)
code-digest --max-tokens 50000
code-digest --max-tokens 100000

# LLM tool selection
code-digest -t gemini-cli
code-digest --tool codex

# Configuration file
code-digest -c config.toml
code-digest --config /path/to/config.toml
```

### Verbosity and Output

```bash
# Quiet mode (suppress all output except errors)
code-digest -q
code-digest --quiet

# Verbose mode (detailed logging)
code-digest -v
code-digest --verbose

# Progress indicators
code-digest --progress

# Combined options
code-digest -d /path/to/project -o output.md --verbose --progress
```

### Help and Information

```bash
# Show help
code-digest -h
code-digest --help

# Show version
code-digest --version

# List supported file types
code-digest --list-types

# Show configuration schema
code-digest --config-schema
```

## Token Management

### Understanding Token Limits

Token limits help optimize output for LLM context windows:

```bash
# GPT-3.5 Turbo (4K context)
code-digest --max-tokens 3000

# GPT-4 (8K context)
code-digest --max-tokens 7000

# GPT-4 Turbo (128K context)
code-digest --max-tokens 120000

# Claude-3 (200K context)
code-digest --max-tokens 180000
```

### Token Usage Examples

```bash
# Small project analysis
code-digest -d small-project --max-tokens 5000

# Medium project with prioritization
code-digest -d medium-project --max-tokens 25000 --verbose

# Large project with careful selection
code-digest -d large-project --max-tokens 100000 --progress

# No token limit (include everything)
code-digest -d project  # No --max-tokens
```

## File Filtering

### Using .digestignore

Create a `.digestignore` file in your project root:

```bash
# Example .digestignore
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
# ~/.config/code-digest/config.toml

[defaults]
max_tokens = 50000
progress = true
verbose = false
tool = "gemini-cli"

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
cat > .code-digest.toml << EOF
[defaults]
max_tokens = 25000
progress = true

ignore = ["tests/", "benches/"]

[[priorities]]
pattern = "src/core/*"
weight = 200.0
EOF

# Use project config
code-digest -c .code-digest.toml -d .
```

## Workflow Examples

### Code Review Workflow

```bash
# 1. Generate comprehensive review
code-digest -d feature-branch --max-tokens 50000 -o review.md

# 2. Focus on changes
git diff main..feature-branch --name-only | while read file; do
    echo "## $file" >> changes.md
    code-digest -d . --include "$file" >> changes.md
done

# 3. Interactive review with LLM
code-digest -d feature-branch "Review this code for:"
code-digest -d feature-branch "1. Security vulnerabilities"
code-digest -d feature-branch "2. Performance issues"
code-digest -d feature-branch "3. Code quality concerns"
```

### Documentation Generation

```bash
# Generate API documentation
code-digest -d src/ --max-tokens 30000 "Generate API documentation"

# Create onboarding guide
code-digest -d . "Create a guide for new developers"

# Architecture overview
code-digest -d . --max-tokens 20000 "Explain the system architecture"

# Technical debt analysis
code-digest -d . "Identify areas of technical debt"
```

### Migration Planning

```bash
# Analyze codebase for migration
code-digest -d legacy-app "Analyze for Python 2 to 3 migration"
code-digest -d js-app "Plan migration from JavaScript to TypeScript"
code-digest -d . "Identify dependencies for cloud migration"

# Generate migration checklist
code-digest -d . -o migration-analysis.md --max-tokens 40000
```

### Learning and Understanding

```bash
# Understand new codebase
code-digest -d unknown-project "Explain what this project does"

# Learn specific patterns
code-digest -d . "Show examples of the observer pattern"

# Understand architecture
code-digest -d . "Describe the microservices architecture"

# Find examples
code-digest -d . "Show how authentication is implemented"
```

## Advanced Usage

### Parallel Processing

```bash
# Control parallelism
export RAYON_NUM_THREADS=8
code-digest -d large-project

# Process multiple projects
parallel -j4 'code-digest -d {} -o {/.}.md' ::: project1 project2 project3 project4
```

### Batch Processing

```bash
#!/bin/bash
# Process multiple directories

for project in projects/*/; do
    echo "Processing $project..."
    code-digest -d "$project" -o "docs/$(basename "$project").md" --progress
done
```

### Custom Templates

```bash
# Using configuration templates
code-digest -c templates/api-docs.toml -d api/
code-digest -c templates/security-review.toml -d .
code-digest -c templates/performance.toml -d critical-path/
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
      
      - name: Install code-digest
        run: cargo install code-digest
      
      - name: Generate documentation
        run: |
          code-digest -d . -o docs/codebase.md --max-tokens 100000
          
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
code-digest -d project -o project.md

# With table of contents
code-digest -d project -o project.md  # TOC included by default

# File tree structure
code-digest -d project -o project.md  # Tree included by default
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
doc_header_template = "# 🚀 Code Digest: {directory}"
```

## Performance Optimization

### Large Projects

```bash
# Use token limits for large projects
code-digest -d huge-project --max-tokens 50000 --progress

# Exclude unnecessary files
code-digest -d huge-project -c minimal-config.toml

# Process in chunks
find huge-project -type d -maxdepth 1 | xargs -I {} code-digest -d {} -o {}.md
```

### Memory Management

```bash
# For memory-constrained environments
export CODE_DIGEST_CHUNK_SIZE=1000
code-digest -d project --max-tokens 10000

# Streaming mode for very large outputs
code-digest -d project | head -n 10000 > partial.md
```

### Caching

```bash
# Enable caching (experimental)
export CODE_DIGEST_CACHE_DIR=~/.cache/code-digest
code-digest -d project -o project.md

# Clear cache
rm -rf ~/.cache/code-digest
```

## Error Handling

### Common Issues and Solutions

```bash
# Permission denied
sudo chown -R $(whoami) /path/to/project
code-digest -d /path/to/project

# Out of memory
code-digest -d project --max-tokens 10000

# LLM tool not found
which gemini-cli  # Check if installed
export PATH="$PATH:/path/to/llm/tools"
code-digest -d project --tool gemini-cli "prompt"

# Configuration error
code-digest --config-schema > schema.json
code-digest -c config.toml --validate-config
```

### Debugging

```bash
# Debug mode
RUST_LOG=debug code-digest -d project --verbose

# Trace execution
RUST_LOG=trace code-digest -d project 2> debug.log

# Check configuration
code-digest -c config.toml --dry-run

# Validate before processing
code-digest -d project --validate-only
```

## Tips and Best Practices

### Performance Tips

1. **Use token limits** for large projects to avoid memory issues
2. **Configure .digestignore** to exclude unnecessary files
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

1. **Review .digestignore** to exclude secrets
2. **Use configuration files** to avoid command-line exposure
3. **Limit token output** for sensitive codebases
4. **Check output files** for sensitive information
5. **Use secure LLM endpoints** for private code

## Next Steps

- Check out [Configuration Reference](configuration.md) for advanced setup
- See [Examples](examples.md) for real-world use cases
- Read [API Reference](api.md) for programmatic usage
- Visit [Troubleshooting](troubleshooting.md) for common issues