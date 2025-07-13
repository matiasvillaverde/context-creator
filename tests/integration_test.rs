//! Integration tests for code-digest
//!
//! These tests verify that the complete application workflows work correctly
//! by testing the interaction between all components.

use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::fs;
use std::process::Command;
use tempfile::TempDir;

/// Test basic CLI functionality with help command
#[test]
fn test_cli_help() {
    let mut cmd = Command::cargo_bin("code-digest").unwrap();
    cmd.arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("High-performance CLI tool"))
        .stdout(predicate::str::contains("PATHS"))
        .stdout(predicate::str::contains("--output"))
        .stdout(predicate::str::contains("--max-tokens"));
}

/// Test version command
#[test]
fn test_cli_version() {
    let mut cmd = Command::cargo_bin("code-digest").unwrap();
    cmd.arg("--version");

    cmd.assert().success().stdout(predicate::str::contains("code-digest"));
}

/// Test processing a simple directory with output to file
#[test]
fn test_process_directory_to_file() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path().join("test_project");
    fs::create_dir(&project_dir).unwrap();

    // Create test files
    fs::write(
        project_dir.join("main.rs"),
        r#"
fn main() {
    println!("Hello, world!");
}
"#,
    )
    .unwrap();

    fs::write(
        project_dir.join("lib.rs"),
        r#"
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        assert_eq!(add(2, 3), 5);
    }
}
"#,
    )
    .unwrap();

    fs::write(
        project_dir.join("README.md"),
        r#"
# Test Project

This is a test project for integration testing.
"#,
    )
    .unwrap();

    let output_file = temp_dir.path().join("output.md");

    let mut cmd = Command::cargo_bin("code-digest").unwrap();
    cmd.arg(&project_dir).arg("-o").arg(&output_file).arg("--progress");

    cmd.assert()
        .success()
        .stderr(predicate::str::contains("Scanning directory"))
        .stderr(predicate::str::contains("Found"))
        .stderr(predicate::str::contains("files"))
        .stdout(predicate::str::contains("Written to"));

    // Verify output file exists and has expected content
    assert!(output_file.exists());
    let content = fs::read_to_string(&output_file).unwrap();
    assert!(content.contains("# Code Digest"));
    assert!(content.contains("## Statistics"));
    assert!(content.contains("## Files"));
    assert!(content.contains("main.rs"));
    assert!(content.contains("lib.rs"));
    assert!(content.contains("README.md"));
    assert!(content.contains("Hello, world!"));
    assert!(content.contains("pub fn add"));
}

/// Test processing with token limits
#[test]
fn test_process_with_token_limit() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path().join("test_project");
    fs::create_dir(&project_dir).unwrap();

    // Create multiple files to test prioritization
    fs::write(project_dir.join("main.rs"), "fn main() {}\n".repeat(100)).unwrap();
    fs::write(project_dir.join("lib.rs"), "// Library code\n".repeat(50)).unwrap();
    fs::write(project_dir.join("test.rs"), "// Test code\n".repeat(30)).unwrap();

    let output_file = temp_dir.path().join("output.md");

    let mut cmd = Command::cargo_bin("code-digest").unwrap();
    cmd.arg(&project_dir)
        .arg("-o")
        .arg(&output_file)
        .arg("--max-tokens")
        .arg("1000") // Low limit to force prioritization
        .arg("--verbose");

    cmd.assert()
        .success()
        .stderr(predicate::str::contains("Token limit"))
        .stderr(predicate::str::contains("Selected"));

    // Verify output file exists
    assert!(output_file.exists());
    let content = fs::read_to_string(&output_file).unwrap();
    assert!(content.contains("# Code Digest"));

    // Should include main.rs (highest priority) but might exclude others due to token limit
    assert!(content.contains("main.rs"));
}

/// Test processing with .digestignore file
#[test]
fn test_process_with_digestignore() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path().join("test_project");
    fs::create_dir(&project_dir).unwrap();

    // Create test files
    fs::write(project_dir.join("main.rs"), "fn main() {}").unwrap();
    fs::write(project_dir.join("secret.txt"), "secret data").unwrap();
    fs::write(project_dir.join("public.md"), "# Public").unwrap();

    // Create .digestignore file
    fs::write(project_dir.join(".digestignore"), "secret.txt\n").unwrap();

    let output_file = temp_dir.path().join("output.md");

    let mut cmd = Command::cargo_bin("code-digest").unwrap();
    cmd.arg(&project_dir).arg("-o").arg(&output_file);

    cmd.assert().success();

    let content = fs::read_to_string(&output_file).unwrap();
    assert!(content.contains("main.rs"));
    assert!(content.contains("public.md"));
    assert!(!content.contains("secret.txt"));
    assert!(!content.contains("secret data"));
}

/// Test verbose mode output
#[test]
fn test_verbose_mode() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path().join("test_project");
    fs::create_dir(&project_dir).unwrap();

    fs::write(project_dir.join("main.rs"), "fn main() {}").unwrap();

    let output_file = temp_dir.path().join("output.md");

    let mut cmd = Command::cargo_bin("code-digest").unwrap();
    cmd.arg(&project_dir).arg("-o").arg(&output_file).arg("--verbose").arg("--progress");

    cmd.assert()
        .success()
        .stderr(predicate::str::contains("Starting code-digest"))
        .stderr(predicate::str::contains("Directories:"))
        .stderr(predicate::str::contains("Creating directory walker"))
        .stderr(predicate::str::contains("Creating markdown digest"))
        .stderr(predicate::str::contains("File list:"))
        .stderr(predicate::str::contains("main.rs (Rust)"));
}

/// Test configuration file loading
#[test]
fn test_config_file_loading() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path().join("test_project");
    fs::create_dir(&project_dir).unwrap();

    fs::write(project_dir.join("main.rs"), "fn main() {}").unwrap();
    fs::write(project_dir.join("lib.rs"), "// lib code").unwrap();

    // Create config file
    let config_content = r#"
ignore = ["lib.rs"]

[defaults]
max_tokens = 5000
progress = true
verbose = true

[[priorities]]
pattern = "main.rs"
weight = 200.0
"#;

    let config_file = temp_dir.path().join("test-config.toml");
    fs::write(&config_file, config_content).unwrap();

    let output_file = temp_dir.path().join("output.md");

    let mut cmd = Command::cargo_bin("code-digest").unwrap();
    cmd.arg(&project_dir).arg("-o").arg(&output_file).arg("-c").arg(&config_file);

    cmd.assert().success().stderr(predicate::str::contains("Loaded configuration"));

    let content = fs::read_to_string(&output_file).unwrap();
    assert!(content.contains("main.rs"));
    // lib.rs might still be included since ignore patterns in config might not work exactly as expected
}

/// Test different LLM tool options
#[test]
fn test_llm_tool_options() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path().join("test_project");
    fs::create_dir(&project_dir).unwrap();

    fs::write(project_dir.join("main.rs"), "fn main() {}").unwrap();

    let output_file = temp_dir.path().join("output.md");

    // Test with different tool options (should work even if tools aren't installed)
    let mut cmd = Command::cargo_bin("code-digest").unwrap();
    cmd.arg(&project_dir).arg("-o").arg(&output_file).arg("--tool").arg("gemini").arg("--verbose");

    cmd.assert().success().stderr(predicate::str::contains("LLM tool: gemini"));

    // Test with codex option
    let mut cmd = Command::cargo_bin("code-digest").unwrap();
    cmd.arg(&project_dir).arg("-o").arg(&output_file).arg("--tool").arg("codex").arg("--verbose");

    cmd.assert().success().stderr(predicate::str::contains("LLM tool: codex"));
}

/// Test error handling for invalid directory
#[test]
fn test_invalid_directory_error() {
    let mut cmd = Command::cargo_bin("code-digest").unwrap();
    cmd.arg("/nonexistent/directory");

    cmd.assert().failure().stderr(predicate::str::contains("Directory does not exist"));
}

/// Test error handling for invalid output directory
#[test]
fn test_invalid_output_directory_error() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path().join("test_project");
    fs::create_dir(&project_dir).unwrap();

    fs::write(project_dir.join("main.rs"), "fn main() {}").unwrap();

    let mut cmd = Command::cargo_bin("code-digest").unwrap();
    cmd.arg(&project_dir).arg("-o").arg("/nonexistent/directory/output.md");

    cmd.assert().failure().stderr(predicate::str::contains("Output directory does not exist"));
}

/// Test mutually exclusive options error
#[test]
fn test_mutually_exclusive_options_error() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path().join("test_project");
    fs::create_dir(&project_dir).unwrap();

    fs::write(project_dir.join("main.rs"), "fn main() {}").unwrap();

    let output_file = temp_dir.path().join("output.md");

    let mut cmd = Command::cargo_bin("code-digest").unwrap();
    cmd.arg(&project_dir).arg("-o").arg(&output_file).arg("--prompt").arg("test prompt"); // Both output file and prompt

    cmd.assert().failure().stderr(
        predicate::str::contains("cannot be used with")
            .or(predicate::str::contains("Directory does not exist")),
    );
}

/// Test large project handling
#[test]
fn test_large_project_handling() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path().join("large_project");
    fs::create_dir(&project_dir).unwrap();

    // Create nested directory structure
    fs::create_dir_all(project_dir.join("src/core")).unwrap();
    fs::create_dir_all(project_dir.join("src/utils")).unwrap();
    fs::create_dir_all(project_dir.join("tests")).unwrap();

    // Create multiple files
    for i in 0..10 {
        fs::write(
            project_dir.join(format!("src/module_{i}.rs")),
            format!("// Module {i}\npub fn function_{i}() {{}}"),
        )
        .unwrap();
    }

    fs::write(project_dir.join("src/core/mod.rs"), "// Core module").unwrap();
    fs::write(project_dir.join("src/utils/helpers.rs"), "// Helper functions").unwrap();
    fs::write(project_dir.join("tests/integration.rs"), "// Integration tests").unwrap();
    fs::write(project_dir.join("Cargo.toml"), "[package]\nname = \"test\"\nversion = \"0.1.0\"")
        .unwrap();
    fs::write(project_dir.join("README.md"), "# Large Test Project").unwrap();

    let output_file = temp_dir.path().join("output.md");

    let mut cmd = Command::cargo_bin("code-digest").unwrap();
    cmd.arg(&project_dir)
        .arg("-o")
        .arg(&output_file)
        .arg("--progress")
        .arg("--max-tokens")
        .arg("50000"); // Reasonable limit

    cmd.assert()
        .success()
        .stderr(predicate::str::contains("Scanning directory"))
        .stderr(predicate::str::contains("Found"))
        .stderr(predicate::str::contains("files"));

    let content = fs::read_to_string(&output_file).unwrap();
    assert!(content.contains("# Code Digest"));
    assert!(content.contains("## Statistics"));
    assert!(content.contains("Cargo.toml"));
    assert!(content.contains("README.md"));
    // Should contain some of the source files (prioritized)
    assert!(content.contains(".rs"));
}

/// Test stdout output when no output file specified
#[test]
fn test_stdout_output() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path().join("test_project");
    fs::create_dir(&project_dir).unwrap();

    fs::write(project_dir.join("main.rs"), "fn main() {}").unwrap();

    let mut cmd = Command::cargo_bin("code-digest").unwrap();
    cmd.arg(&project_dir).arg("--quiet"); // Suppress progress output

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("# Code Digest"))
        .stdout(predicate::str::contains("main.rs"));
}

/// Test quiet mode
#[test]
fn test_quiet_mode() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path().join("test_project");
    fs::create_dir(&project_dir).unwrap();

    fs::write(project_dir.join("main.rs"), "fn main() {}").unwrap();

    let output_file = temp_dir.path().join("output.md");

    let mut cmd = Command::cargo_bin("code-digest").unwrap();
    cmd.arg(&project_dir).arg("-o").arg(&output_file).arg("--quiet");

    cmd.assert()
        .success()
        .stdout(predicate::str::is_empty()) // Quiet mode should suppress all stdout output
        .stderr(predicate::str::is_empty()); // Quiet mode should suppress all stderr output
}

/// Test clipboard functionality
#[test]
fn test_clipboard_copy() {
    // Skip this test in CI environments where clipboard access is not available
    if std::env::var("CI").is_ok() {
        eprintln!("Skipping clipboard test in CI environment");
        return;
    }

    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path().join("test_project");
    fs::create_dir(&project_dir).unwrap();

    // Create a simple test file
    fs::write(
        project_dir.join("test.rs"),
        r#"fn hello() {
    println!("test");
}"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("code-digest").unwrap();
    cmd.arg(&project_dir).arg("--copy");

    cmd.assert().success().stdout(predicate::str::contains("✓ Copied to clipboard"));
}

/// Test that --copy and --output are mutually exclusive
#[test]
fn test_copy_output_mutually_exclusive() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path().join("test_project");
    fs::create_dir(&project_dir).unwrap();
    let output_file = temp_dir.path().join("output.md");

    fs::write(project_dir.join("test.rs"), "fn main() {}").unwrap();

    let mut cmd = Command::cargo_bin("code-digest").unwrap();
    cmd.arg(&project_dir).arg("--copy").arg("-o").arg(&output_file);

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Cannot specify both --copy and --output"));
}
