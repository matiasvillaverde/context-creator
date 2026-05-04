#![cfg(test)]

use assert_cmd::Command;
use predicates::prelude::*;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use tempfile::TempDir;

fn setup_project() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    fs::write(
        temp_dir.path().join("main.rs"),
        r#"fn main() {
    println!("hello from the mock tool e2e project");
}
"#,
    )
    .unwrap();
    temp_dir
}

fn setup_mock_tools() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    for tool in ["gemini", "codex", "claude", "ollama"] {
        write_mock_tool(temp_dir.path(), tool);
    }
    temp_dir
}

fn write_mock_tool(bin_dir: &Path, tool: &str) {
    let script_path = bin_dir.join(tool);
    let script = format!(
        r#"#!/bin/sh
set -eu
log_dir="${{CONTEXT_CREATOR_MOCK_LOG_DIR:?missing log dir}}"
printf '%s\n' "$@" > "$log_dir/{tool}.args"
cat > "$log_dir/{tool}.stdin"
printf '%s\n' "{tool} ok"
"#
    );
    fs::write(&script_path, script).unwrap();
    let mut permissions = fs::metadata(&script_path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&script_path, permissions).unwrap();
}

fn path_with_mock_tools(mock_dir: &Path) -> OsString {
    let current_path = env::var_os("PATH").unwrap_or_default();
    env::join_paths(
        std::iter::once(mock_dir.as_os_str().to_os_string())
            .chain(env::split_paths(&current_path).map(|path| path.into_os_string())),
    )
    .unwrap()
}

#[test]
fn test_prompt_executes_gemini_with_combined_prompt_and_context() {
    let project = setup_project();
    let mock_tools = setup_mock_tools();
    let log_dir = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.current_dir(project.path())
        .env("PATH", path_with_mock_tools(mock_tools.path()))
        .env("CONTEXT_CREATOR_MOCK_LOG_DIR", log_dir.path())
        .args(["--prompt", "Explain the project", "--tool", "gemini", "."]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("gemini ok"));

    let stdin = fs::read_to_string(log_dir.path().join("gemini.stdin")).unwrap();
    assert!(stdin.contains("Explain the project"));
    assert!(stdin.contains("hello from the mock tool e2e project"));

    let args = fs::read_to_string(log_dir.path().join("gemini.args")).unwrap();
    assert!(
        args.trim().is_empty(),
        "gemini should not receive argv: {args}"
    );
}

#[test]
fn test_prompt_executes_codex_with_combined_prompt_and_context() {
    let project = setup_project();
    let mock_tools = setup_mock_tools();
    let log_dir = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.current_dir(project.path())
        .env("PATH", path_with_mock_tools(mock_tools.path()))
        .env("CONTEXT_CREATOR_MOCK_LOG_DIR", log_dir.path())
        .args(["--prompt", "Review the project", "--tool", "codex", "."]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("codex ok"));

    let stdin = fs::read_to_string(log_dir.path().join("codex.stdin")).unwrap();
    assert!(stdin.contains("Review the project"));
    assert!(stdin.contains("hello from the mock tool e2e project"));

    let args = fs::read_to_string(log_dir.path().join("codex.args")).unwrap();
    assert!(
        args.trim().is_empty(),
        "codex should not receive argv: {args}"
    );
}

#[test]
fn test_prompt_executes_claude_with_prompt_arg_and_context_stdin() {
    let project = setup_project();
    let mock_tools = setup_mock_tools();
    let log_dir = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.current_dir(project.path())
        .env("PATH", path_with_mock_tools(mock_tools.path()))
        .env("CONTEXT_CREATOR_MOCK_LOG_DIR", log_dir.path())
        .args(["--prompt", "Audit the project", "--tool", "claude", "."]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("claude ok"));

    let args = fs::read_to_string(log_dir.path().join("claude.args")).unwrap();
    assert!(args.contains("-p"));
    assert!(args.contains("Audit the project"));

    let stdin = fs::read_to_string(log_dir.path().join("claude.stdin")).unwrap();
    assert!(!stdin.contains("Audit the project"));
    assert!(stdin.contains("hello from the mock tool e2e project"));
}

#[test]
fn test_prompt_executes_ollama_with_model_and_combined_input() {
    let project = setup_project();
    let mock_tools = setup_mock_tools();
    let log_dir = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.current_dir(project.path())
        .env("PATH", path_with_mock_tools(mock_tools.path()))
        .env("CONTEXT_CREATOR_MOCK_LOG_DIR", log_dir.path())
        .args([
            "--prompt",
            "Summarize the project",
            "--tool",
            "ollama",
            "--ollama-model",
            "llama3.1",
            ".",
        ]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("ollama ok"));

    let args = fs::read_to_string(log_dir.path().join("ollama.args")).unwrap();
    assert!(args.contains("run"));
    assert!(args.contains("llama3.1"));

    let stdin = fs::read_to_string(log_dir.path().join("ollama.stdin")).unwrap();
    assert!(stdin.contains("Summarize the project"));
    assert!(stdin.contains("hello from the mock tool e2e project"));
}
