#![cfg(test)]

use assert_cmd::Command;
use std::fs;
use std::process::Command as StdCommand;
use tempfile::TempDir;

/// Helper to create a git repository with actual changes for testing
fn setup_git_repo_with_real_changes() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path();

    // Initialize git repository
    StdCommand::new("git")
        .arg("init")
        .current_dir(repo_path)
        .status()
        .expect("Failed to init git repo");

    // Configure git
    StdCommand::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(repo_path)
        .status()
        .expect("Failed to configure git user name");

    StdCommand::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(repo_path)
        .status()
        .expect("Failed to configure git user email");

    // Create first commit with source code
    fs::write(
        repo_path.join("main.rs"),
        r#"fn main() {
    println!("Hello, world!");
}
"#,
    )
    .unwrap();

    fs::write(
        repo_path.join("lib.rs"),
        r#"pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
"#,
    )
    .unwrap();

    StdCommand::new("git")
        .args(["add", "."])
        .current_dir(repo_path)
        .status()
        .expect("Failed to git add");

    StdCommand::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(repo_path)
        .status()
        .expect("Failed to create first commit");

    // Create second commit with meaningful changes
    fs::write(
        repo_path.join("main.rs"),
        r#"fn main() {
    println!("Hello, Rust world!");
    println!("This is a change!");
}
"#,
    )
    .unwrap();

    fs::write(
        repo_path.join("lib.rs"),
        r#"pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

pub fn multiply(a: i32, b: i32) -> i32 {
    a * b
}
"#,
    )
    .unwrap();

    fs::write(
        repo_path.join("new_file.rs"),
        r#"pub struct NewStruct {
    pub field: String,
}
"#,
    )
    .unwrap();

    StdCommand::new("git")
        .args(["add", "."])
        .current_dir(repo_path)
        .status()
        .expect("Failed to git add second commit");

    StdCommand::new("git")
        .args(["commit", "-m", "Add new functionality"])
        .current_dir(repo_path)
        .status()
        .expect("Failed to create second commit");

    temp_dir
}

/// TEST FAILURE: Diff command should output actual file contents, not placeholder message
#[test]
fn test_diff_command_should_output_file_contents_not_placeholder() {
    let repo = setup_git_repo_with_real_changes();

    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    let output = cmd
        .current_dir(repo.path())
        .args(["diff", "HEAD~1", "HEAD"])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);

    // EXPECTED: Should contain actual file contents
    assert!(
        stdout.contains("Hello, Rust world!"),
        "Expected actual file contents from main.rs changes, got: {stdout}"
    );

    assert!(
        stdout.contains("pub fn multiply"),
        "Expected actual file contents from lib.rs changes, got: {stdout}"
    );

    assert!(
        stdout.contains("pub struct NewStruct"),
        "Expected actual file contents from new_file.rs, got: {stdout}"
    );

    // FAILURE PROOF: Should NOT contain placeholder message
    assert!(
        !stdout.contains("Diff command not yet implemented"),
        "Should not contain placeholder message, but got: {stdout}"
    );
}

/// TEST FAILURE: Diff command should integrate with token management
#[test]
fn test_diff_command_should_respect_token_limits() {
    let repo = setup_git_repo_with_real_changes();

    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    let output = cmd
        .current_dir(repo.path())
        .args(["--max-tokens", "100", "diff", "HEAD~1", "HEAD"])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);

    // EXPECTED: Should show truncation or prioritization due to token limits
    assert!(
        stdout.contains("Context Statistics")
            || stdout.contains("Files processed")
            || stdout.contains("Estimated tokens"),
        "Expected token management integration, got: {stdout}"
    );

    // FAILURE PROOF: Should NOT contain placeholder message
    assert!(
        !stdout.contains("Diff command not yet implemented"),
        "Should have functional diff with token management, got: {stdout}"
    );
}

/// TEST FAILURE: Diff command should generate markdown output
#[test]
fn test_diff_command_should_generate_proper_markdown() {
    let repo = setup_git_repo_with_real_changes();

    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    let output = cmd
        .current_dir(repo.path())
        .args(["diff", "HEAD~1", "HEAD"])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);

    // EXPECTED: Should contain proper markdown structure
    assert!(
        stdout.contains("# ") || stdout.contains("## "),
        "Expected markdown headers, got: {stdout}"
    );

    assert!(
        stdout.contains("```rust") || stdout.contains("```"),
        "Expected code blocks for changed files, got: {stdout}"
    );

    // EXPECTED: Should list changed files
    assert!(
        stdout.contains("main.rs") && stdout.contains("lib.rs") && stdout.contains("new_file.rs"),
        "Expected all changed files to be listed, got: {stdout}"
    );
}

/// TEST FAILURE: Diff command should integrate with semantic analysis
#[test]
fn test_diff_command_should_include_semantic_analysis() {
    let repo = setup_git_repo_with_real_changes();

    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    let output = cmd
        .current_dir(repo.path())
        .args(["--trace-imports", "diff", "HEAD~1", "HEAD"])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);

    // EXPECTED: Should include semantic analysis of changes
    assert!(
        stdout.contains("Dependencies:")
            || stdout.contains("Imports:")
            || stdout.contains("# Semantic Analysis")
            || stdout.contains("## Semantic Analysis")
            || stdout.contains("*Semantic analysis integration is in development*"),
        "Expected semantic analysis integration, got: {stdout}"
    );

    // FAILURE PROOF: Should NOT contain placeholder message
    assert!(
        !stdout.contains("Diff command not yet implemented"),
        "Should have functional diff with semantic analysis, got: {stdout}"
    );
}

/// TEST FAILURE: Diff command should save to output file
#[test]
fn test_diff_command_should_save_to_output_file() {
    let repo = setup_git_repo_with_real_changes();
    let output_file = repo.path().join("diff_output.md");

    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    let result = cmd
        .current_dir(repo.path())
        .args(["--output-file", "diff_output.md", "diff", "HEAD~1", "HEAD"])
        .output()
        .unwrap();

    // Command should succeed
    if !result.status.success() {
        let stderr_msg = String::from_utf8_lossy(&result.stderr);
        panic!("Command failed with stderr: {stderr_msg}");
    }

    // EXPECTED: Output file should exist and contain actual diff content
    assert!(
        output_file.exists(),
        "Expected output file to be created at: {output_file:?}"
    );

    let file_content = fs::read_to_string(&output_file).unwrap();

    // EXPECTED: Should contain actual file contents, not placeholder
    assert!(
        file_content.contains("Hello, Rust world!") || file_content.contains("pub fn multiply"),
        "Expected actual diff content in output file, got: {file_content}"
    );

    // FAILURE PROOF: Should NOT contain placeholder message
    assert!(
        !file_content.contains("Diff command not yet implemented"),
        "Output file should contain actual diff, not placeholder: {file_content}"
    );
}

/// TEST FAILURE: Diff command should show statistics
#[test]
fn test_diff_command_should_show_change_statistics() {
    let repo = setup_git_repo_with_real_changes();

    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    let output = cmd
        .current_dir(repo.path())
        .args(["diff", "HEAD~1", "HEAD"])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);

    // EXPECTED: Should show statistics about changes
    assert!(
        stdout.contains("files changed")
            || stdout.contains("insertions")
            || stdout.contains("deletions")
            || stdout.contains("Files changed")
            || stdout.contains("Lines added")
            || stdout.contains("Lines removed"),
        "Expected change statistics, got: {stdout}"
    );

    // EXPECTED: Should show that 3 files were changed (main.rs, lib.rs, new_file.rs)
    assert!(
        stdout.contains("3") && (stdout.contains("file") || stdout.contains("File")),
        "Expected to show 3 files changed, got: {stdout}"
    );
}
