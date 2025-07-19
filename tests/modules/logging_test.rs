//! Tests for structured logging functionality

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

#[test]
fn test_verbose_flag_counting() {
    let temp_dir = TempDir::new().unwrap();

    // Test single -v flag
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg("-v")
        .arg("--output-file")
        .arg(temp_dir.path().join("output.md"))
        .arg(temp_dir.path());

    let output = cmd.output().unwrap();
    assert!(output.status.success());
}

#[test]
fn test_double_verbose_flag() {
    let temp_dir = TempDir::new().unwrap();

    // Test -vv flag
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg("-v")
        .arg("-v")
        .arg("--output-file")
        .arg(temp_dir.path().join("output.md"))
        .arg(temp_dir.path());

    let output = cmd.output().unwrap();
    assert!(output.status.success());
}

#[test]
fn test_log_format_json_parsing() {
    let temp_dir = TempDir::new().unwrap();

    // Test --log-format json
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg("--log-format")
        .arg("json")
        .arg("--output-file")
        .arg(temp_dir.path().join("output.md"))
        .arg(temp_dir.path());

    let output = cmd.output().unwrap();
    assert!(output.status.success());
}

#[test]
fn test_log_format_plain_parsing() {
    let temp_dir = TempDir::new().unwrap();

    // Test --log-format plain (explicit)
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg("--log-format")
        .arg("plain")
        .arg("--output-file")
        .arg(temp_dir.path().join("output.md"))
        .arg(temp_dir.path());

    let output = cmd.output().unwrap();
    assert!(output.status.success());
}

#[test]
fn test_quiet_flag_still_works() {
    let temp_dir = TempDir::new().unwrap();

    // Test -q flag for backward compatibility
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg("-q")
        .arg("--output-file")
        .arg(temp_dir.path().join("output.md"))
        .arg(temp_dir.path());

    let output = cmd.output().unwrap();
    assert!(output.status.success());

    // With quiet flag, stderr should be empty (no logs)
    assert!(output.stderr.is_empty());
}

#[test]
fn test_verbose_and_quiet_mutual_exclusion() {
    let temp_dir = TempDir::new().unwrap();

    // Test that -v and -q are mutually exclusive
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg("-v")
        .arg("-q")
        .arg("--output-file")
        .arg(temp_dir.path().join("output.md"))
        .arg(temp_dir.path());

    cmd.assert().failure().stderr(predicate::str::contains(
        "Cannot use both --verbose (-v) and --quiet (-q) flags together",
    ));
}

#[test]
fn test_log_format_json_and_verbose_combination() {
    let temp_dir = TempDir::new().unwrap();

    // Test that --log-format json and -v can be used together
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg("--log-format")
        .arg("json")
        .arg("-v")
        .arg("--output-file")
        .arg(temp_dir.path().join("output.md"))
        .arg(temp_dir.path());

    let output = cmd.output().unwrap();
    assert!(output.status.success());
}

#[test]
fn test_rust_log_env_variable() {
    let temp_dir = TempDir::new().unwrap();

    // Test RUST_LOG environment variable
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.env("RUST_LOG", "context_creator=debug")
        .arg("--output-file")
        .arg(temp_dir.path().join("output.md"))
        .arg(temp_dir.path());

    let output = cmd.output().unwrap();
    assert!(output.status.success());
}

#[test]
fn test_default_behavior_unchanged() {
    let temp_dir = TempDir::new().unwrap();

    // Test default behavior without any logging flags
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg("--output-file")
        .arg(temp_dir.path().join("output.md"))
        .arg(temp_dir.path());

    let output = cmd.output().unwrap();
    assert!(output.status.success());

    // Default should not show debug logs
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("DEBUG"));
    assert!(!stderr.contains("TRACE"));
}
