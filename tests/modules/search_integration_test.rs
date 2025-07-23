//! Integration tests for search command functionality

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

#[test]
fn test_search_command_finds_files() {
    let temp_dir = TempDir::new().unwrap();

    // Create test files
    std::fs::write(temp_dir.path().join("auth.rs"), "fn authenticate() {}").unwrap();
    std::fs::write(temp_dir.path().join("main.rs"), "// No auth here").unwrap();

    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg("search")
        .arg("authenticate")
        .arg(temp_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("auth.rs"))
        .stdout(predicate::str::contains("authenticate"));
}

#[test]
fn test_search_command_auto_enables_semantic_flags() {
    // This test verifies that semantic flags are automatically enabled
    // We'll check this by looking for semantic-related output in verbose mode
    let temp_dir = TempDir::new().unwrap();

    std::fs::write(temp_dir.path().join("test.rs"), "use auth::login;").unwrap();

    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg("search")
        .arg("login")
        .arg(temp_dir.path())
        .assert()
        .success();
    // The actual semantic feature is blocked by issue #18
    // For now, we just verify the command runs successfully
}

#[test]
fn test_search_command_no_semantic_flag() {
    let temp_dir = TempDir::new().unwrap();

    std::fs::write(temp_dir.path().join("test.rs"), "login();").unwrap();

    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg("search")
        .arg("login")
        .arg("--no-semantic")
        .arg(temp_dir.path())
        .assert()
        .success();
}
