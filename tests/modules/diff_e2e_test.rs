#![cfg(test)]

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::process::Command as StdCommand;
use tempfile::TempDir;

/// Helper function to set up a git repository with commits for E2E testing
fn setup_git_repo_for_e2e() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path();

    // Initialize git repository
    StdCommand::new("git")
        .arg("init")
        .current_dir(repo_path)
        .status()
        .expect("Failed to init git repo");

    // Configure git for testing
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

    // Create some realistic source files for first commit
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
        r#"pub mod utils;

pub fn process_data(input: &str) -> String {
    format!("processed: {}", input)
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

    // Create second commit with changes
    fs::write(
        repo_path.join("main.rs"),
        r#"use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    println!("Hello from args: {:?}", args);
}
"#,
    )
    .unwrap();

    fs::write(
        repo_path.join("utils.rs"),
        r#"pub fn helper_function() -> i32 {
    42
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
        .args(["commit", "-m", "Add utils and modify main"])
        .current_dir(repo_path)
        .status()
        .expect("Failed to create second commit");

    temp_dir
}

/// Test E2E: Happy path - diff command with valid git references
#[test]
fn test_diff_command_e2e_happy_path() {
    let repo = setup_git_repo_for_e2e();

    // This test will fail until we implement the diff command logic
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg("diff")
        .arg("HEAD~1")
        .arg("HEAD")
        .current_dir(repo.path());

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("# Git Diff Analysis"))
        .stdout(predicate::str::contains("From: HEAD~1"))
        .stdout(predicate::str::contains("To: HEAD"))
        .stdout(predicate::str::contains("Files changed:"))
        .stdout(predicate::str::contains("main.rs"))
        .stdout(predicate::str::contains("utils.rs"));
}

/// Test E2E: diff command output includes changed file content
#[test]
fn test_diff_command_e2e_includes_content() {
    let repo = setup_git_repo_for_e2e();

    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg("diff")
        .arg("HEAD~1")
        .arg("HEAD")
        .current_dir(repo.path());

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("# main.rs"))
        .stdout(predicate::str::contains("# utils.rs"))
        .stdout(predicate::str::contains("use std::env"))
        .stdout(predicate::str::contains("helper_function"));
}

/// Test E2E: diff command with invalid git reference
#[test]
fn test_diff_command_e2e_invalid_ref() {
    let repo = setup_git_repo_for_e2e();

    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg("diff")
        .arg("invalid-ref-12345")
        .arg("HEAD")
        .current_dir(repo.path());

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Git command failed"));
}

/// Test E2E: diff command in non-git directory
#[test]
fn test_diff_command_e2e_not_git_repo() {
    let temp_dir = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg("diff")
        .arg("HEAD~1")
        .arg("HEAD")
        .current_dir(temp_dir.path());

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Not a git repository"));
}

/// Test E2E: diff command with identical references (no changes)
#[test]
fn test_diff_command_e2e_no_changes() {
    let repo = setup_git_repo_for_e2e();

    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg("diff")
        .arg("HEAD")
        .arg("HEAD")
        .current_dir(repo.path());

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("# Git Diff Analysis"))
        .stdout(predicate::str::contains("Files changed: 0"));
}

/// Test E2E: diff command with global flags
#[test]
fn test_diff_command_e2e_with_global_flags() {
    let repo = setup_git_repo_for_e2e();

    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg("--max-tokens")
        .arg("1000")
        .arg("diff")
        .arg("HEAD~1")
        .arg("HEAD")
        .current_dir(repo.path());

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("# Git Diff Analysis"));
}
