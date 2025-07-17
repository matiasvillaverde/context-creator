//! Tests for binary name and version output after rename

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_binary_name_is_context_creator() {
    // Test that the binary builds with the correct name
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("context-creator"));
}

#[test]
fn test_version_output_contains_new_name() {
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("context-creator"))
        .stdout(predicate::str::contains("1.0.0"));
}

#[test]
fn test_help_output_contains_new_name() {
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("context-creator"))
        .stdout(predicate::str::contains(
            "High-performance CLI tool to convert codebases to Markdown for LLM context",
        ));
}

#[test]
fn test_old_binary_name_no_longer_exists() {
    // This should fail because code-digest binary shouldn't exist anymore
    let result = Command::cargo_bin("code-digest");
    assert!(
        result.is_err(),
        "Old binary name 'code-digest' should not exist"
    );
}
