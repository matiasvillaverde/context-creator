# Contributing to context-creator

First off, thank you for considering contributing to context-creator! It's people like you that make context-creator such a great tool.

## 🤝 Code of Conduct

This project and everyone participating in it is governed by our Code of Conduct. By participating, you are expected to uphold this code. Please report unacceptable behavior to the project maintainers.

## 🚀 Getting Started

### Prerequisites

- Rust 1.74.0 or later
- Git
- Make (optional but recommended)

### Setting Up Your Development Environment

1. Fork the repository on GitHub
2. Clone your fork locally:
   ```bash
   git clone https://github.com/YOUR_USERNAME/context-creator.git
   cd context-creator
   ```

3. Add the upstream repository:
   ```bash
   git remote add upstream https://github.com/matiasvillaverde/context-creator.git
   ```

4. Create a new branch for your feature/fix:
   ```bash
   git checkout -b feature/your-feature-name
   ```

5. Install development dependencies:
   ```bash
   cargo build
   ```

## 🔧 Development Workflow

### Running Tests

```bash
# Run all tests
make test

# Run specific test
cargo test test_name

# Run tests with output
cargo test -- --nocapture
```

### Code Formatting

We use `rustfmt` for consistent code formatting:

```bash
# Format your code
make fmt

# Check formatting without changes
make fmt-check
```

### Linting

We use `clippy` for catching common mistakes:

```bash
# Run clippy
make lint

# Run with pedantic lints
cargo clippy -- -W clippy::pedantic
```

### Running All Checks

Before submitting a PR, run all checks:

```bash
make validate
```

## 📝 Making Changes

### Code Style

- Follow Rust naming conventions
- Use meaningful variable and function names
- Add comments for complex logic
- Keep functions focused and small
- Write unit tests for new functionality

### Commit Messages

We follow conventional commits:

- `feat:` New features
- `fix:` Bug fixes
- `docs:` Documentation changes
- `test:` Test additions/changes
- `chore:` Maintenance tasks
- `perf:` Performance improvements
- `refactor:` Code refactoring

Example:
```
feat: add support for custom ignore patterns

- Implement .contextignore file parsing
- Add tests for ignore pattern matching
- Update documentation
```

### Testing

- Write tests for all new functionality
- Ensure all tests pass before submitting
- Include both unit and integration tests where appropriate
- Test edge cases and error conditions

## 🎯 Pull Request Process

1. Update documentation for any changed functionality
2. Add tests for your changes
3. Ensure all tests pass
4. Update the README.md if needed
5. Submit a pull request with a clear description

### PR Checklist

- [ ] Tests pass locally
- [ ] Code is formatted (`make fmt`)
- [ ] Lints pass (`make lint`)
- [ ] Documentation is updated
- [ ] Commit messages follow conventions
- [ ] PR description explains the changes

## 🏗️ Architecture Guidelines

### Module Organization

- `cli.rs` - CLI argument parsing only
- `core/` - Core business logic
- `utils/` - Shared utilities
- Keep modules focused on a single responsibility

### Error Handling

- Use `thiserror` for custom error types
- Use `anyhow` for error propagation
- Provide helpful error messages for users
- Handle edge cases gracefully

### Performance Considerations

- Use `rayon` for parallelizable operations
- Avoid unnecessary allocations
- Profile before optimizing
- Document performance-critical code

## 📚 Documentation

### Code Documentation

- Add doc comments (`///`) for public APIs
- Include examples in doc comments
- Document complex algorithms
- Keep documentation up to date

### User Documentation

- Update README.md for user-facing changes
- Add examples for new features
- Document configuration options
- Include troubleshooting tips

## 🐛 Reporting Issues

### Before Submitting an Issue

- Check existing issues for duplicates
- Try the latest version
- Collect relevant information:
  - Rust version (`rustc --version`)
  - Operating system
  - Steps to reproduce
  - Error messages

### Issue Template

```markdown
## Description
Brief description of the issue

## Steps to Reproduce
1. Run command X
2. See error Y

## Expected Behavior
What should happen

## Actual Behavior
What actually happens

## Environment
- OS: [e.g., macOS 13.0]
- Rust: [e.g., 1.74.0]
- context-creator version: [e.g., 0.1.0]
```

## 💡 Suggesting Features

We love feature suggestions! Please provide:

- Use case description
- Proposed solution
- Alternative solutions considered
- Any implementation ideas

## 🎉 Recognition

Contributors will be recognized in:
- The project README
- Release notes
- GitHub contributors page

## 📧 Questions?

Feel free to:
- Open a discussion on GitHub
- Ask in issues (tag as "question")
- Contact maintainers directly

## 🚀 Release Process

1. Update version in `Cargo.toml`
2. Update CHANGELOG.md
3. Create a PR with version bump
4. After merge, tag the release
5. GitHub Actions will build and publish

Thank you for contributing to context-creator! 🙏