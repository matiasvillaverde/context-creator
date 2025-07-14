//! Integration tests for glob pattern support in --include flag

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs::{self, File};
use tempfile::TempDir;

/// Test that all test scenarios from GitHub issue #16 work correctly
mod glob_pattern_integration_tests {
    use super::*;

    #[test]
    fn test_simple_wildcard_patterns() {
        // Test scenario: code-digest --include "*.py" --prompt "analyze Python files"
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create test files
        File::create(root.join("main.py")).unwrap();
        File::create(root.join("utils.py")).unwrap();
        File::create(root.join("README.md")).unwrap();
        File::create(root.join("config.toml")).unwrap();

        let mut cmd = Command::cargo_bin("code-digest").unwrap();
        cmd.current_dir(root).arg("--include").arg("*.py").arg("--output-file").arg("output.md");

        cmd.assert().success();

        // Check that output file was created and contains only Python files
        let output_content = fs::read_to_string(root.join("output.md")).unwrap();
        assert!(output_content.contains("main.py"));
        assert!(output_content.contains("utils.py"));
        assert!(!output_content.contains("README.md"));
        assert!(!output_content.contains("config.toml"));
    }

    #[test]
    fn test_recursive_directory_matching() {
        // Test scenario: code-digest --include "**/*.rs" --prompt "analyze Rust code"
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

        let mut cmd = Command::cargo_bin("code-digest").unwrap();
        cmd.current_dir(root).arg("--include").arg("**/*.rs").arg("--output-file").arg("output.md");

        cmd.assert().success();

        // Check that output contains all Rust files recursively
        let output_content = fs::read_to_string(root.join("output.md")).unwrap();
        assert!(output_content.contains("main.rs"));
        assert!(output_content.contains("src/lib.rs"));
        assert!(output_content.contains("src/core/mod.rs"));
        assert!(output_content.contains("tests/test.rs"));
        assert!(!output_content.contains("README.md"));
    }

    #[test]
    fn test_brace_expansion() {
        // Test scenario: code-digest --include "src/**/*.{py,js}" --prompt "analyze source files"
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

        let mut cmd = Command::cargo_bin("code-digest").unwrap();
        cmd.current_dir(root)
            .arg("--include")
            .arg("src/**/*.{py,js}")
            .arg("--output-file")
            .arg("output.md");

        cmd.assert().success();

        // Check that output contains only Python and JavaScript files in src/
        let output_content = fs::read_to_string(root.join("output.md")).unwrap();
        assert!(output_content.contains("src/main.py"));
        assert!(output_content.contains("src/app.js"));
        assert!(output_content.contains("src/api/handler.py"));
        assert!(output_content.contains("src/api/client.js"));
        assert!(!output_content.contains("tests/test.py"));
        assert!(!output_content.contains("src/config.toml"));
    }

    #[test]
    fn test_character_sets_and_ranges() {
        // Test scenario: code-digest --include "**/test[0-9].py" --prompt "analyze test files"
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

        let mut cmd = Command::cargo_bin("code-digest").unwrap();
        cmd.current_dir(root)
            .arg("--include")
            .arg("**/test[0-9].py")
            .arg("--output-file")
            .arg("output.md");

        cmd.assert().success();

        // Check that output contains only numbered test files
        let output_content = fs::read_to_string(root.join("output.md")).unwrap();
        assert!(output_content.contains("test1.py"));
        assert!(output_content.contains("test2.py"));
        assert!(output_content.contains("test9.py"));
        assert!(output_content.contains("tests/test3.py"));
        assert!(!output_content.contains("test.py"));
        assert!(!output_content.contains("test10.py"));
        assert!(!output_content.contains("testA.py"));
    }

    #[test]
    fn test_complex_pattern_combinations() {
        // Test scenario: code-digest --include "**/*{repository,service,model}*.py" --include "**/db/**"
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

        let mut cmd = Command::cargo_bin("code-digest").unwrap();
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
        assert!(output_content.contains("src/user_repository.py"));
        assert!(output_content.contains("src/api/auth_service.py"));
        assert!(output_content.contains("src/user_model.py"));

        // Files matching second pattern (**/db/**)
        assert!(output_content.contains("src/db/models/base.py"));
        assert!(output_content.contains("src/db/connection.py"));
        assert!(output_content.contains("db/migrations/001.sql"));

        // Files that should not match
        assert!(!output_content.contains("src/utils.py"));
        assert!(!output_content.contains("src/config.py"));
    }

    #[test]
    fn test_invalid_glob_pattern_error() {
        // Test scenario: code-digest --include "src/[" --prompt "test"
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        let mut cmd = Command::cargo_bin("code-digest").unwrap();
        cmd.current_dir(root)
            .arg("--include")
            .arg("src/[") // Invalid pattern - unclosed bracket
            .arg("--output-file")
            .arg("output.md");

        cmd.assert().failure().stderr(predicate::str::contains("Invalid include pattern"));
    }

    #[test]
    fn test_empty_pattern_handling() {
        // Test that empty patterns are handled gracefully
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        File::create(root.join("test.py")).unwrap();

        let mut cmd = Command::cargo_bin("code-digest").unwrap();
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
        assert!(output_content.contains("test.py"));
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

        let mut cmd = Command::cargo_bin("code-digest").unwrap();
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
        assert!(output_content.contains("main.py"));
        assert!(output_content.contains("app.js"));
        assert!(!output_content.contains("config.rs"));
        assert!(!output_content.contains("README.md"));
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
        let mut cmd = Command::cargo_bin("code-digest").unwrap();
        cmd.current_dir(root)
            .arg("--include")
            .arg("src/*.rs") // This would be expanded by shell if not quoted properly
            .arg("--output-file")
            .arg("output.md");

        cmd.assert().success();

        let output_content = fs::read_to_string(root.join("output.md")).unwrap();
        assert!(output_content.contains("src/main.rs"));
        assert!(output_content.contains("src/lib.rs"));
        assert!(!output_content.contains("test.py"));
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

        let mut cmd = Command::cargo_bin("code-digest").unwrap();
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
    fn test_include_patterns_with_positional_args_conflict() {
        // Test that --include and positional arguments are mutually exclusive
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        let mut cmd = Command::cargo_bin("code-digest").unwrap();
        cmd.current_dir(root)
            .arg("src/") // Positional argument
            .arg("--include")
            .arg("*.py")
            .arg("--output-file")
            .arg("output.md");

        cmd.assert().failure().stderr(predicate::str::contains("cannot be used with"));
    }

    #[test]
    fn test_help_shows_glob_examples() {
        // Test that help text includes glob pattern examples
        let mut cmd = Command::cargo_bin("code-digest").unwrap();
        cmd.arg("--help");

        cmd.assert()
            .success()
            .stdout(predicate::str::contains("glob pattern"))
            .stdout(predicate::str::contains("quote patterns"))
            .stdout(predicate::str::contains("**/*.py"));
    }
}
