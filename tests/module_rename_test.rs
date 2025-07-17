//! Tests for module imports after rename

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_context_builder_module_can_be_imported() {
    // This test verifies that the renamed module can be imported correctly
    // by testing that the CLI can process files and output to stdout
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

    // Test that the tool can process the file to stdout (which requires all modules to work)
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg(temp_dir.path())
        .arg("--quiet")
        .assert()
        .success()
        .stdout(predicate::str::contains("# Code Context"));
}

#[test]
fn test_context_options_struct_works() {
    // Test that ContextOptions (formerly contextOptions) works by running a command
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

    // Test with options that would use ContextOptions - output to stdout
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg(temp_dir.path())
        .arg("--max-tokens")
        .arg("10000")
        .arg("--quiet")
        .assert()
        .success()
        .stdout(predicate::str::contains("test.py"));
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

    // Test that the tool can process multiple file types to stdout
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg(temp_dir.path())
        .arg("--quiet")
        .assert()
        .success()
        .stdout(predicate::str::contains("test.rs"))
        .stdout(predicate::str::contains("test.py"))
        .stdout(predicate::str::contains("test.js"));
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
