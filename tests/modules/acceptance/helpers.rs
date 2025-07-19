//! Common helper functions for acceptance tests

#![allow(dead_code)] // Some helpers will be used in later test phases
#![allow(clippy::uninlined_format_args)] // Keep traditional format! style

use assert_cmd::Command;
use std::path::Path;

/// Helper to create a command for the context-creator binary
pub fn context_creator_cmd() -> Command {
    Command::cargo_bin("context-creator").unwrap()
}

/// Helper to check if content contains a file path, handling both Unix and Windows separators
pub fn assert_contains_file(output: &str, file_path: &str) {
    let unix_path = file_path.replace('\\', "/");
    let windows_path = file_path.replace('/', "\\");

    // Also check for just the filename without directory prefix
    let filename = std::path::Path::new(file_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(file_path);

    // Check if the file appears in a header format (## filename)
    let header_check = output.contains(&format!("## {filename}"));

    assert!(
        output.contains(&unix_path) || output.contains(&windows_path) || header_check,
        "Expected output to contain file '{}', but it didn't.\nOutput:\n{}",
        file_path,
        output
    );
}

/// Helper to assert that content does NOT contain a file path
pub fn assert_not_contains_file(output: &str, file_path: &str) {
    let unix_path = file_path.replace('\\', "/");
    let windows_path = file_path.replace('/', "\\");

    // Also check for just the filename without directory prefix
    let filename = std::path::Path::new(file_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(file_path);

    // Check if the file appears in a header format (## filename)
    let header_check = output.contains(&format!("## {filename}"));

    assert!(
        !output.contains(&unix_path) && !output.contains(&windows_path) && !header_check,
        "Expected output NOT to contain file '{}', but it did.\nOutput:\n{}",
        file_path,
        output
    );
}

/// Helper to assert that output contains a specific function or class
pub fn assert_contains_code(output: &str, code_snippet: &str) {
    assert!(
        output.contains(code_snippet),
        "Expected output to contain code snippet '{}', but it didn't.\nOutput:\n{}",
        code_snippet,
        output
    );
}

/// Helper to assert markdown structure contains file header
pub fn assert_contains_file_header(output: &str, file_name: &str) {
    // Check for common markdown file header patterns
    let patterns = [
        format!("## {file_name}"),
        format!("### {file_name}"),
        format!("# {file_name}"),
        format!("File: {file_name}"),
    ];

    let found = patterns.iter().any(|pattern| output.contains(pattern));

    assert!(
        found,
        "Expected output to contain header for file '{}', but it didn't.\nOutput:\n{}",
        file_name, output
    );
}

/// Helper to run context-creator with specific arguments and get output
pub fn run_context_creator(args: &[&str], project_dir: &Path) -> String {
    let mut cmd = context_creator_cmd();

    // Change to project directory for relative path testing
    cmd.current_dir(project_dir);

    // Create a temporary output file
    let output_file = project_dir.join("test_output.md");

    // Add output file argument if not already present
    let mut has_output = false;
    let mut has_prompt = false;
    for arg in args {
        if *arg == "--output-file" || *arg == "-o" {
            has_output = true;
        }
        if *arg == "--prompt" || *arg == "-p" {
            has_prompt = true;
        }
    }

    // Add arguments
    for arg in args {
        cmd.arg(arg);
    }

    // If no output file or prompt specified, add output file
    if !has_output && !has_prompt {
        cmd.arg("--output-file").arg(&output_file);
    }

    // Run and capture output
    let output = cmd.output().expect("Failed to execute context-creator");

    // Check if command was successful
    assert!(
        output.status.success(),
        "context-creator failed with status: {}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );

    // If we used an output file, read its contents
    if !has_output && !has_prompt && output_file.exists() {
        let content = std::fs::read_to_string(&output_file).expect("Failed to read output file");
        // Clean up
        let _ = std::fs::remove_file(&output_file);
        content
    } else {
        String::from_utf8_lossy(&output.stdout).to_string()
    }
}

/// Helper to run context-creator and expect it to fail
pub fn run_context_creator_expect_failure(args: &[&str], project_dir: &Path) -> String {
    let mut cmd = context_creator_cmd();

    cmd.current_dir(project_dir);

    for arg in args {
        cmd.arg(arg);
    }

    let output = cmd.output().expect("Failed to execute context-creator");

    assert!(
        !output.status.success(),
        "Expected context-creator to fail, but it succeeded.\nstdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );

    String::from_utf8_lossy(&output.stderr).to_string()
}

/// Helper to count occurrences of a pattern in output
pub fn count_occurrences(output: &str, pattern: &str) -> usize {
    output.matches(pattern).count()
}

/// Helper to extract file content from markdown output
pub fn extract_file_content(output: &str, file_name: &str) -> Option<String> {
    // Look for the file header and extract content until next file or end
    let file_header = format!("## {file_name}");

    if let Some(start_idx) = output.find(&file_header) {
        let content_start = start_idx + file_header.len();

        // Find the next file header or end of string
        let content = if let Some(next_file_idx) = output[content_start..].find("## ") {
            &output[content_start..content_start + next_file_idx]
        } else {
            &output[content_start..]
        };

        Some(content.trim().to_string())
    } else {
        None
    }
}
