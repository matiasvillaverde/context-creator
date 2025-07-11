# Troubleshooting Guide

Common issues, solutions, and debugging techniques for code-digest.

## Installation Issues

### Cargo Installation Fails

**Problem**: `cargo install code-digest` fails with compilation errors.

**Solutions**:

```bash
# Update Rust to latest stable
rustup update stable

# Clear cargo cache
cargo clean
rm -rf ~/.cargo/registry/cache

# Install with verbose output
cargo install code-digest --verbose

# Try with specific features
cargo install code-digest --no-default-features
```

**Common Causes**:
- Outdated Rust version (requires 1.70.0+)
- Corrupted cargo cache
- Missing system dependencies

### Permission Denied

**Problem**: Cannot execute code-digest after installation.

**Solutions**:

```bash
# Linux/macOS: Fix permissions
sudo chown $(whoami) /usr/local/bin/code-digest
chmod +x /usr/local/bin/code-digest

# Add to PATH if installed via cargo
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc

# Windows: Run as Administrator or add to user PATH
```

### LLM Tools Not Found

**Problem**: `gemini-cli: command not found` or similar.

**Solutions**:

```bash
# Install gemini-cli
pip install gemini-cli

# Install codex CLI
npm install -g @openai/codex-cli

# Check PATH
echo $PATH
which gemini-cli

# Use full path
code-digest -d project --tool /full/path/to/gemini-cli "prompt"
```

## Runtime Issues

### Out of Memory Errors

**Problem**: Process killed or memory allocation failures.

**Solutions**:

```bash
# Use token limits to reduce memory usage
code-digest -d project --max-tokens 25000

# Process in smaller chunks
code-digest -d project/src --max-tokens 10000
code-digest -d project/lib --max-tokens 10000

# Exclude large files
echo "*.log" >> .digestignore
echo "node_modules/" >> .digestignore
echo "target/" >> .digestignore

# Monitor memory usage
top -p $(pgrep code-digest)
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
time code-digest -d project --max-tokens 50000

# Use parallel processing
code-digest -d project --progress --verbose
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
code-digest -d project --max-tokens 4000    # GPT-3.5
code-digest -d project --max-tokens 8000    # GPT-4
code-digest -d project --max-tokens 32000   # GPT-4 Turbo

# Check token usage
code-digest -d project --max-tokens 10000 --verbose

# Prioritize important files
cat > priority-config.toml << EOF
[[priorities]]
pattern = "src/main.*"
weight = 300.0

[[priorities]]
pattern = "src/core/*"
weight = 200.0
EOF

code-digest -c priority-config.toml -d project --max-tokens 15000
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
code-digest -d project --verbose 2>&1 | grep -v "Permission denied"

# Use ignore patterns
echo "restricted/" >> .digestignore
```

## Configuration Issues

### Configuration Not Loading

**Problem**: Configuration file ignored or not found.

**Debug Steps**:

```bash
# Check configuration loading
code-digest --show-config

# Test specific config file
code-digest -c config.toml --validate-config

# Debug configuration loading
RUST_LOG=debug code-digest -d project 2>&1 | grep config

# Check file locations
ls -la ~/.config/code-digest/
ls -la .code-digest.toml
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
code-digest --config-template > valid-config.toml

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
code-digest -d project --dry-run --verbose

# Show matched files
code-digest -d project --show-matches

# Test specific patterns
code-digest -d project --include "src/**/*.rs" --dry-run
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
gemini-cli --help
codex --version

# Check tool configuration
code-digest --show-tools
```

### LLM Tool Timeouts

**Problem**: LLM requests timeout or fail.

**Solutions**:

```bash
# Increase timeout
code-digest -d project --llm-timeout 120 "prompt"

# Use smaller token limits
code-digest -d project --max-tokens 10000 "prompt"

# Test tool directly
echo "test prompt" | gemini-cli

# Use alternative tool
code-digest -d project --tool codex "prompt"
```

**Configuration**:

```toml
[tools.gemini-cli]
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
code-digest -d project --llm-delay 5 "prompt"

# Use smaller chunks
code-digest -d project --max-tokens 5000 "prompt"

# Batch process with delays
for dir in src/*; do
    code-digest -d "$dir" --max-tokens 8000 "analyze this module"
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
code-digest -d project --show-templates
```

**Template Fixes**:

```toml
[format]
file_header_template = "## {path}"          # Simple format
doc_header_template = "# Code Digest"       # Remove variables if problematic
```

### Empty or Minimal Output

**Problem**: Output contains very little content.

**Debug Steps**:

```bash
# Check if files are being found
code-digest -d project --dry-run --verbose

# Verify token limits aren't too low
code-digest -d project --max-tokens 50000 --verbose

# Check ignore patterns
code-digest -d project --show-ignored

# Test without token limits
code-digest -d project
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
code-digest -d "C:/Projects/MyApp"
code-digest -d "C:\\Projects\\MyApp"

# Run as Administrator for system-wide installation
# Right-click Command Prompt -> "Run as Administrator"

# Use Windows Subsystem for Linux (WSL)
wsl
code-digest -d /mnt/c/Projects/MyApp

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
sudo xattr -r -d com.apple.quarantine /usr/local/bin/code-digest

# Install via Homebrew for signed version
brew install code-digest
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
ldd $(which code-digest)

# Use static binary if available
wget https://releases.../code-digest-linux-static
```

## Debugging Techniques

### Enable Debug Logging

```bash
# Full debug output
RUST_LOG=debug code-digest -d project --verbose

# Specific module debugging
RUST_LOG=code_digest::core=debug code-digest -d project

# Trace level (very verbose)
RUST_LOG=trace code-digest -d project 2> debug.log

# Filter debug output
RUST_LOG=debug code-digest -d project 2>&1 | grep -E "(ERROR|WARN)"
```

### Performance Profiling

```bash
# Time execution
time code-digest -d project --max-tokens 50000

# Profile with system tools
perf record -g code-digest -d project
perf report

# Memory profiling
valgrind --tool=massif code-digest -d project
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
cp ~/.config/code-digest/config.toml ~/.config/code-digest/config.toml.bak

# Reset to defaults
rm ~/.config/code-digest/config.toml
code-digest --generate-config > ~/.config/code-digest/config.toml

# Test with minimal config
code-digest -d project --no-config
```

### Clear Cache

```bash
# Clear application cache
rm -rf ~/.cache/code-digest/

# Clear cargo cache
cargo clean

# Clear temporary files
rm -rf /tmp/code-digest-*
```

### Reinstall

```bash
# Complete reinstall
cargo uninstall code-digest
rm -rf ~/.cargo/registry/cache/
cargo install code-digest

# Or use binary installation
curl -L https://github.com/.../latest/download/... | tar xz
sudo mv code-digest /usr/local/bin/
```

## Getting Help

### Collect Debug Information

```bash
# System information
uname -a
rustc --version
cargo --version

# Application version
code-digest --version

# Configuration dump
code-digest --show-config > debug-config.txt

# Test run with debug output
RUST_LOG=debug code-digest -d small-project 2> debug-output.txt
```

### Report Issues

When reporting issues, include:

1. **System Information**: OS, architecture, Rust version
2. **Code-digest Version**: `code-digest --version`
3. **Command Used**: Exact command that failed
4. **Error Output**: Full error messages
5. **Configuration**: Relevant config file contents
6. **Project Structure**: General description of project being analyzed

**Issue Template**:

```markdown
## Bug Report

**Environment:**
- OS: [Linux/macOS/Windows]
- Code-digest version: [output of `code-digest --version`]
- Rust version: [output of `rustc --version`]

**Command:**
```bash
code-digest -d my-project --max-tokens 50000
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

- **GitHub Issues**: [Report bugs and feature requests](https://github.com/matiasvillaverde/code-digest/issues)
- **Discussions**: [Ask questions and share ideas](https://github.com/matiasvillaverde/code-digest/discussions)
- **Documentation**: [Complete documentation](https://docs.rs/code-digest)
- **Examples**: [Community examples repository](https://github.com/matiasvillaverde/code-digest-examples)

## Prevention

### Best Practices

1. **Regular Updates**: Keep code-digest updated
2. **Configuration Validation**: Test configs before deployment
3. **Resource Monitoring**: Monitor memory and CPU usage
4. **Backup Configs**: Keep configuration backups
5. **Test Early**: Test with small projects first

### Health Checks

```bash
# Regular health check script
#!/bin/bash
echo "Code-digest Health Check"
echo "========================"

# Version check
echo "Version: $(code-digest --version)"

# Configuration check
if code-digest --validate-config; then
    echo "✓ Configuration valid"
else
    echo "✗ Configuration invalid"
fi

# Performance test
start_time=$(date +%s)
code-digest -d /tmp --max-tokens 1000 --quiet
end_time=$(date +%s)
duration=$((end_time - start_time))

echo "Performance: ${duration}s for small test"

if [ $duration -lt 10 ]; then
    echo "✓ Performance good"
else
    echo "! Performance degraded"
fi
```

This comprehensive troubleshooting guide should help users resolve most common issues they might encounter with code-digest.