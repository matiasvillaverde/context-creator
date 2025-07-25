#![cfg(test)]

use context_creator::core::cache::FileCache;
use context_creator::core::context_builder::{generate_markdown, ContextOptions};
use context_creator::core::walker::FileInfo;
use context_creator::utils::file_ext::FileType;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use tempfile::TempDir;

/// Helper to setup a git repo with file history
fn setup_test_repo() -> TempDir {
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
    let file_path = repo_path.join("example.rs");
    fs::write(&file_path, "fn main() {}\n").unwrap();

    Command::new("git")
        .args(["add", "."])
        .current_dir(repo_path)
        .status()
        .expect("Failed to git add");

    Command::new("git")
        .args(["commit", "-m", "feat: initial implementation"])
        .current_dir(repo_path)
        .status()
        .expect("Failed to create first commit");

    // Make another commit
    fs::write(&file_path, "fn main() {\n    println!(\"Hello\");\n}\n").unwrap();

    Command::new("git")
        .args(["add", "."])
        .current_dir(repo_path)
        .status()
        .expect("Failed to git add");

    Command::new("git")
        .args(["commit", "-m", "feat: add hello message"])
        .current_dir(repo_path)
        .status()
        .expect("Failed to create second commit");

    temp_dir
}

#[test]
fn test_markdown_with_git_context_enabled() {
    let repo = setup_test_repo();
    let file_path = repo.path().join("example.rs");

    let file_info = FileInfo {
        path: file_path.clone(),
        relative_path: PathBuf::from("example.rs"),
        size: 100,
        file_type: FileType::Rust,
        priority: 1.0,
        imports: vec![],
        imported_by: vec![],
        function_calls: vec![],
        type_references: vec![],
        exported_functions: vec![],
    };

    let options = ContextOptions {
        max_tokens: None,
        include_tree: false,
        include_stats: false,
        group_by_type: false,
        sort_by_priority: false,
        file_header_template: "## {path}".to_string(),
        doc_header_template: "".to_string(),
        include_toc: false,
        enhanced_context: false,
        git_context: true,
        git_context_depth: 3,
    };

    let cache = Arc::new(FileCache::new());
    let markdown = generate_markdown(vec![file_info], options, cache).unwrap();

    // Should include git commit information in the file header
    assert!(markdown.contains("## example.rs"));
    assert!(
        markdown.contains("feat: add hello message")
            || markdown.contains("feat: initial implementation"),
        "Markdown should contain git commit messages"
    );
    assert!(
        markdown.contains("Test User"),
        "Markdown should contain commit author"
    );
}

#[test]
fn test_markdown_without_git_context() {
    let repo = setup_test_repo();
    let file_path = repo.path().join("example.rs");

    let file_info = FileInfo {
        path: file_path.clone(),
        relative_path: PathBuf::from("example.rs"),
        size: 100,
        file_type: FileType::Rust,
        priority: 1.0,
        imports: vec![],
        imported_by: vec![],
        function_calls: vec![],
        type_references: vec![],
        exported_functions: vec![],
    };

    let options = ContextOptions {
        max_tokens: None,
        include_tree: false,
        include_stats: false,
        group_by_type: false,
        sort_by_priority: false,
        file_header_template: "## {path}".to_string(),
        doc_header_template: "".to_string(),
        include_toc: false,
        enhanced_context: false,
        git_context: false,
        git_context_depth: 3,
    };

    let cache = Arc::new(FileCache::new());
    let markdown = generate_markdown(vec![file_info], options, cache).unwrap();

    // Should NOT include git commit information
    assert!(markdown.contains("## example.rs"));
    assert!(
        !markdown.contains("feat: add hello message"),
        "Markdown should not contain commit messages when git_context is false"
    );
    assert!(
        !markdown.contains("Test User"),
        "Markdown should not contain author when git_context is false"
    );
}

#[test]
fn test_markdown_with_git_context_and_enhanced_context() {
    let repo = setup_test_repo();
    let file_path = repo.path().join("example.rs");

    let file_info = FileInfo {
        path: file_path.clone(),
        relative_path: PathBuf::from("example.rs"),
        size: 100,
        file_type: FileType::Rust,
        priority: 1.0,
        imports: vec![],
        imported_by: vec![],
        function_calls: vec![],
        type_references: vec![],
        exported_functions: vec![],
    };

    let options = ContextOptions {
        max_tokens: None,
        include_tree: false,
        include_stats: false,
        group_by_type: false,
        sort_by_priority: false,
        file_header_template: "## {path}".to_string(),
        doc_header_template: "".to_string(),
        include_toc: false,
        enhanced_context: true,
        git_context: true,
        git_context_depth: 3,
    };

    let cache = Arc::new(FileCache::new());
    let markdown = generate_markdown(vec![file_info], options, cache).unwrap();

    // Should include both enhanced context (file size, type) and git context
    assert!(markdown.contains("example.rs"));
    assert!(
        markdown.contains("100 B") || markdown.contains("Rust"),
        "Should include enhanced context info"
    );
    assert!(
        markdown.contains("feat:") || markdown.contains("Test User"),
        "Should include git context info"
    );
}

#[test]
fn test_markdown_git_context_non_git_directory() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("example.rs");
    fs::write(&file_path, "fn main() {}\n").unwrap();

    let file_info = FileInfo {
        path: file_path.clone(),
        relative_path: PathBuf::from("example.rs"),
        size: 100,
        file_type: FileType::Rust,
        priority: 1.0,
        imports: vec![],
        imported_by: vec![],
        function_calls: vec![],
        type_references: vec![],
        exported_functions: vec![],
    };

    let options = ContextOptions {
        max_tokens: None,
        include_tree: false,
        include_stats: false,
        group_by_type: false,
        sort_by_priority: false,
        file_header_template: "## {path}".to_string(),
        doc_header_template: "".to_string(),
        include_toc: false,
        enhanced_context: false,
        git_context: true,
        git_context_depth: 3,
    };

    let cache = Arc::new(FileCache::new());
    let markdown = generate_markdown(vec![file_info], options, cache).unwrap();

    // Should gracefully handle non-git directories
    assert!(markdown.contains("## example.rs"));
    // Should not crash or include git info
    assert!(
        !markdown.contains("commit"),
        "Should not contain commit info for non-git directory"
    );
}
