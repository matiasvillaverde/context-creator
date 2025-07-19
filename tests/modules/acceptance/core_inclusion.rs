//! Category 1: Core Inclusion and Exclusion Tests
//!
//! These tests validate basic file inclusion/exclusion functionality
//! using path-based and pattern-based filtering.

#![cfg(test)]
#![allow(clippy::needless_borrow)] // project_root is PathBuf, function expects &Path

use super::builders::*;
use super::fixtures::*;
use super::helpers::*;

#[test]
fn scenario_1_1_python_single_directory() {
    // Scenario 1.1 (Python): Process a single directory
    // CLI Flags: src/
    // Project Sketch: src/main.py, src/utils.py, tests/test_main.py
    // Assertion: Output contains src/main.py and src/utils.py; NOT tests/test_main.py

    let (_temp_dir, project_root) = create_python_basic_project();

    // Run context-creator on src/ directory only
    let output = run_context_creator(&["src/"], &project_root);

    // Verify assertions - files appear without src/ prefix since we're running from src/
    assert_contains_file(&output, "main.py");
    assert_contains_file(&output, "utils.py");
    assert_not_contains_file(&output, "test_main.py");

    // Verify file headers are present
    assert_contains_file_header(&output, "main.py");
    assert_contains_file_header(&output, "utils.py");
}

#[test]
fn scenario_1_2_python_glob_pattern() {
    // Scenario 1.2 (Python): Include using glob pattern
    // CLI Flags: --include "**/*.py"
    // Project Sketch: src/main.py, README.md, app/api.py
    // Assertion: Output contains src/main.py and app/api.py; NOT README.md

    let (_temp_dir, project_root) = PythonProjectBuilder::new()
        .add_file("src/main.py", "def main(): pass")
        .add_file("README.md", "# Test Project")
        .add_file("app/api.py", "def api_handler(): pass")
        .build();

    // Run with glob pattern
    let output = run_context_creator(&["--include", "**/*.py"], &project_root);

    // Verify assertions
    assert_contains_file(&output, "src/main.py");
    assert_contains_file(&output, "app/api.py");
    assert_not_contains_file(&output, "README.md");
}

#[test]
fn scenario_1_3_typescript_multiple_directories() {
    // Scenario 1.3 (TypeScript): Process multiple directories
    // CLI Flags: src/ components/
    // Project Sketch: src/index.ts, components/Button.tsx, package.json
    // Assertion: Output contains src/index.ts and components/Button.tsx; NOT package.json

    let (_temp_dir, project_root) = create_typescript_basic_project();

    // Run on multiple directories
    let output = run_context_creator(&["src/", "components/"], &project_root);

    // Verify assertions
    assert_contains_file(&output, "src/index.ts");
    assert_contains_file(&output, "components/Button.tsx");
    assert_not_contains_file(&output, "package.json");
}

#[test]
fn scenario_1_4_typescript_ignore_test_files() {
    // Scenario 1.4 (TypeScript): Ignore test files by pattern
    // CLI Flags: src/ --ignore "**/*.test.ts"
    // Project Sketch: src/utils.ts, src/utils.test.ts
    // Assertion: Output contains src/utils.ts; NOT src/utils.test.ts

    let (_temp_dir, project_root) = create_project_with_test_files();

    // Run with ignore pattern
    let output = run_context_creator(&["src/", "--ignore", "**/*.test.ts"], &project_root);

    // Verify assertions - files appear without src/ prefix since we're running from src/
    assert_contains_file(&output, "utils.ts");
    assert_not_contains_file(&output, "utils.test.ts");

    // Also check that .spec.ts files are still included (not ignored)
    assert_contains_file(&output, "api.spec.ts");
}

#[test]
fn scenario_1_5_rust_ignore_target_directory() {
    // Scenario 1.5 (Rust): Ignore target directory
    // CLI Flags: . --ignore "target/**"
    // Project Sketch: src/main.rs, target/debug/my_app
    // Assertion: Output contains src/main.rs; NOT anything from target directory

    let (_temp_dir, project_root) = create_rust_basic_project();

    // Run from project root with ignore pattern
    let output = run_context_creator(&[".", "--ignore", "target/**"], &project_root);

    // Verify assertions
    assert_contains_file(&output, "src/main.rs");
    assert_contains_file(&output, "src/lib.rs");
    assert_not_contains_file(&output, "target/debug/my_app");

    // Ensure no target directory content is present
    assert!(
        !output.contains("target/"),
        "Output should not contain any target/ paths"
    );
    assert!(
        !output.contains("my_app"),
        "Output should not contain binary file"
    );
}

#[test]
fn test_multiple_include_patterns() {
    // Additional test: Multiple include patterns
    let (_temp_dir, project_root) = PythonProjectBuilder::new()
        .add_file("src/main.py", "# Main file")
        .add_file("src/utils.py", "# Utils")
        .add_file("tests/test_utils.py", "# Tests")
        .add_file("docs/readme.md", "# Docs")
        .add_file("scripts/build.sh", "#!/bin/bash")
        .build();

    // Include only Python files and shell scripts
    let output = run_context_creator(
        &["--include", "**/*.py", "--include", "**/*.sh"],
        &project_root,
    );

    // Python files should be included
    assert_contains_file(&output, "src/main.py");
    assert_contains_file(&output, "src/utils.py");
    assert_contains_file(&output, "tests/test_utils.py");
    assert_contains_file(&output, "scripts/build.sh");

    // Other files should be excluded
    assert_not_contains_file(&output, "docs/readme.md");
}

#[test]
fn test_ignore_pattern_precedence() {
    // Test that ignore patterns take precedence over include patterns
    let (_temp_dir, project_root) = TypeScriptProjectBuilder::new()
        .add_file("src/app.ts", "// App")
        .add_file("src/app.test.ts", "// App tests")
        .add_file("src/lib.ts", "// Lib")
        .add_file("src/lib.test.ts", "// Lib tests")
        .build();

    // Include all TS files but ignore test files
    let output = run_context_creator(
        &["--include", "**/*.ts", "--ignore", "**/*.test.ts"],
        &project_root,
    );

    // Non-test files should be included
    assert_contains_file(&output, "app.ts");
    assert_contains_file(&output, "lib.ts");

    // Test files should be ignored despite matching include pattern
    assert_not_contains_file(&output, "app.test.ts");
    assert_not_contains_file(&output, "lib.test.ts");
}

#[test]
fn test_nested_directory_processing() {
    // Test deeply nested directory structures
    let (_temp_dir, project_root) = RustProjectBuilder::new()
        .add_file("src/main.rs", "fn main() {}")
        .add_file("src/core/mod.rs", "pub mod engine;")
        .add_file("src/core/engine.rs", "pub fn run() {}")
        .add_file("src/utils/helpers/mod.rs", "pub mod string;")
        .add_file("src/utils/helpers/string.rs", "pub fn format() {}")
        .build();

    // Process entire src directory
    let output = run_context_creator(&["src/"], &project_root);

    // All nested files should be included - files appear without src/ prefix
    assert_contains_file(&output, "main.rs");
    assert_contains_file(&output, "core/mod.rs");
    assert_contains_file(&output, "core/engine.rs");
    assert_contains_file(&output, "utils/helpers/mod.rs");
    assert_contains_file(&output, "utils/helpers/string.rs");
}

#[test]
fn test_empty_directory_handling() {
    // Test handling of empty directories
    let temp_dir = tempfile::TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create empty src directory
    std::fs::create_dir_all(project_root.join("src")).unwrap();

    // Should not fail on empty directory
    let output = run_context_creator(&["src/"], &project_root);

    // Output should indicate no files found or be minimal
    assert!(
        output.len() < 1000,
        "Empty directory should produce minimal output"
    );
}

#[test]
fn test_glob_pattern_edge_cases() {
    // Test various glob pattern edge cases
    let (_temp_dir, project_root) = PythonProjectBuilder::new()
        .add_file("test.py", "# Single test")
        .add_file("test1.py", "# Test 1")
        .add_file("test2.py", "# Test 2")
        .add_file("my_test.py", "# My test")
        .add_file("tests/unit_test.py", "# Unit test")
        .build();

    // Pattern: test[0-9].py - should match test1.py and test2.py only
    let output = run_context_creator(&["--include", "test[0-9].py"], &project_root);

    assert_not_contains_file(&output, "test.py");
    assert_contains_file(&output, "test1.py");
    assert_contains_file(&output, "test2.py");
    assert_not_contains_file(&output, "my_test.py");
    assert_not_contains_file(&output, "tests/unit_test.py");
}
