//! Tests for search command respecting .gitignore and hidden files

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

/// Helper to create a test project with gitignore
fn create_project_with_gitignore() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();

    // Create directory structure
    fs::create_dir_all(base.join("src")).unwrap();
    fs::create_dir_all(base.join("build")).unwrap();
    fs::create_dir_all(base.join("node_modules")).unwrap();
    fs::create_dir_all(base.join(".git")).unwrap();
    fs::create_dir_all(base.join(".hidden")).unwrap();

    // Create .gitignore
    fs::write(
        base.join(".gitignore"),
        "build/
node_modules/
*.log
temp_*
",
    )
    .unwrap();

    // Create files that should be found
    fs::write(
        base.join("src/main.rs"),
        "fn main() { println!(\"test_pattern\"); }",
    )
    .unwrap();

    fs::write(
        base.join("src/lib.rs"),
        "pub fn test_pattern() -> String { \"test\".to_string() }",
    )
    .unwrap();

    // Create files that should be ignored
    fs::write(
        base.join("build/output.txt"),
        "test_pattern in build directory",
    )
    .unwrap();

    fs::write(
        base.join("node_modules/package.js"),
        "console.log('test_pattern in node_modules');",
    )
    .unwrap();

    fs::write(base.join("debug.log"), "test_pattern in log file").unwrap();

    fs::write(base.join("temp_file.txt"), "test_pattern in temp file").unwrap();

    // Create files in hidden directories
    fs::write(base.join(".git/config"), "[core] test_pattern = true").unwrap();

    fs::write(
        base.join(".hidden/secret.txt"),
        "test_pattern in hidden directory",
    )
    .unwrap();

    temp_dir
}

#[test]
fn test_search_respects_gitignore() {
    let project = create_project_with_gitignore();

    Command::cargo_bin("context-creator")
        .unwrap()
        .arg("search")
        .arg("test_pattern")
        .arg(project.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("main.rs"))
        .stdout(predicate::str::contains("lib.rs"))
        // Should NOT contain ignored files
        .stdout(predicate::str::contains("build").not())
        .stdout(predicate::str::contains("node_modules").not())
        .stdout(predicate::str::contains(".log").not())
        .stdout(predicate::str::contains("temp_").not());
}

#[test]
fn test_search_excludes_hidden_directories() {
    let project = create_project_with_gitignore();

    Command::cargo_bin("context-creator")
        .unwrap()
        .arg("search")
        .arg("test_pattern")
        .arg(project.path())
        .assert()
        .success()
        // Should NOT contain hidden directories
        .stdout(predicate::str::contains(".git").not())
        .stdout(predicate::str::contains(".hidden").not());
}

#[test]
fn test_search_with_paths_style_respects_gitignore() {
    let project = create_project_with_gitignore();

    Command::cargo_bin("context-creator")
        .unwrap()
        .arg("--style")
        .arg("paths")
        .arg("search")
        .arg("test_pattern")
        .arg(project.path())
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"src[/\\]main\.rs").unwrap())
        .stdout(predicate::str::is_match(r"src[/\\]lib\.rs").unwrap())
        // Ensure paths output doesn't contain ignored paths
        .stdout(predicate::str::contains("build").not())
        .stdout(predicate::str::contains("node_modules").not())
        .stdout(predicate::str::contains(".git").not())
        .stdout(predicate::str::contains(".log").not());
}

#[test]
fn test_search_respects_ignore_file() {
    let project = create_project_with_gitignore();

    // Create a .ignore file (which is respected by the ignore crate)
    fs::write(project.path().join(".ignore"), "src/test.rs").unwrap();

    // Also create a file that would normally be included
    fs::write(project.path().join("src/test.rs"), "fn test_pattern() {}").unwrap();

    Command::cargo_bin("context-creator")
        .unwrap()
        .arg("search")
        .arg("test_pattern")
        .arg(project.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("main.rs"))
        .stdout(predicate::str::contains("lib.rs"))
        // Should not contain the file excluded by .ignore
        .stdout(predicate::str::contains("test.rs").not());
}
