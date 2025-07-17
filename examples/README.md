# Code context Examples

This directory contains example projects and configuration files to demonstrate the capabilities of `context-creator`.

## Directory Structure

```
examples/
├── sample-rust-project/    # Pure Rust project example
├── sample-mixed-project/   # Mixed-language project (JS, Python)
├── config-examples/        # Configuration file examples
└── README.md              # This file
```

## Sample Projects

### sample-rust-project

A minimal Rust project demonstrating:
- Basic Cargo project structure
- Source files with documentation
- Test modules
- Dependencies

### sample-mixed-project

A mixed-language project showing:
- Node.js/Express backend
- Python utility scripts
- JavaScript tests
- Multiple file types and structures
- Custom `.contextignore` and `.contextkeep` files

## Configuration Examples

### minimal.toml

Shows the bare minimum configuration needed:
- Token limit setting
- Single priority rule

### comprehensive.toml

Demonstrates all available configuration options:
- Detailed priority weights
- Custom ignore patterns
- Output formatting options
- Performance tuning
- Logging configuration

## Testing the Tool

You can test `context-creator` with these examples:

```bash
# Process the Rust project
context-creator -d examples/sample-rust-project

# Process with token limit
context-creator -d examples/sample-mixed-project --max-tokens 10000

# Use custom configuration
context-creator -d examples/sample-mixed-project -c examples/config-examples/comprehensive.toml

# Save output to file
context-creator -d examples/sample-rust-project -o rust-project.md
```

## Creating Your Own Examples

To add new examples:

1. Create a new directory under `examples/`
2. Add relevant source files
3. Include `.contextignore` for exclusions
4. Include `.contextkeep` for priorities
5. Add a README explaining the example

## Tips for Testing

1. **Token Limits**: Test with various `--max-tokens` values to see prioritization in action
2. **File Types**: Include diverse file extensions to test language detection
3. **Large Files**: Add some larger files to test performance and chunking
4. **Binary Files**: Include some binary files to test detection and skipping
5. **Symbolic Links**: Create symlinks to test handling (if supported by your OS)