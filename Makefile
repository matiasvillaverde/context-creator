# Makefile for the context-creator Rust project

# Use cargo-first approach for toolchain consistency and versioning.
CARGO := cargo

# Set the default goal to 'help' for better user experience.
.DEFAULT_GOAL := help

# Phony targets ensure these commands run even if a file with the same name exists.
.PHONY: help build release check test test-fast doc fmt fmt-check lint validate clean bench run-example install dev

# ====================================================================================
# Main Targets
# ====================================================================================

build: validate ## Build the project in debug mode (runs all validations first).
	$(CARGO) build

release: validate ## Build the project in release mode for production (runs all validations first).
	$(CARGO) build --release

test: fmt-check lint ## Run all tests (runs format and lint checks first).
	$(CARGO) test --all-targets

test-fast: ## Run essential tests quickly (for CI under 1 minute).
	$(CARGO) test --lib --bins
	$(CARGO) test --test semantic_include_types_test
	$(CARGO) test --test cli_test
	$(CARGO) test --test integration_test

run-example: ## Run the tool with example usage.
	$(CARGO) run -- --help

install: ## Install the tool locally.
	$(CARGO) install --path .

dev: ## Run in development mode with example arguments.
	$(CARGO) run -- examples/sample-project

# ====================================================================================
# Quality & CI
# ====================================================================================

check: ## Check the project for errors quickly, without building.
	$(CARGO) check --all-targets

fmt: ## Format the code using rustfmt.
	$(CARGO) fmt

fmt-check: ## Check if the code is correctly formatted.
	$(CARGO) fmt -- --check

lint: ## Lint the code with clippy.
	$(CARGO) clippy --all-targets --all-features -- -D warnings

validate: fmt-check lint ## Run all validation checks (format, lint). Ideal for pre-build.
	@echo "✅ Code quality validation successful."

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
	@echo "context-creator - High-performance CLI tool to convert codebases to Markdown for LLM context"
	@echo ""
	@echo "Usage: make [target]"
	@echo ""
	@echo "Available targets:"
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_0-9-]+:.*?## / {printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2}' $(MAKEFILE_LIST) | sort