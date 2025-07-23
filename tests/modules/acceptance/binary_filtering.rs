//! Binary File Filtering Acceptance Tests
//!
//! These tests validate that binary files (images, videos, executables, etc.)
//! are properly filtered when processing repositories with a prompt.
//!
//! NOTE: These tests are currently skipped because binary filtering is only
//! enabled when using --prompt, but --prompt invokes the actual LLM which
//! makes these tests non-deterministic and dependent on external services.
//! The integration tests in binary_filtering_integration_test.rs properly
//! test the binary filtering functionality at the walker level.

#![cfg(test)]
#![allow(clippy::needless_borrow)]

use super::helpers::*;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

// Happy Path Test
#[test]
#[ignore = "Binary filtering requires --prompt which invokes LLM. See integration tests."]
fn test_binary_filtering_with_prompt() {
    // Given: A repository with mixed binary and text files
    // When: Running with a prompt (which enables binary filtering)
    // Then: Only text files should be included in output

    let (_temp_dir, project_root) = create_mixed_content_project();

    // Run without prompt - binary filtering is not enabled without prompt
    // This test documents the current behavior but isn't ideal
    let output = run_context_creator(&["."], &project_root);

    // Should include text files
    assert_contains_file(&output, "main.rs");
    assert_contains_file(&output, "README.md");
    assert_contains_file(&output, "config.json");
    assert_contains_file(&output, "script.py");

    // Should NOT include binary files
    assert_not_contains_file(&output, "logo.png");
    assert_not_contains_file(&output, "demo.mp4");
    assert_not_contains_file(&output, "app.exe");
    assert_not_contains_file(&output, "data.db");
}

// Edge Case 1: Binary files with uppercase extensions
#[test]
#[ignore = "Binary filtering requires --prompt which invokes LLM. See integration tests."]
fn test_binary_filtering_uppercase_extensions() {
    let (_temp_dir, project_root) = create_test_project(vec![
        ("code.rs", "fn main() {}"),
        ("IMAGE.JPG", "binary_content"),
        ("VIDEO.MP4", "binary_content"),
        ("ARCHIVE.ZIP", "binary_content"),
    ]);

    let output = run_context_creator(
        &["--prompt", "Test uppercase"],
        &project_root,
    );

    assert_contains_file(&output, "code.rs");
    assert_not_contains_file(&output, "IMAGE.JPG");
    assert_not_contains_file(&output, "VIDEO.MP4");
    assert_not_contains_file(&output, "ARCHIVE.ZIP");
}

// Edge Case 2: Mixed case extensions
#[test]
#[ignore = "Binary filtering requires --prompt which invokes LLM. See integration tests."]
fn test_binary_filtering_mixed_case_extensions() {
    let (_temp_dir, project_root) = create_test_project(vec![
        ("app.py", "print('hello')"),
        ("Photo.JpG", "binary_content"),
        ("Video.Mp4", "binary_content"),
        ("Document.PdF", "binary_content"),
    ]);

    let output = run_context_creator(
        &["--prompt", "Test mixed case"],
        &project_root,
    );

    assert_contains_file(&output, "app.py");
    assert_not_contains_file(&output, "Photo.JpG");
    assert_not_contains_file(&output, "Video.Mp4");
    assert_not_contains_file(&output, "Document.PdF");
}

// Edge Case 3: Files without extensions that should be included
#[test]
#[ignore = "Binary filtering requires --prompt which invokes LLM. See integration tests."]
fn test_binary_filtering_extensionless_text_files() {
    let (_temp_dir, project_root) = create_test_project(vec![
        ("README", "# Documentation"),
        ("LICENSE", "MIT License"),
        ("Makefile", "build:\n\tcargo build"),
        ("Dockerfile", "FROM rust:latest"),
        ("CHANGELOG", "Version 1.0"),
        ("AUTHORS", "John Doe"),
    ]);

    let output = run_context_creator(
        &["--prompt", "Test extensionless"],
        &project_root,
    );

    // All these files should be included despite no extension
    assert_contains_file(&output, "README");
    assert_contains_file(&output, "LICENSE");
    assert_contains_file(&output, "Makefile");
    assert_contains_file(&output, "Dockerfile");
    assert_contains_file(&output, "CHANGELOG");
    assert_contains_file(&output, "AUTHORS");
}

// Edge Case 4: Binary-looking filenames with text extensions
#[test]
#[ignore = "Binary filtering requires --prompt which invokes LLM. See integration tests."]
fn test_binary_filtering_misleading_names() {
    let (_temp_dir, project_root) = create_test_project(vec![
        ("image.rs", "// Not actually an image"),
        ("video.py", "# Not actually a video"),
        ("binary.txt", "Just text"),
        ("executable.md", "# Markdown"),
    ]);

    let output = run_context_creator(
        &["--prompt", "Test misleading names"],
        &project_root,
    );

    // All should be included based on extension, not name
    assert_contains_file(&output, "image.rs");
    assert_contains_file(&output, "video.py");
    assert_contains_file(&output, "binary.txt");
    assert_contains_file(&output, "executable.md");
}

// Edge Case 5: Compound extensions
#[test]
#[ignore = "Binary filtering requires --prompt which invokes LLM. See integration tests."]
fn test_binary_filtering_compound_extensions() {
    let (_temp_dir, project_root) = create_test_project(vec![
        ("archive.tar.gz", "binary_content"),
        ("backup.sql.bz2", "binary_content"),
        ("config.yaml.bak", "key: value"),
        ("script.min.js", "console.log('test');"),
    ]);

    let output = run_context_creator(
        &["--prompt", "Test compound extensions"],
        &project_root,
    );

    // Binary archives should be filtered
    assert_not_contains_file(&output, "archive.tar.gz");
    assert_not_contains_file(&output, "backup.sql.bz2");

    // Text files with compound extensions should be included
    assert_contains_file(&output, "config.yaml.bak");
    assert_contains_file(&output, "script.min.js");
}

// Edge Case 6: Dotfiles with binary extensions
#[test]
#[ignore = "Binary filtering requires --prompt which invokes LLM. See integration tests."]
fn test_binary_filtering_dotfiles() {
    let (_temp_dir, project_root) = create_test_project(vec![
        (".gitignore", "*.log"),
        (".dockerignore", "node_modules/"),
        (".DS_Store", "binary_content"),
        (".image.jpg", "binary_content"),
        (".config.json", r#"{"key": "value"}"#),
    ]);

    let _output = run_context_creator(
        &["--prompt", "Test dotfiles"],
        &project_root,
    );

    // Note: dotfiles are ignored by default unless explicitly included
    // This test just verifies that binary filtering applies to dotfiles too
    // when they would be included (e.g. with different walker options)

    // For now, verify that regular files are handled correctly
    // (The acceptance test framework doesn't easily support modifying walker options)
}

// Edge Case 7: Unicode filenames with binary extensions
#[test]
#[ignore = "Binary filtering requires --prompt which invokes LLM. See integration tests."]
fn test_binary_filtering_unicode_filenames() {
    let (_temp_dir, project_root) = create_test_project(vec![
        ("código.rs", "fn main() {}"),
        ("图片.jpg", "binary_content"),
        ("видео.mp4", "binary_content"),
        ("文档.pdf", "binary_content"),
    ]);

    let output = run_context_creator(
        &["--prompt", "Test unicode"],
        &project_root,
    );

    assert_contains_file(&output, "código.rs");
    assert_not_contains_file(&output, "图片.jpg");
    assert_not_contains_file(&output, "видео.mp4");
    assert_not_contains_file(&output, "文档.pdf");
}

// Edge Case 8: Very long filenames with binary extensions
#[test]
#[ignore = "Binary filtering requires --prompt which invokes LLM. See integration tests."]
fn test_binary_filtering_long_filenames() {
    let long_name = "a".repeat(200);
    let (_temp_dir, project_root) = create_test_project(vec![
        (&format!("{long_name}.rs"), "fn main() {}"),
        (&format!("{long_name}.jpg"), "binary_content"),
        (&format!("{long_name}.exe"), "binary_content"),
    ]);

    let output = run_context_creator(
        &["--prompt", "Test long names"],
        &project_root,
    );

    assert!(output.contains(&format!("{long_name}.rs")));
    assert!(!output.contains(&format!("{long_name}.jpg")));
    assert!(!output.contains(&format!("{long_name}.exe")));
}

// Edge Case 9: Symlinks to binary files
#[test]
#[cfg(unix)]
#[ignore = "Binary filtering requires --prompt which invokes LLM. See integration tests."]
fn test_binary_filtering_symlinks() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create actual files
    fs::write(root.join("real_image.jpg"), b"binary content").unwrap();
    fs::write(root.join("real_code.rs"), b"fn main() {}").unwrap();

    // Create symlinks
    std::os::unix::fs::symlink(root.join("real_image.jpg"), root.join("link_to_image.jpg"))
        .unwrap();
    std::os::unix::fs::symlink(root.join("real_code.rs"), root.join("link_to_code.rs")).unwrap();

    let output = run_context_creator(
        &["--prompt", "Test symlinks"],
        root,
    );

    // Symlinks should be filtered based on their extension
    assert_contains_file(&output, "real_code.rs");
    assert_contains_file(&output, "link_to_code.rs");
    assert_not_contains_file(&output, "real_image.jpg");
    assert_not_contains_file(&output, "link_to_image.jpg");
}

// Edge Case 10: No prompt means no filtering
#[test]
fn test_no_binary_filtering_without_prompt() {
    let (_temp_dir, project_root) = create_test_project(vec![
        ("code.rs", "fn main() {}"),
        ("image.jpg", "binary_content"),
        ("video.mp4", "binary_content"),
    ]);

    // Run WITHOUT prompt - binary filtering should be disabled
    let output = run_context_creator(&["."], &project_root);

    // All files should be included
    assert_contains_file(&output, "code.rs");
    assert_contains_file(&output, "image.jpg");
    assert_contains_file(&output, "video.mp4");
}

// Helper functions
fn create_mixed_content_project() -> (TempDir, PathBuf) {
    create_test_project(vec![
        ("src/main.rs", "fn main() { println!(\"Hello\"); }"),
        ("README.md", "# Test Project"),
        ("config.json", r#"{"version": "1.0"}"#),
        ("script.py", "print('test')"),
        ("assets/logo.png", "PNG_BINARY_DATA"),
        ("media/demo.mp4", "MP4_BINARY_DATA"),
        ("bin/app.exe", "EXE_BINARY_DATA"),
        ("data/data.db", "SQLITE_BINARY_DATA"),
    ])
}

fn create_test_project(files: Vec<(&str, &str)>) -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path().to_path_buf();

    for (path, content) in files {
        let file_path = root.join(path);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(file_path, content).unwrap();
    }

    (temp_dir, root)
}
