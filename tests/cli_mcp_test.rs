//! Tests for MCP server CLI integration

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_mcp_flag_help() {
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("--mcp"))
        .stdout(predicate::str::contains("Start MCP server mode"));
}

#[test]
fn test_mcp_flag_with_default_port() {
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg("--mcp");
    cmd.timeout(std::time::Duration::from_secs(1));

    // Server should start but we'll kill it after 1 second
    let output = cmd.output().unwrap();

    // Should contain server startup message
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("MCP server listening on") || stderr.contains("127.0.0.1:9090"),
        "Expected server startup message, got: {stderr}"
    );
}

#[test]
fn test_mcp_flag_with_custom_port() {
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.args(["--mcp", "--mcp-port", "8888"]);
    cmd.timeout(std::time::Duration::from_secs(1));

    let output = cmd.output().unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("8888"),
        "Expected custom port 8888 in output, got: {stderr}"
    );
}

#[test]
fn test_mcp_flag_conflicts_with_paths() {
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.args(["--mcp", "some/path"]);

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}
