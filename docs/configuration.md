# Configuration Reference

Complete reference for configuring code-digest through TOML files, environment variables, and command-line arguments.

## Configuration Hierarchy

Code-digest uses the following configuration precedence (highest to lowest):

1. **Command-line arguments** (highest priority)
2. **Environment variables**
3. **Project configuration file** (`.code-digest.toml`)
4. **User configuration file** (`~/.config/code-digest/config.toml`)
5. **System configuration file** (`/etc/code-digest/config.toml`)
6. **Built-in defaults** (lowest priority)

## Configuration File Locations

### Auto-discovery Order

Code-digest automatically searches for configuration files in this order:

```bash
# 1. Current directory
./.code-digest.toml
./code-digest.toml

# 2. User config directory
~/.config/code-digest/config.toml
~/.config/code-digest.toml

# 3. System config directory (Linux/macOS)
/etc/code-digest/config.toml

# 4. Windows AppData
%APPDATA%\code-digest\config.toml
```

### Explicit Configuration

```bash
# Use specific config file
code-digest -c /path/to/config.toml

# Use specific config with environment variable
export CODE_DIGEST_CONFIG="/path/to/config.toml"
code-digest
```

## Configuration File Format

### Complete Example

```toml
# Complete configuration example
# ~/.config/code-digest/config.toml

[defaults]
max_tokens = 50000
progress = true
verbose = false
quiet = false
tool = "gemini-cli"
include_tree = true
include_stats = true
include_toc = true
group_by_type = false
sort_by_priority = true

[format]
file_header_template = "## {path}"
doc_header_template = "# Code Digest: {directory}"

[processing]
parallel_jobs = 0  # 0 = auto-detect CPU cores
chunk_size = 1000
timeout_seconds = 300

[cache]
enabled = true
directory = "~/.cache/code-digest"
max_size_mb = 1024
ttl_hours = 24

# Global ignore patterns (applied to all projects)
ignore = [
    ".git/",
    ".svn/",
    ".hg/",
    "node_modules/",
    "target/",
    "dist/",
    "build/",
    "__pycache__/",
    "*.pyc",
    "*.pyo",
    "*.pyd",
    "*.so",
    "*.dylib",
    "*.dll",
    "*.exe",
    "*.o",
    "*.obj",
    "*.class",
    "*.jar",
    "*.war",
    "*.ear",
    "*.zip",
    "*.tar.gz",
    "*.tar.bz2",
    "*.rar",
    "*.7z",
    "*.log",
    "*.tmp",
    "*.temp",
    "*.cache",
    ".DS_Store",
    "Thumbs.db",
    "*.swp",
    "*.swo",
    "*~"
]

# Global include patterns (overrides ignore)
include = [
    "src/**/*",
    "lib/**/*",
    "*.md",
    "*.toml",
    "*.yaml",
    "*.yml",
    "*.json",
    "Makefile",
    "Dockerfile",
    "*.dockerfile"
]

# File type priorities (higher = more important)
[[priorities]]
pattern = "src/main.*"
weight = 200.0
description = "Main entry points"

[[priorities]]
pattern = "src/lib.*"
weight = 180.0
description = "Library roots"

[[priorities]]
pattern = "*.rs"
weight = 150.0
description = "Rust source files"

[[priorities]]
pattern = "*.py"
weight = 140.0
description = "Python source files"

[[priorities]]
pattern = "*.js"
weight = 130.0
description = "JavaScript source files"

[[priorities]]
pattern = "*.ts"
weight = 130.0
description = "TypeScript source files"

[[priorities]]
pattern = "*.go"
weight = 120.0
description = "Go source files"

[[priorities]]
pattern = "*.java"
weight = 110.0
description = "Java source files"

[[priorities]]
pattern = "*.cpp"
weight = 110.0
description = "C++ source files"

[[priorities]]
pattern = "*.c"
weight = 100.0
description = "C source files"

[[priorities]]
pattern = "*.toml"
weight = 90.0
description = "Configuration files"

[[priorities]]
pattern = "*.yaml"
weight = 80.0
description = "YAML configuration"

[[priorities]]
pattern = "*.yml"
weight = 80.0
description = "YAML configuration"

[[priorities]]
pattern = "*.json"
weight = 70.0
description = "JSON configuration"

[[priorities]]
pattern = "README.*"
weight = 60.0
description = "README files"

[[priorities]]
pattern = "*.md"
weight = 50.0
description = "Markdown documentation"

[[priorities]]
pattern = "tests/**/*"
weight = 30.0
description = "Test files"

[[priorities]]
pattern = "docs/**/*"
weight = 20.0
description = "Documentation files"

# LLM tool configurations
[tools.gemini-cli]
command = "gemini-cli"
args = ["--format", "markdown"]
timeout = 60
max_retries = 3

[tools.codex]
command = "codex"
args = ["--temperature", "0.1"]
timeout = 120
max_retries = 2

[tools.claude]
command = "claude"
args = ["--model", "claude-3-sonnet"]
timeout = 90
max_retries = 3

# Project-specific overrides
[projects]

[projects."~/work/rust-project"]
max_tokens = 100000
ignore = ["target/", "benches/"]

[projects."~/work/js-project"]
max_tokens = 75000
ignore = ["node_modules/", "dist/"]
tool = "codex"

[projects."/opt/legacy-code"]
max_tokens = 25000
verbose = true
progress = true
```

## Configuration Sections

### [defaults]

Default values for command-line options:

```toml
[defaults]
# Maximum tokens in output (0 = unlimited)
max_tokens = 50000

# Show progress indicators
progress = true

# Enable verbose logging
verbose = false

# Suppress all non-error output
quiet = false

# Default LLM tool
tool = "gemini-cli"

# Include file tree in output
include_tree = true

# Include statistics section
include_stats = true

# Include table of contents
include_toc = true

# Group files by type
group_by_type = false

# Sort files by priority
sort_by_priority = true
```

### [format]

Output formatting templates:

```toml
[format]
# Template for file headers (supports {path}, {name}, {ext})
file_header_template = "## {path}"

# Template for document header (supports {directory}, {date}, {time})
doc_header_template = "# Code Digest: {directory}"

# Custom section templates
stats_template = "## 📊 Statistics"
tree_template = "## 📁 File Structure"
toc_template = "## 📋 Table of Contents"
```

### [processing]

Processing and performance options:

```toml
[processing]
# Number of parallel workers (0 = auto-detect)
parallel_jobs = 0

# Files processed per batch
chunk_size = 1000

# Maximum processing time per file (seconds)
timeout_seconds = 300

# Memory limit per worker (MB)
memory_limit_mb = 512

# Enable streaming mode for large outputs
streaming = false
```

### [cache]

Caching configuration:

```toml
[cache]
# Enable caching
enabled = true

# Cache directory
directory = "~/.cache/code-digest"

# Maximum cache size (MB)
max_size_mb = 1024

# Cache entry time-to-live (hours)
ttl_hours = 24

# Cache compression
compress = true

# Auto-cleanup on startup
auto_cleanup = true
```

### ignore and include

File filtering patterns:

```toml
# Files/directories to ignore (glob patterns)
ignore = [
    ".git/",
    "node_modules/",
    "target/",
    "*.log",
    "*.tmp"
]

# Files to include (overrides ignore)
include = [
    "src/**/*",
    "*.toml",
    "README.*"
]
```

### [[priorities]]

File prioritization rules:

```toml
# High-priority pattern
[[priorities]]
pattern = "src/main.*"    # Glob pattern
weight = 200.0           # Priority weight (higher = more important)
description = "Main entry points"  # Optional description

# Medium-priority pattern
[[priorities]]
pattern = "*.rs"
weight = 150.0
description = "Rust source files"

# Conditional priorities
[[priorities]]
pattern = "tests/**/*"
weight = 30.0
description = "Test files"
condition = "include_tests"  # Only apply if condition is true
```

### [tools.*]

LLM tool configurations:

```toml
[tools.gemini-cli]
command = "gemini-cli"           # Command to execute
args = ["--format", "markdown"]  # Default arguments
timeout = 60                     # Timeout in seconds
max_retries = 3                  # Retry attempts
env = { GOOGLE_API_KEY = "..." } # Environment variables

[tools.custom-llm]
command = "/path/to/custom-llm"
args = ["--mode", "code-analysis"]
timeout = 120
working_dir = "/tmp"
```

### [projects]

Project-specific overrides:

```toml
[projects."~/work/project1"]
max_tokens = 100000
tool = "codex"
ignore = ["old-code/"]

[projects."/absolute/path"]
verbose = true
include = ["core/**/*"]
```

## Environment Variables

All configuration options can be set via environment variables using the prefix `CODE_DIGEST_`:

```bash
# Basic options
export CODE_DIGEST_MAX_TOKENS=50000
export CODE_DIGEST_PROGRESS=true
export CODE_DIGEST_VERBOSE=false
export CODE_DIGEST_TOOL=gemini-cli

# Paths
export CODE_DIGEST_CONFIG=/path/to/config.toml
export CODE_DIGEST_CACHE_DIR=/path/to/cache

# Processing
export CODE_DIGEST_PARALLEL_JOBS=8
export CODE_DIGEST_CHUNK_SIZE=1000
export CODE_DIGEST_TIMEOUT_SECONDS=300

# Patterns (comma-separated)
export CODE_DIGEST_IGNORE="target/,node_modules/,*.log"
export CODE_DIGEST_INCLUDE="src/**/*,*.toml"

# LLM integration
export CODE_DIGEST_LLM_TIMEOUT=120
export CODE_DIGEST_LLM_RETRIES=3

# Debug options
export RUST_LOG=debug
export RUST_BACKTRACE=1
```

## Command-Line Arguments

All options can be overridden via command-line arguments:

```bash
# Basic usage
code-digest --max-tokens 25000
code-digest --tool codex
code-digest --verbose --progress

# File filtering
code-digest --ignore "*.log,*.tmp"
code-digest --include "src/**/*"

# Configuration
code-digest --config custom.toml
code-digest --no-config  # Skip all config files

# Output format
code-digest --no-tree --no-stats --no-toc
code-digest --group-by-type
```

## Validation

### Configuration Validation

```bash
# Validate configuration file
code-digest --validate-config -c config.toml

# Show effective configuration
code-digest --show-config

# Generate configuration schema
code-digest --config-schema > schema.json

# Dry run (validate without processing)
code-digest --dry-run -d project
```

### Schema Validation

```json
{
  "$schema": "https://raw.githubusercontent.com/matiasvillaverde/code-digest/main/schema/config.schema.json",
  "type": "object",
  "properties": {
    "defaults": {
      "type": "object",
      "properties": {
        "max_tokens": { "type": "integer", "minimum": 0 },
        "progress": { "type": "boolean" },
        "verbose": { "type": "boolean" },
        "tool": { "type": "string", "enum": ["gemini-cli", "codex", "claude"] }
      }
    }
  }
}
```

## Best Practices

### Configuration Organization

```bash
# Project structure
project/
├── .code-digest.toml      # Project-specific config
├── src/
└── docs/

# User configuration
~/.config/code-digest/
├── config.toml            # Main config
├── rust-projects.toml     # Rust-specific config
└── web-projects.toml      # Web project config
```

### Inheritance and Includes

```toml
# Main config
[defaults]
max_tokens = 50000

# Include specialized configs
include = [
    "rust-projects.toml",
    "web-projects.toml"
]

# Override for specific projects
[projects."~/work/special-project"]
config = "special-config.toml"
```

### Performance Optimization

```toml
[processing]
# Use all CPU cores
parallel_jobs = 0

# Larger chunks for better performance
chunk_size = 2000

# Enable caching for frequently accessed projects
[cache]
enabled = true
max_size_mb = 2048
```

### Security Configuration

```toml
# Secure defaults
ignore = [
    ".env",
    ".env.*",
    "secrets/",
    "*.key",
    "*.pem",
    "*.p12",
    "credentials.json",
    "auth.json",
    "*.secret"
]

[tools.gemini-cli]
# Don't log sensitive data
args = ["--no-log", "--secure"]
```

## Troubleshooting

### Common Configuration Issues

```bash
# Check configuration loading
RUST_LOG=debug code-digest --show-config

# Validate TOML syntax
toml-lint config.toml

# Test pattern matching
code-digest --dry-run --verbose -d project

# Check environment variables
env | grep CODE_DIGEST
```

### Configuration Debugging

```toml
# Add debugging section
[debug]
log_config_loading = true
log_pattern_matching = true
log_priority_calculation = true
show_filtered_files = true
```

## Migration

### From v0.x to v1.0

```bash
# Migrate old configuration
code-digest --migrate-config ~/.code-digest.conf

# Convert environment variables
code-digest --migrate-env > new-config.toml
```

### Configuration Templates

```bash
# Generate template for specific use case
code-digest --template rust-project > rust.toml
code-digest --template web-project > web.toml
code-digest --template minimal > minimal.toml
```

## Advanced Configuration

### Custom File Types

```toml
[[file_types]]
extensions = [".custom"]
language = "custom"
priority = 100.0
processor = "text"

[[file_types]]
extensions = [".proto"]
language = "protobuf"
priority = 120.0
processor = "code"
```

### Conditional Configuration

```toml
# Environment-based configuration
[environments.development]
verbose = true
include = ["tests/**/*"]

[environments.production]
quiet = true
ignore = ["tests/**/*", "docs/**/*"]

# Use with: CODE_DIGEST_ENV=development code-digest
```

### Plugin Configuration

```toml
# Plugin system (future feature)
[[plugins]]
name = "custom-processor"
path = "/path/to/plugin.so"
config = { option1 = "value1" }

[[plugins]]
name = "output-formatter"
url = "https://github.com/user/plugin"
version = "1.2.3"
```

## Next Steps

- See [Usage Guide](usage.md) for practical examples
- Check [Examples](examples.md) for configuration templates
- Read [API Reference](api.md) for programmatic configuration
- Visit [Troubleshooting](troubleshooting.md) for common issues