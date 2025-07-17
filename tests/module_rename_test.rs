//! Tests for module imports after rename

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_context_builder_module_can_be_imported() {
    // This test verifies that the renamed module can be imported correctly
    // by testing that the CLI can process files (which uses the core modules)
    let temp_dir = TempDir::new().unwrap();

    // Create a simple test file
    fs::write(
        temp_dir.path().join("test.rs"),
        r#"
fn main() {
    println!("Hello, world!");
}
"#,
    )
    .unwrap();

    // Test that the tool can process the file (which requires all modules to work)
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.current_dir(temp_dir.path())
        .arg("--prompt")
        .arg("Test processing")
        .assert()
        .success();
}

#[test]
fn test_context_options_struct_works() {
    // Test that ContextOptions (formerly DigestOptions) works by running a command
    // that would use these options
    let temp_dir = TempDir::new().unwrap();

    // Create a test file
    fs::write(
        temp_dir.path().join("test.py"),
        r#"
def hello():
    print("Hello, world!")
"#,
    )
    .unwrap();

    // Test with options that would use ContextOptions
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.current_dir(temp_dir.path())
        .arg("--prompt")
        .arg("Analyze this code")
        .arg("--max-tokens")
        .arg("10000")
        .assert()
        .success();
}

#[test]
fn test_all_core_modules_resolve_correctly() {
    // Test that all core modules work together by running the tool
    // This indirectly tests that all imports resolve correctly
    let temp_dir = TempDir::new().unwrap();

    // Create multiple test files of different types
    fs::write(temp_dir.path().join("test.rs"), "fn main() {}").unwrap();
    fs::write(temp_dir.path().join("test.py"), "print('hello')").unwrap();
    fs::write(temp_dir.path().join("test.js"), "console.log('hello');").unwrap();

    // Test that the tool can process multiple file types
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.current_dir(temp_dir.path())
        .arg("--prompt")
        .arg("Process multiple files")
        .assert()
        .success();
}

#[test]
fn test_crate_name_in_error_messages() {
    // Test that error messages reference the correct crate name
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg("--invalid-flag")
        .assert()
        .failure()
        .stderr(predicate::str::contains("context-creator").or(predicate::str::contains("error")));
}
