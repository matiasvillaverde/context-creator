#![cfg(test)]

//! Integration tests for glob pattern support in --include flag

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs::{self, File};
use tempfile::TempDir;

/// Helper function to check if content contains a path, handling both Unix and Windows separators
fn contains_path(content: &str, path: &str) -> bool {
    let unix_path = path.replace('\\', "/");
    let windows_path = path.replace('/', "\\");
    content.contains(&unix_path) || content.contains(&windows_path)
}

/// Test that all test scenarios from GitHub issue #16 work correctly
mod glob_pattern_integration_tests {
    use super::*;

    #[test]
    fn test_simple_wildcard_patterns() {
        // Test scenario: context-creator --include "*.py" --prompt "analyze Python files"
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create test files with actual content
        fs::write(root.join("main.py"), "print('Hello from main')\\n").unwrap();
        fs::write(root.join("utils.py"), "def helper(): pass\\n").unwrap();
        fs::write(root.join("README.md"), "# Test Project\\n").unwrap();
        fs::write(root.join("config.toml"), "[test]\\nvalue = 1\\n").unwrap();

        let mut cmd = Command::cargo_bin("context-creator").unwrap();
        cmd.current_dir(root)
            .arg("--include")
            .arg("*.py")
            .arg("--output-file")
            .arg("output.md");

        // Run command and capture output for debugging
        let output = cmd.output().unwrap();
        if !output.status.success() {
            panic!(
                "Command failed with stderr: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        // Ensure output file exists before reading
        assert!(
            root.join("output.md").exists(),
            "Output file was not created"
        );

        // Check that output file was created and contains only Python files
        let output_content = fs::read_to_string(root.join("output.md")).unwrap();
        assert!(contains_path(&output_content, "main.py"));
        assert!(contains_path(&output_content, "utils.py"));
        assert!(!contains_path(&output_content, "README.md"));
        assert!(!contains_path(&output_content, "config.toml"));
    }

    #[test]
    fn test_recursive_directory_matching() {
        // Test scenario: context-creator --include "**/*.rs" --prompt "analyze Rust code"
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create nested structure
        fs::create_dir_all(root.join("src/core")).unwrap();
        fs::create_dir_all(root.join("tests")).unwrap();
        File::create(root.join("main.rs")).unwrap();
        File::create(root.join("src/lib.rs")).unwrap();
        File::create(root.join("src/core/mod.rs")).unwrap();
        File::create(root.join("tests/test.rs")).unwrap();
        File::create(root.join("README.md")).unwrap();

        let mut cmd = Command::cargo_bin("context-creator").unwrap();
        cmd.current_dir(root)
            .arg("--include")
            .arg("**/*.rs")
            .arg("--output-file")
            .arg("output.md");

        cmd.assert().success();

        // Check that output contains all Rust files recursively
        let output_content = fs::read_to_string(root.join("output.md")).unwrap();
        assert!(contains_path(&output_content, "main.rs"));
        assert!(contains_path(&output_content, "src/lib.rs"));
        assert!(contains_path(&output_content, "src/core/mod.rs"));
        assert!(contains_path(&output_content, "tests/test.rs"));
        assert!(!contains_path(&output_content, "README.md"));
    }

    #[test]
    fn test_brace_expansion() {
        // Test scenario: context-creator --include "src/**/*.{py,js}" --prompt "analyze source files"
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create test files
        fs::create_dir_all(root.join("src/api")).unwrap();
        fs::create_dir_all(root.join("tests")).unwrap();
        File::create(root.join("src/main.py")).unwrap();
        File::create(root.join("src/app.js")).unwrap();
        File::create(root.join("src/api/handler.py")).unwrap();
        File::create(root.join("src/api/client.js")).unwrap();
        File::create(root.join("tests/test.py")).unwrap(); // Should not match
        File::create(root.join("src/config.toml")).unwrap(); // Should not match

        let mut cmd = Command::cargo_bin("context-creator").unwrap();
        cmd.current_dir(root)
            .arg("--include")
            .arg("src/**/*.{py,js}")
            .arg("--output-file")
            .arg("output.md");

        cmd.assert().success();

        // Check that output contains only Python and JavaScript files in src/
        let output_content = fs::read_to_string(root.join("output.md")).unwrap();
        assert!(contains_path(&output_content, "src/main.py"));
        assert!(contains_path(&output_content, "src/app.js"));
        assert!(contains_path(&output_content, "src/api/handler.py"));
        assert!(contains_path(&output_content, "src/api/client.js"));
        assert!(!contains_path(&output_content, "tests/test.py"));
        assert!(!contains_path(&output_content, "src/config.toml"));
    }

    #[test]
    fn test_character_sets_and_ranges() {
        // Test scenario: context-creator --include "**/test[0-9].py" --prompt "analyze test files"
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create test files
        fs::create_dir_all(root.join("tests")).unwrap();
        File::create(root.join("test1.py")).unwrap();
        File::create(root.join("test2.py")).unwrap();
        File::create(root.join("test9.py")).unwrap();
        File::create(root.join("tests/test3.py")).unwrap();
        File::create(root.join("test.py")).unwrap(); // Should not match
        File::create(root.join("test10.py")).unwrap(); // Should not match
        File::create(root.join("testA.py")).unwrap(); // Should not match

        let mut cmd = Command::cargo_bin("context-creator").unwrap();
        cmd.current_dir(root)
            .arg("--include")
            .arg("**/test[0-9].py")
            .arg("--output-file")
            .arg("output.md");

        cmd.assert().success();

        // Check that output contains only numbered test files
        let output_content = fs::read_to_string(root.join("output.md")).unwrap();
        assert!(contains_path(&output_content, "test1.py"));
        assert!(contains_path(&output_content, "test2.py"));
        assert!(contains_path(&output_content, "test9.py"));
        assert!(contains_path(&output_content, "tests/test3.py"));
        assert!(!contains_path(&output_content, "test.py"));
        assert!(!contains_path(&output_content, "test10.py"));
        assert!(!contains_path(&output_content, "testA.py"));
    }

    #[test]
    fn test_complex_pattern_combinations() {
        // Test scenario: context-creator --include "**/*{repository,service,model}*.py" --include "**/db/**"
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create test files
        fs::create_dir_all(root.join("src/db/models")).unwrap();
        fs::create_dir_all(root.join("src/api")).unwrap();
        fs::create_dir_all(root.join("db/migrations")).unwrap();

        File::create(root.join("src/user_repository.py")).unwrap();
        File::create(root.join("src/api/auth_service.py")).unwrap();
        File::create(root.join("src/user_model.py")).unwrap();
        File::create(root.join("src/db/models/base.py")).unwrap();
        File::create(root.join("src/db/connection.py")).unwrap();
        File::create(root.join("db/migrations/001.sql")).unwrap();
        File::create(root.join("src/utils.py")).unwrap(); // Should not match first pattern
        File::create(root.join("src/config.py")).unwrap(); // Should not match first pattern

        let mut cmd = Command::cargo_bin("context-creator").unwrap();
        cmd.current_dir(root)
            .arg("--include")
            .arg("**/*{repository,service,model}*.py")
            .arg("--include")
            .arg("**/db/**")
            .arg("--output-file")
            .arg("output.md");

        cmd.assert().success();

        // Check that output contains files matching either pattern
        let output_content = fs::read_to_string(root.join("output.md")).unwrap();

        // Files matching first pattern (repository, service, model)
        assert!(contains_path(&output_content, "src/user_repository.py"));
        assert!(contains_path(&output_content, "src/api/auth_service.py"));
        assert!(contains_path(&output_content, "src/user_model.py"));

        // Files matching second pattern (**/db/**)
        assert!(contains_path(&output_content, "src/db/models/base.py"));
        assert!(contains_path(&output_content, "src/db/connection.py"));
        assert!(contains_path(&output_content, "db/migrations/001.sql"));

        // Files that should not match
        assert!(!contains_path(&output_content, "src/utils.py"));
        assert!(!contains_path(&output_content, "src/config.py"));
    }

    #[test]
    fn test_invalid_glob_pattern_error() {
        // Test scenario: context-creator --include "src/[" --prompt "test"
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        let mut cmd = Command::cargo_bin("context-creator").unwrap();
        cmd.current_dir(root)
            .arg("--include")
            .arg("src/[") // Invalid pattern - unclosed bracket
            .arg("--output-file")
            .arg("output.md");

        cmd.assert()
            .failure()
            .stderr(predicate::str::contains("Invalid include pattern"));
    }

    #[test]
    fn test_empty_pattern_handling() {
        // Test that empty patterns are handled gracefully
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        File::create(root.join("test.py")).unwrap();

        let mut cmd = Command::cargo_bin("context-creator").unwrap();
        cmd.current_dir(root)
            .arg("--include")
            .arg("") // Empty pattern
            .arg("--include")
            .arg("*.py") // Valid pattern
            .arg("--output-file")
            .arg("output.md");

        cmd.assert().success();

        // Should include the Python file despite empty pattern
        let output_content = fs::read_to_string(root.join("output.md")).unwrap();
        assert!(contains_path(&output_content, "test.py"));
    }

    #[test]
    fn test_multiple_include_patterns() {
        // Test multiple --include flags work additively
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        File::create(root.join("main.py")).unwrap();
        File::create(root.join("app.js")).unwrap();
        File::create(root.join("config.rs")).unwrap();
        File::create(root.join("README.md")).unwrap();

        let mut cmd = Command::cargo_bin("context-creator").unwrap();
        cmd.current_dir(root)
            .arg("--include")
            .arg("*.py")
            .arg("--include")
            .arg("*.js")
            .arg("--output-file")
            .arg("output.md");

        cmd.assert().success();

        // Should include both Python and JavaScript files
        let output_content = fs::read_to_string(root.join("output.md")).unwrap();
        assert!(contains_path(&output_content, "main.py"));
        assert!(contains_path(&output_content, "app.js"));
        assert!(!contains_path(&output_content, "config.rs"));
        assert!(!contains_path(&output_content, "README.md"));
    }

    #[test]
    fn test_quoted_patterns_prevent_shell_expansion() {
        // Test that the CLI properly handles quoted patterns (integration test)
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        fs::create_dir_all(root.join("src")).unwrap();
        File::create(root.join("src/main.rs")).unwrap();
        File::create(root.join("src/lib.rs")).unwrap();
        File::create(root.join("test.py")).unwrap();

        // Test with quoted pattern
        let mut cmd = Command::cargo_bin("context-creator").unwrap();
        cmd.current_dir(root)
            .arg("--include")
            .arg("src/*.rs") // This would be expanded by shell if not quoted properly
            .arg("--output-file")
            .arg("output.md");

        cmd.assert().success();

        let output_content = fs::read_to_string(root.join("output.md")).unwrap();
        assert!(contains_path(&output_content, "src/main.rs"));
        assert!(contains_path(&output_content, "src/lib.rs"));
        assert!(!contains_path(&output_content, "test.py"));
    }
}

/// Test edge cases and error conditions
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_pattern_with_no_matches() {
        // Test that CLI handles gracefully when pattern matches no files
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        File::create(root.join("test.py")).unwrap();
        File::create(root.join("README.md")).unwrap();

        let mut cmd = Command::cargo_bin("context-creator").unwrap();
        cmd.current_dir(root)
            .arg("--include")
            .arg("*.rs") // No Rust files exist
            .arg("--output-file")
            .arg("output.md");

        cmd.assert().success();

        // Should create output file with no content files
        let output_content = fs::read_to_string(root.join("output.md")).unwrap();
        assert!(output_content.contains("Total files: 0"));
    }

    #[test]
    fn test_include_patterns_with_positional_args_now_allowed() {
        // Test that --include and positional arguments are now allowed together
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create the src directory and some files
        fs::create_dir_all(root.join("src")).unwrap();
        File::create(root.join("src/test.py")).unwrap();
        File::create(root.join("src/main.rs")).unwrap();
        File::create(root.join("other.py")).unwrap();

        let mut cmd = Command::cargo_bin("context-creator").unwrap();
        cmd.current_dir(root)
            .arg("src/") // Positional argument
            .arg("--include")
            .arg("*.py")
            .arg("--output-file")
            .arg("output.md");

        cmd.assert().success();

        // Should create output file with content
        let output_content = fs::read_to_string(root.join("output.md")).unwrap();

        // Check if the files were actually included
        // With the current implementation, include patterns operate on the current directory
        // while positional paths specify directories to scan
        assert!(contains_path(&output_content, "test.py")); // Should find test.py in current directory
    }

    #[test]
    fn test_help_shows_glob_examples() {
        // Test that help text includes glob pattern examples
        let mut cmd = Command::cargo_bin("context-creator").unwrap();
        cmd.arg("--help");

        cmd.assert()
            .success()
            .stdout(predicate::str::contains("glob pattern"))
            .stdout(predicate::str::contains("quote patterns"))
            .stdout(predicate::str::contains("**/*.py"));
    }
}
