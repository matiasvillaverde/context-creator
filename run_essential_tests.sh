#!/bin/bash
# Run only essential tests to meet the 1-minute requirement

echo "Running essential tests..."

# Run unit tests (fast)
echo "Running unit tests..."
cargo test --lib --bins

# Run specific critical integration tests
echo "Running critical integration tests..."
cargo test --test semantic_include_types_test
cargo test --test cli_test
cargo test --test integration_test

echo "Essential tests completed!"