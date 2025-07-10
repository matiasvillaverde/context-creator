# Makefile for the code-digest Rust project

# Use cargo-first approach for toolchain consistency and versioning.
CARGO := cargo

# Set the default goal to 'help' for better user experience.
.DEFAULT_GOAL := help

# Phony targets ensure these commands run even if a file with the same name exists.
.PHONY: help build release check test doc fmt fmt-check lint validate clean bench run-example install dev

# ====================================================================================
# Main Targets
# ====================================================================================

build: ## Build the project in debug mode.
	$(CARGO) build

release: ## Build the project in release mode for production.
	$(CARGO) build --release

test: ## Run all tests.
	$(CARGO) test --all-targets

run-example: ## Run the tool with example usage.
	$(CARGO) run -- --help

install: ## Install the tool locally.
	$(CARGO) install --path .

dev: ## Run in development mode with example arguments.
	$(CARGO) run -- -d examples/sample-project

# ====================================================================================
# Quality & CI
# ====================================================================================

check: ## Check the project for errors quickly, without building.
	$(CARGO) check --all-targets

fmt: ## Format the code using rustfmt.
	$(CARGO) fmt

fmt-check: ## Check if the code is correctly formatted.
	$(CARGO) fmt -- --check

lint: ## Lint the code with clippy for style and correctness issues.
	$(CARGO) clippy --all-targets -- -D warnings

validate: fmt-check lint test ## Run all validation checks (format, lint, test). Ideal for CI.
	@echo "✅ Validation successful."

# ====================================================================================
# Project Utilities
# ====================================================================================

clean: ## Remove build artifacts from the target directory.
	$(CARGO) clean

doc: ## Generate and open project documentation in the browser.
	$(CARGO) doc --open

bench: ## Run benchmarks.
	$(CARGO) bench

update: ## Update dependencies to their latest versions.
	$(CARGO) update

audit: ## Check for security vulnerabilities in dependencies.
	$(CARGO) audit

# ====================================================================================
# Development Helpers
# ====================================================================================

watch: ## Watch for changes and recompile (requires cargo-watch).
	cargo watch -x build

watch-test: ## Watch for changes and run tests (requires cargo-watch).
	cargo watch -x test

coverage: ## Generate test coverage report (requires cargo-tarpaulin).
	cargo tarpaulin --out Html

# ====================================================================================
# Help
# ====================================================================================

help: ## Display this help message.
	@echo "code-digest - High-performance CLI tool to convert codebases to Markdown for LLM context"
	@echo ""
	@echo "Usage: make [target]"
	@echo ""
	@echo "Available targets:"
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_0-9-]+:.*?## / {printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2}' $(MAKEFILE_LIST) | sort