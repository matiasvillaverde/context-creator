# Troubleshooting Guide

Common issues, solutions, and debugging techniques for context-creator.

## Installation Issues

### Cargo Installation Fails

**Problem**: `cargo install context-creator` fails with compilation errors.

**Solutions**:

```bash
# Update Rust to latest stable
rustup update stable

# Clear cargo cache
cargo clean
rm -rf ~/.cargo/registry/cache

# Install with verbose output
cargo install context-creator --verbose

# Try with specific features
cargo install context-creator --no-default-features
```

**Common Causes**:
- Outdated Rust version (requires 1.70.0+)
- Corrupted cargo cache
- Missing system dependencies

### Permission Denied

**Problem**: Cannot execute context-creator after installation.

**Solutions**:

```bash
# Linux/macOS: Fix permissions
sudo chown $(whoami) /usr/local/bin/context-creator
chmod +x /usr/local/bin/context-creator

# Add to PATH if installed via cargo
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc

# Windows: Run as Administrator or add to user PATH
```

### LLM Tools Not Found

**Problem**: `gemini: command not found` or similar.

**Solutions**:

```bash
# Install gemini
pip install gemini

# Install codex CLI
npm install -g @openai/codex-cli

# Check PATH
echo $PATH
which gemini

# Use full path
context-creator -d project --tool /full/path/to/gemini "prompt"
```

## Runtime Issues

### Out of Memory Errors

**Problem**: Process killed or memory allocation failures.

**Solutions**:

```bash
# Use token limits to reduce memory usage
context-creator -d project --max-tokens 25000

# Process in smaller chunks
context-creator -d project/src --max-tokens 10000
context-creator -d project/lib --max-tokens 10000

# Exclude large files
echo "*.log" >> .contextignore
echo "node_modules/" >> .contextignore
echo "target/" >> .contextignore

# Monitor memory usage
top -p $(pgrep context-creator)
```

**Memory Optimization**:

```toml
# config.toml
[processing]
chunk_size = 500          # Smaller chunks
parallel_jobs = 2         # Fewer workers
memory_limit_mb = 512     # Per-worker limit

[cache]
enabled = false           # Disable caching if needed
```

### Performance Issues

**Problem**: Slow processing of large projects.

**Solutions**:

```bash
# Use all CPU cores
export RAYON_NUM_THREADS=$(nproc)

# Optimize for current CPU
export RUSTFLAGS="-C target-cpu=native"

# Profile performance
time context-creator -d project --max-tokens 50000

# Use parallel processing
context-creator -d project --progress --verbose
```

**Performance Configuration**:

```toml
[processing]
parallel_jobs = 0         # Auto-detect CPU cores
chunk_size = 2000         # Larger chunks for better performance
timeout_seconds = 300     # Prevent hangs

[cache]
enabled = true
max_size_mb = 2048        # Larger cache
```

### Token Count Issues

**Problem**: Output exceeds LLM context window.

**Solutions**:

```bash
# Set appropriate token limits
context-creator -d project --max-tokens 4000    # GPT-3.5
context-creator -d project --max-tokens 8000    # GPT-4
context-creator -d project --max-tokens 32000   # GPT-4 Turbo

# Check token usage
context-creator -d project --max-tokens 10000 --verbose

# Prioritize important files
cat > priority-config.toml << EOF
[[priorities]]
pattern = "src/main.*"
weight = 300.0

[[priorities]]
pattern = "src/core/*"
weight = 200.0
EOF

context-creator -c priority-config.toml -d project --max-tokens 15000
```

### File Access Errors

**Problem**: Permission denied reading files or directories.

**Solutions**:

```bash
# Check permissions
ls -la project/
ls -la project/src/

# Fix ownership
sudo chown -R $(whoami) project/

# Skip inaccessible files
context-creator -d project --verbose 2>&1 | grep -v "Permission denied"

# Use ignore patterns
echo "restricted/" >> .contextignore
```

## Configuration Issues

### Configuration Not Loading

**Problem**: Configuration file ignored or not found.

**Debug Steps**:

```bash
# Check configuration loading
context-creator --show-config

# Test specific config file
context-creator -c config.toml --validate-config

# Debug configuration loading
RUST_LOG=debug context-creator -d project 2>&1 | grep config

# Check file locations
ls -la ~/.config/context-creator/
ls -la .context-creator.toml
```

**Common Issues**:
- TOML syntax errors
- Wrong file location
- Incorrect permissions

### TOML Syntax Errors

**Problem**: Configuration parsing fails.

**Solutions**:

```bash
# Validate TOML syntax
toml-lint config.toml

# Use online validator
# https://www.toml-lint.com/

# Generate valid template
context-creator --config-template > valid-config.toml

# Common fixes:
# - Use double quotes for strings
# - Proper array syntax: ["item1", "item2"]
# - Correct section headers: [section]
```

**Example Fixes**:

```toml
# Wrong
ignore = [target/, node_modules/]

# Correct
ignore = ["target/", "node_modules/"]

# Wrong
[priority]
pattern = src/*.rs

# Correct
[[priorities]]
pattern = "src/*.rs"
```

### Pattern Matching Issues

**Problem**: Files not filtered as expected.

**Debug Patterns**:

```bash
# Test pattern matching
context-creator -d project --dry-run --verbose

# Show matched files
context-creator -d project --show-matches

# Test specific patterns
context-creator -d project --include "src/**/*.rs" --dry-run
```

**Pattern Examples**:

```bash
# Glob patterns (correct)
"src/**/*.rs"      # All .rs files in src/ recursively
"*.{js,ts}"        # All .js and .ts files
"**/test_*.py"     # All test files

# Common mistakes
"src/**.rs"        # Wrong: should be src/**/*.rs
"src/*.rs"         # Only matches direct children
"src**"            # Matches filenames starting with 'src'
```

## LLM Integration Issues

### API Key Problems

**Problem**: LLM tool authentication fails.

**Solutions**:

```bash
# Set API keys
export OPENAI_API_KEY="your-key"
export GOOGLE_API_KEY="your-key"
export ANTHROPIC_API_KEY="your-key"

# Verify keys are set
echo $OPENAI_API_KEY | cut -c1-10

# Test tool directly
gemini --help
codex --version

# Check tool configuration
context-creator --show-tools
```

### LLM Tool Timeouts

**Problem**: LLM requests timeout or fail.

**Solutions**:

```bash
# Increase timeout
context-creator -d project --llm-timeout 120 "prompt"

# Use smaller token limits
context-creator -d project --max-tokens 10000 "prompt"

# Test tool directly
echo "test prompt" | gemini

# Use alternative tool
context-creator -d project --tool codex "prompt"
```

**Configuration**:

```toml
[tools.gemini]
timeout = 120
max_retries = 3

[tools.codex]
timeout = 180
max_retries = 2
```

### Rate Limiting

**Problem**: API rate limits exceeded.

**Solutions**:

```bash
# Add delays between requests
context-creator -d project --llm-delay 5 "prompt"

# Use smaller chunks
context-creator -d project --max-tokens 5000 "prompt"

# Batch process with delays
for dir in src/*; do
    context-creator -d "$dir" --max-tokens 8000 "analyze this module"
    sleep 10
done
```

## Output Issues

### Malformed Markdown

**Problem**: Generated markdown has formatting issues.

**Solutions**:

```bash
# Validate markdown
markdownlint output.md

# Check encoding
file output.md
iconv -f UTF-8 -t UTF-8 output.md > clean-output.md

# Debug template issues
context-creator -d project --show-templates
```

**Template Fixes**:

```toml
[format]
file_header_template = "## {path}"          # Simple format
doc_header_template = "# Code context"       # Remove variables if problematic
```

### Empty or Minimal Output

**Problem**: Output contains very little content.

**Debug Steps**:

```bash
# Check if files are being found
context-creator -d project --dry-run --verbose

# Verify token limits aren't too low
context-creator -d project --max-tokens 50000 --verbose

# Check ignore patterns
context-creator -d project --show-ignored

# Test without token limits
context-creator -d project
```

### Encoding Issues

**Problem**: Special characters or encoding problems.

**Solutions**:

```bash
# Force UTF-8 encoding
export LANG=en_US.UTF-8
export LC_ALL=en_US.UTF-8

# Convert encoding
iconv -f ISO-8859-1 -t UTF-8 input.md > output.md

# Check file encodings
file project/src/*.rs
```

## Platform-Specific Issues

### Windows Issues

**Problem**: Path or permission issues on Windows.

**Solutions**:

```powershell
# Use forward slashes or escape backslashes
context-creator -d "C:/Projects/MyApp"
context-creator -d "C:\\Projects\\MyApp"

# Run as Administrator for system-wide installation
# Right-click Command Prompt -> "Run as Administrator"

# Use Windows Subsystem for Linux (WSL)
wsl
context-creator -d /mnt/c/Projects/MyApp

# Fix line endings
git config --global core.autocrlf true
```

### macOS Issues

**Problem**: Code signing or notarization warnings.

**Solutions**:

```bash
# Allow unsigned binary
sudo spctl --master-disable

# Or allow specific binary
sudo xattr -r -d com.apple.quarantine /usr/local/bin/context-creator

# Install via Homebrew for signed version
brew install context-creator
```

### Linux Issues

**Problem**: Missing dependencies or library issues.

**Solutions**:

```bash
# Install required libraries
sudo apt-get install build-essential pkg-config
sudo apt-get install libssl-dev

# For older distributions
sudo apt-get install libc6-dev

# Check library dependencies
ldd $(which context-creator)

# Use static binary if available
wget https://releases.../context-creator-linux-static
```

## Debugging Techniques

### Enable Debug Logging

```bash
# Full debug output
RUST_LOG=debug context-creator -d project --verbose

# Specific module debugging
RUST_LOG=code_context::core=debug context-creator -d project

# Trace level (very verbose)
RUST_LOG=trace context-creator -d project 2> debug.log

# Filter debug output
RUST_LOG=debug context-creator -d project 2>&1 | grep -E "(ERROR|WARN)"
```

### Performance Profiling

```bash
# Time execution
time context-creator -d project --max-tokens 50000

# Profile with system tools
perf record -g context-creator -d project
perf report

# Memory profiling
valgrind --tool=massif context-creator -d project
```

### Network Debugging

```bash
# Monitor network requests (for LLM integration)
tcpdump -i any port 443

# Check DNS resolution
nslookup api.openai.com

# Test connectivity
curl -v https://api.openai.com/

# Debug proxy issues
export HTTP_PROXY=http://proxy:8080
export HTTPS_PROXY=http://proxy:8080
```

## Recovery Procedures

### Reset Configuration

```bash
# Backup current config
cp ~/.config/context-creator/config.toml ~/.config/context-creator/config.toml.bak

# Reset to defaults
rm ~/.config/context-creator/config.toml
context-creator --generate-config > ~/.config/context-creator/config.toml

# Test with minimal config
context-creator -d project --no-config
```

### Clear Cache

```bash
# Clear application cache
rm -rf ~/.cache/context-creator/

# Clear cargo cache
cargo clean

# Clear temporary files
rm -rf /tmp/context-creator-*
```

### Reinstall

```bash
# Complete reinstall
cargo uninstall context-creator
rm -rf ~/.cargo/registry/cache/
cargo install context-creator

# Or use binary installation
curl -L https://github.com/.../latest/download/... | tar xz
sudo mv context-creator /usr/local/bin/
```

## Getting Help

### Collect Debug Information

```bash
# System information
uname -a
rustc --version
cargo --version

# Application version
context-creator --version

# Configuration dump
context-creator --show-config > debug-config.txt

# Test run with debug output
RUST_LOG=debug context-creator -d small-project 2> debug-output.txt
```

### Report Issues

When reporting issues, include:

1. **System Information**: OS, architecture, Rust version
2. **context-creator Version**: `context-creator --version`
3. **Command Used**: Exact command that failed
4. **Error Output**: Full error messages
5. **Configuration**: Relevant config file contents
6. **Project Structure**: General description of project being analyzed

**Issue Template**:

```markdown
## Bug Report

**Environment:**
- OS: [Linux/macOS/Windows]
- context-creator version: [output of `context-creator --version`]
- Rust version: [output of `rustc --version`]

**Command:**
```bash
context-creator -d my-project --max-tokens 50000
```

**Expected Behavior:**
[What you expected to happen]

**Actual Behavior:**
[What actually happened]

**Error Output:**
```
[Full error message]
```

**Configuration:**
```toml
[Relevant config file contents]
```

**Additional Context:**
[Any other relevant information]
```

### Community Resources

- **GitHub Issues**: [Report bugs and feature requests](https://github.com/matiasvillaverde/context-creator/issues)
- **Discussions**: [Ask questions and share ideas](https://github.com/matiasvillaverde/context-creator/discussions)
- **Documentation**: [Complete documentation](https://docs.rs/context-creator)
- **Examples**: [Community examples repository](https://github.com/matiasvillaverde/context-creator-examples)

## Prevention

### Best Practices

1. **Regular Updates**: Keep context-creator updated
2. **Configuration Validation**: Test configs before deployment
3. **Resource Monitoring**: Monitor memory and CPU usage
4. **Backup Configs**: Keep configuration backups
5. **Test Early**: Test with small projects first

### Health Checks

```bash
# Regular health check script
#!/bin/bash
echo "context-creator Health Check"
echo "========================"

# Version check
echo "Version: $(context-creator --version)"

# Configuration check
if context-creator --validate-config; then
    echo "✓ Configuration valid"
else
    echo "✗ Configuration invalid"
fi

# Performance test
start_time=$(date +%s)
context-creator -d /tmp --max-tokens 1000 --quiet
end_time=$(date +%s)
duration=$((end_time - start_time))

echo "Performance: ${duration}s for small test"

if [ $duration -lt 10 ]; then
    echo "✓ Performance good"
else
    echo "! Performance degraded"
fi
```

This comprehensive troubleshooting guide should help users resolve most common issues they might encounter with context-creator.