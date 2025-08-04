//! Tests for telemetry command CLI argument parsing

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

#[test]
fn test_telemetry_command_requires_telemetry_file() {
    // Given: No telemetry file argument
    let mut cmd = Command::cargo_bin("context-creator").unwrap();

    // When: Running telemetry command without required argument
    let assert = cmd.arg("telemetry").assert();

    // Then: Should fail with error message
    assert
        .failure()
        .stderr(predicate::str::contains(
            "required arguments were not provided",
        ))
        .stderr(predicate::str::contains("--telemetry-file"));
}

#[test]
fn test_telemetry_command_with_valid_file() {
    // Given: A valid telemetry file
    let temp_dir = TempDir::new().unwrap();
    let telemetry_file = temp_dir.path().join("traces.json");
    std::fs::write(&telemetry_file, r#"{"traces": []}"#).unwrap();

    let mut cmd = Command::cargo_bin("context-creator").unwrap();

    // When: Running telemetry command with valid file
    let assert = cmd
        .arg("telemetry")
        .arg("--telemetry-file")
        .arg(&telemetry_file)
        .assert();

    // Then: Should run successfully
    assert
        .success()
        .stdout(predicate::str::contains("Total spans: 0"));
}

#[test]
fn test_telemetry_command_with_time_range() {
    // Given: Telemetry file and time range
    let temp_dir = TempDir::new().unwrap();
    let telemetry_file = temp_dir.path().join("traces.json");
    std::fs::write(&telemetry_file, r#"{"traces": []}"#).unwrap();

    let mut cmd = Command::cargo_bin("context-creator").unwrap();

    // When: Running with time range filter
    let assert = cmd
        .arg("telemetry")
        .arg("--telemetry-file")
        .arg(&telemetry_file)
        .arg("--time-range")
        .arg("2024-01-01T00:00:00Z/2024-01-02T00:00:00Z")
        .assert();

    // Then: Should accept the time range
    assert
        .success()
        .stdout(predicate::str::contains("Total spans: 0"));
}

#[test]
fn test_telemetry_command_with_service_filter() {
    // Given: Telemetry file and service name
    let temp_dir = TempDir::new().unwrap();
    let telemetry_file = temp_dir.path().join("traces.json");
    std::fs::write(&telemetry_file, r#"{"traces": []}"#).unwrap();

    let mut cmd = Command::cargo_bin("context-creator").unwrap();

    // When: Running with service filter
    let assert = cmd
        .arg("telemetry")
        .arg("--telemetry-file")
        .arg(&telemetry_file)
        .arg("--service")
        .arg("payment-api")
        .assert();

    // Then: Should accept the service filter
    assert
        .success()
        .stdout(predicate::str::contains("Total spans: 0"));
}

#[test]
fn test_telemetry_command_with_custom_paths() {
    // Given: Telemetry file and custom paths
    let temp_dir = TempDir::new().unwrap();
    let telemetry_file = temp_dir.path().join("traces.json");
    std::fs::write(&telemetry_file, r#"{"traces": []}"#).unwrap();

    let source_dir = temp_dir.path().join("src");
    std::fs::create_dir(&source_dir).unwrap();

    let mut cmd = Command::cargo_bin("context-creator").unwrap();

    // When: Running with custom paths to analyze
    let assert = cmd
        .arg("telemetry")
        .arg("--telemetry-file")
        .arg(&telemetry_file)
        .arg(&source_dir)
        .assert();

    // Then: Should accept paths argument
    assert
        .success()
        .stdout(predicate::str::contains("Total spans: 0"));
}

#[test]
fn test_telemetry_command_validates_file_exists() {
    // Given: Non-existent telemetry file
    let mut cmd = Command::cargo_bin("context-creator").unwrap();

    // When: Running with non-existent file
    let assert = cmd
        .arg("telemetry")
        .arg("--telemetry-file")
        .arg("/non/existent/file.json")
        .assert();

    // Then: Should fail with clear error
    assert
        .failure()
        .stderr(predicate::str::contains("Telemetry file does not exist"));
}

#[test]
fn test_telemetry_command_short_flag() {
    // Given: Using short flag -t
    let temp_dir = TempDir::new().unwrap();
    let telemetry_file = temp_dir.path().join("traces.json");
    std::fs::write(&telemetry_file, r#"{"traces": []}"#).unwrap();

    let mut cmd = Command::cargo_bin("context-creator").unwrap();

    // When: Using -t instead of --telemetry-file
    let assert = cmd.arg("telemetry").arg("-t").arg(&telemetry_file).assert();

    // Then: Should work the same
    assert
        .success()
        .stdout(predicate::str::contains("Total spans: 0"));
}
