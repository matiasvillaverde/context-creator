//! Tests for the search command functionality

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_search_command_basic() {
    // Test that search command is recognized
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg("search").arg("AuthService").assert().success();
}

#[test]
fn test_search_command_help() {
    // Test that search command has help text
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg("search")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Search for files containing the specified term",
        ));
}

#[test]
fn test_search_command_no_semantic() {
    // Test that --no-semantic flag is recognized
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg("search")
        .arg("test")
        .arg("--no-semantic")
        .assert()
        .success();
}
