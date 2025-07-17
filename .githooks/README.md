# Git Hooks

This directory contains git hooks for the context-creator project.

## Setup

To use these hooks, configure git to use this directory:

```bash
git config core.hooksPath .githooks
```

## Available Hooks

- **pre-commit**: Runs `make validate` to ensure code formatting and linting pass before committing
- **pre-push**: Runs `make test` to ensure all tests pass before pushing

## Manual Installation

If you prefer to install hooks manually:

```bash
cp .githooks/* .git/hooks/
chmod +x .git/hooks/pre-commit .git/hooks/pre-push
```