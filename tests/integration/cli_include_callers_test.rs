//! CLI integration tests for --include-callers functionality
//!
//! These tests verify that the include-callers feature works correctly
//! when invoked through the command-line interface.

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

/// Helper to list files recursively for debugging
fn list_files_recursively(dir: &Path, indent: &str) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                eprintln!("{}{}/", indent, path.file_name().unwrap().to_string_lossy());
                list_files_recursively(&path, &format!("{indent}  "));
            } else {
                eprintln!("{}{}", indent, path.file_name().unwrap().to_string_lossy());
            }
        }
    }
}

#[test]
fn test_cli_include_callers_cross_module() {
    // Test scenario: Function defined in one module, used in another
    // This simulates a common pattern in Rust projects where core functionality
    // is used by other modules

    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create a .git directory to mark this as a project root
    fs::create_dir_all(root.join(".git")).unwrap();

    // Create a function in core module
    create_file(
        root,
        "core/src/commands/rule/rule.rs",
        r#"
//! Rule processing module

/// Calculate a value based on the given rule
pub fn calculate(value: i32, rule: &str) -> i32 {
    match rule {
        "double" => value * 2,
        "square" => value * value,
        "increment" => value + 1,
        _ => value,
    }
}

/// Apply a rule to a list of values
pub fn apply_rule(values: &[i32], rule: &str) -> Vec<i32> {
    values.iter().map(|v| calculate(*v, rule)).collect()
}
"#,
    );

    // Create module file for core
    create_file(
        root,
        "core/src/commands/rule/mod.rs",
        r#"
pub mod rule;

pub use rule::{calculate, apply_rule};
"#,
    );

    // Create another file that uses the calculate function
    // Use a simple direct function call that will be detected
    create_file(
        root,
        "balances/src/account.rs",
        r#"
//! Account balance management

pub struct Account {
    balance: i32,
}

impl Account {
    pub fn new(initial_balance: i32) -> Self {
        Account { balance: initial_balance }
    }
    
    pub fn apply_interest(&mut self, interest_rule: &str) {
        // Direct function call to calculate
        self.balance = calculate(self.balance, interest_rule);
    }
    
    pub fn get_balance(&self) -> i32 {
        self.balance
    }
}

// Test function that directly calls calculate
pub fn test_calculation() {
    let result = calculate(42, "double");
    println!("Result: {}", result);
}
"#,
    );

    // Create a file that doesn't use the calculate function
    create_file(
        root,
        "utils/src/logger.rs",
        r#"
//! Logging utilities

pub fn log_message(msg: &str) {
    println!("[LOG] {}", msg);
}
"#,
    );

    // Run context-creator with --include-callers
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    let output = cmd
        .current_dir(root)
        .args([
            "--include",
            "core/src/commands/rule/*.rs",
            "--include-callers",
            "--enhanced-context", // To get more detailed output
        ])
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Print output for debugging
    eprintln!("STDOUT:\n{stdout}");
    eprintln!("STDERR:\n{stderr}");

    // Also list all files in the temp directory for debugging
    eprintln!("\nFiles in test directory:");
    list_files_recursively(root, "");

    // Assert command succeeded
    assert!(
        output.status.success(),
        "Command failed with exit code: {}",
        output.status
    );

    // The output should contain both files:
    // 1. rule.rs (matched by include pattern)
    assert!(stdout.contains("rule.rs"), "Output should contain rule.rs");

    // 2. account.rs (calls calculate function)
    assert!(
        stdout.contains("account.rs"),
        "Output should contain account.rs as it calls calculate()"
    );

    // Should NOT contain logger.rs (doesn't call any functions from rule.rs)
    assert!(
        !stdout.contains("logger.rs"),
        "Output should not contain logger.rs as it doesn't call functions from rule.rs"
    );
}

#[test]
fn test_cli_include_callers_with_multiple_callers() {
    // Test scenario: Multiple files calling the same function

    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create a .git directory to mark this as a project root
    fs::create_dir_all(root.join(".git")).unwrap();

    // Create a validation module with exported functions
    create_file(
        root,
        "shared/validation.rs",
        r#"
//! Validation utilities

pub fn validate_email(email: &str) -> bool {
    email.contains('@') && email.contains('.')
}

pub fn validate_phone(phone: &str) -> bool {
    phone.len() >= 10 && phone.chars().all(|c| c.is_numeric())
}
"#,
    );

    // Create multiple files that use validation functions
    create_file(
        root,
        "api/user.rs",
        r#"
use crate::shared::validation::validate_email;

pub fn create_user(email: &str, name: &str) -> Result<(), String> {
    if !validate_email(email) {
        return Err("Invalid email".to_string());
    }
    Ok(())
}
"#,
    );

    create_file(
        root,
        "cli/commands.rs",
        r#"
use crate::shared::validation::{validate_email, validate_phone};

pub fn register_command(email: &str, phone: &str) {
    if validate_email(email) && validate_phone(phone) {
        println!("Registration successful");
    }
}
"#,
    );

    create_file(
        root,
        "tests/validation_tests.rs",
        r#"
use crate::shared::validation::validate_email;

#[cfg(test)]
mod tests {
    use crate::shared::validation::validate_phone;
    
    #[test]
    fn test_email_validation() {
        let is_valid = super::validate_email("test@example.com");
        assert!(is_valid);
    }
    
    #[test]
    fn test_phone_validation() {
        let is_valid = validate_phone("1234567890");
        assert!(is_valid);
    }
}

// Direct function call for testing
pub fn check_email(email: &str) -> bool {
    validate_email(email)
}
"#,
    );

    // Run context-creator
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    let output = cmd
        .current_dir(root)
        .args(["--include", "shared/validation.rs", "--include-callers"])
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should include validation.rs (matched by pattern)
    assert!(stdout.contains("validation.rs"));

    // Should include all files that call validation functions
    assert!(stdout.contains("user.rs"));
    assert!(stdout.contains("commands.rs"));
    assert!(stdout.contains("validation_tests.rs"));
}

#[test]
fn test_cli_include_callers_depth_limiting() {
    // Test scenario: Verify that semantic depth is respected

    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create a .git directory to mark this as a project root
    fs::create_dir_all(root.join(".git")).unwrap();

    // Create a chain of function calls
    create_file(
        root,
        "core.rs",
        r#"
pub fn core_function() -> i32 {
    42
}
"#,
    );

    create_file(
        root,
        "middle.rs",
        r#"
use crate::core::core_function;

pub fn middle_function() -> i32 {
    core_function() * 2
}
"#,
    );

    create_file(
        root,
        "outer.rs",
        r#"
use crate::middle::middle_function;

pub fn outer_function() -> i32 {
    middle_function() + 10
}
"#,
    );

    // Run with depth=1 (should only include direct callers)
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    let output = cmd
        .current_dir(root)
        .args([
            "--include",
            "core.rs",
            "--include-callers",
            "--semantic-depth",
            "1",
        ])
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should include core.rs and middle.rs (direct caller)
    assert!(stdout.contains("core.rs"));
    assert!(stdout.contains("middle.rs"));

    // Should NOT include outer.rs (transitive caller at depth 2)
    // Note: Current implementation only finds direct callers regardless of depth
    // This test documents the current behavior
    assert!(!stdout.contains("outer.rs"));
}
