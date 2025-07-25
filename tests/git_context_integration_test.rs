#![cfg(test)]

use context_creator::cli::{Config, OutputFormat};
use context_creator::core::cache::FileCache;
use context_creator::core::context_builder::{generate_digest, ContextOptions};
use context_creator::core::walker::{walk_directory, WalkOptions};
use std::fs;
use std::process::Command;
use std::sync::Arc;
use tempfile::TempDir;

#[test]
fn test_git_context_end_to_end() {
    // Setup a git repository with some history
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path();

    // Initialize git repository
    Command::new("git")
        .arg("init")
        .current_dir(repo_path)
        .status()
        .expect("Failed to init git repo");

    // Configure git
    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(repo_path)
        .status()
        .expect("Failed to configure git user name");

    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(repo_path)
        .status()
        .expect("Failed to configure git user email");

    // Create a Rust file
    let main_rs = repo_path.join("main.rs");
    fs::write(
        &main_rs,
        "fn main() {\n    println!(\"Hello, world!\");\n}\n",
    )
    .unwrap();

    // First commit
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo_path)
        .status()
        .expect("Failed to git add");

    Command::new("git")
        .args(["commit", "-m", "feat: initial commit with hello world"])
        .current_dir(repo_path)
        .status()
        .expect("Failed to create first commit");

    // Update the file
    fs::write(&main_rs, "fn main() {\n    println!(\"Hello, Rust!\");\n    println!(\"Welcome to context-creator\");\n}\n").unwrap();

    // Second commit
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo_path)
        .status()
        .expect("Failed to git add");

    Command::new("git")
        .args(["commit", "-m", "feat: update greeting message"])
        .current_dir(repo_path)
        .status()
        .expect("Failed to create second commit");

    // Now test the context-creator with git context
    let config = Config {
        paths: Some(vec![repo_path.to_path_buf()]),
        git_context: true,
        ..Config::default()
    };

    let walk_options = WalkOptions::from_config(&config).unwrap();
    let files = walk_directory(repo_path, walk_options).unwrap();

    let context_options = ContextOptions::from_config(&config).unwrap();
    let cache = Arc::new(FileCache::new());

    let digest = generate_digest(
        files,
        context_options,
        cache,
        OutputFormat::Markdown,
        repo_path.to_str().unwrap(),
    )
    .unwrap();

    // Verify the output contains git context
    assert!(digest.contains("main.rs"), "Should contain the file name");
    assert!(
        digest.contains("feat: update greeting message"),
        "Should contain recent commit message"
    );
    assert!(digest.contains("Test User"), "Should contain commit author");
    assert!(
        digest.contains("Git history:"),
        "Should contain git history header"
    );
}

#[test]
fn test_git_context_disabled_by_default() {
    // Setup a git repository
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path();

    // Initialize git repository
    Command::new("git")
        .arg("init")
        .current_dir(repo_path)
        .status()
        .expect("Failed to init git repo");

    // Configure git
    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(repo_path)
        .status()
        .expect("Failed to configure git user name");

    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(repo_path)
        .status()
        .expect("Failed to configure git user email");

    // Create and commit a file
    let file_path = repo_path.join("test.py");
    fs::write(&file_path, "def hello():\n    print('Hello')\n").unwrap();

    Command::new("git")
        .args(["add", "."])
        .current_dir(repo_path)
        .status()
        .expect("Failed to git add");

    Command::new("git")
        .args(["commit", "-m", "feat: add hello function"])
        .current_dir(repo_path)
        .status()
        .expect("Failed to create commit");

    // Test without git context (default)
    let config = Config {
        paths: Some(vec![repo_path.to_path_buf()]),
        ..Config::default()
    };

    let walk_options = WalkOptions::from_config(&config).unwrap();
    let files = walk_directory(repo_path, walk_options).unwrap();

    let context_options = ContextOptions::from_config(&config).unwrap();
    let cache = Arc::new(FileCache::new());

    let digest = generate_digest(
        files,
        context_options,
        cache,
        OutputFormat::Markdown,
        repo_path.to_str().unwrap(),
    )
    .unwrap();

    // Verify git context is NOT included when disabled
    assert!(digest.contains("test.py"), "Should contain the file name");
    assert!(
        !digest.contains("feat: add hello function"),
        "Should NOT contain commit message"
    );
    assert!(
        !digest.contains("Git history:"),
        "Should NOT contain git history header"
    );
}

#[test]
fn test_git_context_with_enhanced_context() {
    // Setup a git repository
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path();

    // Initialize git repository
    Command::new("git")
        .arg("init")
        .current_dir(repo_path)
        .status()
        .expect("Failed to init git repo");

    // Configure git
    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(repo_path)
        .status()
        .expect("Failed to configure git user name");

    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(repo_path)
        .status()
        .expect("Failed to configure git user email");

    // Create a file
    let lib_rs = repo_path.join("lib.rs");
    fs::write(
        &lib_rs,
        "pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n",
    )
    .unwrap();

    // Commit
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo_path)
        .status()
        .expect("Failed to git add");

    Command::new("git")
        .args(["commit", "-m", "feat: implement add function"])
        .current_dir(repo_path)
        .status()
        .expect("Failed to create commit");

    // Test with both git context and enhanced context
    let config = Config {
        paths: Some(vec![repo_path.to_path_buf()]),
        git_context: true,
        enhanced_context: true,
        ..Config::default()
    };

    let walk_options = WalkOptions::from_config(&config).unwrap();
    let files = walk_directory(repo_path, walk_options).unwrap();

    let context_options = ContextOptions::from_config(&config).unwrap();
    let cache = Arc::new(FileCache::new());

    let digest = generate_digest(
        files,
        context_options,
        cache,
        OutputFormat::Markdown,
        repo_path.to_str().unwrap(),
    )
    .unwrap();

    // Verify both enhanced context and git context are included
    assert!(digest.contains("lib.rs"), "Should contain the file name");
    assert!(
        digest.contains("Rust"),
        "Should contain file type from enhanced context"
    );
    assert!(
        digest.contains("feat: implement add function"),
        "Should contain commit message"
    );
    assert!(digest.contains("Test User"), "Should contain commit author");
}
