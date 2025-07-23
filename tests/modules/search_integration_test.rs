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
    let temp_dir = TempDir::new().unwrap();

    // Create a file that imports a module
    std::fs::write(
        temp_dir.path().join("main.rs"),
        r#"
use auth::login;

fn main() {
    login("user", "pass");
}
"#,
    )
    .unwrap();

    // Create the imported module
    std::fs::write(
        temp_dir.path().join("auth.rs"),
        r#"
pub fn login(username: &str, password: &str) -> bool {
    // login logic
    true
}
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg("search")
        .arg("login")
        .arg(temp_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("main.rs")) // File with import
        .stdout(predicate::str::contains("auth.rs")) // File with function
        .stdout(predicate::str::contains("Function calls: login")) // Semantic metadata
        .stdout(predicate::str::contains("Type references: login")); // Semantic relationships
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
