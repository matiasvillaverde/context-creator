//! Simple CLI integration test for --include-callers functionality
//!
//! This test verifies the basic functionality works with a simple setup

use assert_cmd::Command;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// Helper to create a file in the test directory
fn create_file(base: &Path, path: &str, content: &str) {
    let file_path = base.join(path);
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(file_path, content).unwrap();
}

#[test]
fn test_cli_include_callers_simple() {
    // Test the simplest case: all Rust files in the same directory
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create a .git directory to mark this as a project root
    fs::create_dir_all(root.join(".git")).unwrap();

    // Create a simple module with an exported function
    create_file(
        root,
        "math.rs",
        r#"
/// Calculate a value
pub fn calculate(a: i32, b: i32) -> i32 {
    a + b
}
"#,
    );

    // Create a file that uses the function
    create_file(
        root,
        "main.rs",
        r#"
mod math;

fn main() {
    let result = math::calculate(5, 3);
    println!("Result: {}", result);
}
"#,
    );

    // Create a file that doesn't use the function
    create_file(
        root,
        "other.rs",
        r#"
fn other_function() {
    println!("Other work");
}
"#,
    );

    // Run context-creator with --include-callers
    // Use *.rs to include all Rust files (this is important!)
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    let output = cmd
        .current_dir(root)
        .args([
            "--include",
            "math.rs",           // Start with math.rs
            "--include-callers", // Find files that call functions from math.rs
        ])
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Print output for debugging
    eprintln!("STDOUT:\n{stdout}");
    eprintln!("STDERR:\n{stderr}");

    // Assert command succeeded
    assert!(
        output.status.success(),
        "Command failed with exit code: {}",
        output.status
    );

    // The output should contain:
    // 1. math.rs (matched by include pattern)
    assert!(stdout.contains("math.rs"), "Output should contain math.rs");

    // 2. main.rs (calls calculate function)
    assert!(
        stdout.contains("main.rs"),
        "Output should contain main.rs as it calls calculate()"
    );

    // Should NOT contain other.rs (doesn't call any functions from math.rs)
    assert!(
        !stdout.contains("other.rs"),
        "Output should not contain other.rs as it doesn't call functions from math.rs"
    );
}

#[test]
fn test_cli_include_callers_with_glob() {
    // Test with glob pattern to start with multiple files
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create a .git directory
    fs::create_dir_all(root.join(".git")).unwrap();

    // Create core modules
    create_file(
        root,
        "core/math.rs",
        r#"
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

pub fn multiply(a: i32, b: i32) -> i32 {
    a * b
}
"#,
    );

    create_file(
        root,
        "core/utils.rs",
        r#"
pub fn format_number(n: i32) -> String {
    format!("Number: {}", n)
}
"#,
    );

    // Create files that use core functions
    create_file(
        root,
        "app.rs",
        r#"
use crate::core::math::{add, multiply};
use crate::core::utils::format_number;

fn calculate() {
    let sum = add(5, 3);
    let product = multiply(4, 2);
    let formatted = format_number(sum);
    println!("{}", formatted);
}
"#,
    );

    create_file(
        root,
        "tests.rs",
        r#"
use crate::core::math::add;

#[cfg(test)]
mod tests {
    use crate::core::math::multiply;
    
    #[test]
    fn test_add() {
        let result = super::add(2, 2);
        assert_eq!(result, 4);
    }
    
    #[test] 
    fn test_multiply() {
        let result = multiply(3, 4);
        assert_eq!(result, 12);
    }
}

// Direct function call outside of test module
pub fn calculate_sum() -> i32 {
    add(10, 20)
}
"#,
    );

    // Run with glob pattern
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    let output = cmd
        .current_dir(root)
        .args([
            "--include",
            "core/*.rs",         // Include all files in core/
            "--include-callers", // Find their callers
        ])
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Debug output
    eprintln!("STDOUT:\n{stdout}");
    eprintln!("STDERR:\n{stderr}");

    // Should include core files
    assert!(stdout.contains("math.rs"));
    assert!(stdout.contains("utils.rs"));

    // Should include files that call core functions
    assert!(stdout.contains("app.rs"));
    assert!(stdout.contains("tests.rs"));
}
