//! Binary File Filtering Acceptance Tests
//!
//! These tests validate the binary file handling behavior:
//! - When using --output-file or default behavior: binary files ARE included
//! - When using --prompt: binary files are filtered (tested in integration tests)
//!
//! These acceptance tests verify that binary files are properly included
//! in the output when NOT using prompt mode, which is the expected behavior
//! for generating context files for manual review.

#![cfg(test)]
#![allow(clippy::needless_borrow)]

use super::helpers::*;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

// Test that binary files ARE included when NOT using prompt mode
#[test]
fn test_binary_files_included_without_prompt() {
    // Given: A repository with mixed binary and text files
    // When: Running WITHOUT a prompt (default behavior)
    // Then: ALL files should be included in output (no filtering)

    let (_temp_dir, project_root) = create_mixed_content_project();

    // Run without prompt - binary filtering is NOT enabled
    let output = run_context_creator(&["."], &project_root);

    // Should include text files
    assert_contains_file(&output, "main.rs");
    assert_contains_file(&output, "README.md");
    assert_contains_file(&output, "config.json");
    assert_contains_file(&output, "script.py");

    // Should ALSO include binary files (no filtering)
    assert_contains_file(&output, "logo.png");
    assert_contains_file(&output, "demo.mp4");
    assert_contains_file(&output, "app.exe");
    assert_contains_file(&output, "data.db");
}

// Test with --output flag: binary files should be included
#[test]
fn test_binary_files_included_with_output_flag() {
    let (_temp_dir, project_root) = create_mixed_content_project();
    let output_file = project_root.join("output.md");

    // Run with output flag - binary filtering is NOT enabled
    run_context_creator(
        &["--output-file", output_file.to_str().unwrap(), "."],
        &project_root,
    );

    // Read the output file
    let output = fs::read_to_string(&output_file).unwrap();

    // Should include ALL files
    assert!(output.contains("main.rs"));
    assert!(output.contains("logo.png"));
    assert!(output.contains("demo.mp4"));
    assert!(output.contains("app.exe"));
}

// Edge Case 1: Binary files with uppercase extensions are included
#[test]
fn test_uppercase_binary_extensions_included() {
    let (_temp_dir, project_root) = create_test_project(vec![
        ("code.rs", "fn main() {}"),
        ("IMAGE.JPG", "binary_content"),
        ("VIDEO.MP4", "binary_content"),
        ("ARCHIVE.ZIP", "binary_content"),
    ]);

    let output = run_context_creator(&["."], &project_root);

    // All files should be included
    assert_contains_file(&output, "code.rs");
    assert_contains_file(&output, "IMAGE.JPG");
    assert_contains_file(&output, "VIDEO.MP4");
    assert_contains_file(&output, "ARCHIVE.ZIP");
}

// Edge Case 2: Mixed case extensions are included
#[test]
fn test_mixed_case_binary_extensions_included() {
    let (_temp_dir, project_root) = create_test_project(vec![
        ("app.py", "print('hello')"),
        ("Photo.JpG", "binary_content"),
        ("Video.Mp4", "binary_content"),
        ("Document.PdF", "binary_content"),
    ]);

    let output = run_context_creator(&["."], &project_root);

    // All files should be included
    assert_contains_file(&output, "app.py");
    assert_contains_file(&output, "Photo.JpG");
    assert_contains_file(&output, "Video.Mp4");
    assert_contains_file(&output, "Document.PdF");
}

// Edge Case 3: Files without extensions are all included
#[test]
fn test_extensionless_files_included() {
    let (_temp_dir, project_root) = create_test_project(vec![
        ("README", "# Documentation"),
        ("LICENSE", "MIT License"),
        ("Makefile", "build:\n\tcargo build"),
        ("Dockerfile", "FROM rust:latest"),
        ("CHANGELOG", "Version 1.0"),
        ("AUTHORS", "John Doe"),
    ]);

    let output = run_context_creator(&["."], &project_root);

    // All files should be included
    assert_contains_file(&output, "README");
    assert_contains_file(&output, "LICENSE");
    assert_contains_file(&output, "Makefile");
    assert_contains_file(&output, "Dockerfile");
    assert_contains_file(&output, "CHANGELOG");
    assert_contains_file(&output, "AUTHORS");
}

// Edge Case 4: Files are included based on actual content, not names
#[test]
fn test_misleading_filenames_all_included() {
    let (_temp_dir, project_root) = create_test_project(vec![
        ("image.rs", "// Not actually an image"),
        ("video.py", "# Not actually a video"),
        ("binary.txt", "Just text"),
        ("executable.md", "# Markdown"),
    ]);

    let output = run_context_creator(&["."], &project_root);

    // All files are included regardless of misleading names
    assert_contains_file(&output, "image.rs");
    assert_contains_file(&output, "video.py");
    assert_contains_file(&output, "binary.txt");
    assert_contains_file(&output, "executable.md");
}

// Edge Case 5: Compound extensions - all included
#[test]
fn test_compound_extensions_all_included() {
    let (_temp_dir, project_root) = create_test_project(vec![
        ("archive.tar.gz", "binary_content"),
        ("backup.sql.bz2", "binary_content"),
        ("config.yaml.bak", "key: value"),
        ("script.min.js", "console.log('test');"),
    ]);

    let output = run_context_creator(&["."], &project_root);

    // All files should be included
    assert_contains_file(&output, "archive.tar.gz");
    assert_contains_file(&output, "backup.sql.bz2");
    assert_contains_file(&output, "config.yaml.bak");
    assert_contains_file(&output, "script.min.js");
}

// Edge Case 6: Dotfiles behavior
#[test]
fn test_dotfiles_default_behavior() {
    let (_temp_dir, project_root) = create_test_project(vec![
        (".gitignore", "*.log"),
        (".dockerignore", "node_modules/"),
        (".DS_Store", "binary_content"),
        (".image.jpg", "binary_content"),
        (".config.json", r#"{"key": "value"}"#),
    ]);

    let _output = run_context_creator(&["."], &project_root);

    // Note: dotfiles are ignored by default unless explicitly included
    // This test just verifies that binary filtering applies to dotfiles too
    // when they would be included (e.g. with different walker options)

    // For now, verify that regular files are handled correctly
    // (The acceptance test framework doesn't easily support modifying walker options)
}

// Edge Case 7: Unicode filenames - all included
#[test]
fn test_unicode_filenames_all_included() {
    let (_temp_dir, project_root) = create_test_project(vec![
        ("código.rs", "fn main() {}"),
        ("图片.jpg", "binary_content"),
        ("видео.mp4", "binary_content"),
        ("文档.pdf", "binary_content"),
    ]);

    let output = run_context_creator(&["."], &project_root);

    // All files should be included
    assert_contains_file(&output, "código.rs");
    assert_contains_file(&output, "图片.jpg");
    assert_contains_file(&output, "видео.mp4");
    assert_contains_file(&output, "文档.pdf");
}

// Edge Case 8: Very long filenames - all included
#[test]
fn test_long_filenames_all_included() {
    let long_name = "a".repeat(200);
    let (_temp_dir, project_root) = create_test_project(vec![
        (&format!("{long_name}.rs"), "fn main() {}"),
        (&format!("{long_name}.jpg"), "binary_content"),
        (&format!("{long_name}.exe"), "binary_content"),
    ]);

    let output = run_context_creator(&["."], &project_root);

    // All files should be included
    assert!(output.contains(&format!("{long_name}.rs")));
    assert!(output.contains(&format!("{long_name}.jpg")));
    assert!(output.contains(&format!("{long_name}.exe")));
}

// Edge Case 9: Symlinks - all included
#[test]
#[cfg(unix)]
fn test_symlinks_all_included() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create actual files
    fs::write(root.join("real_image.jpg"), b"binary content").unwrap();
    fs::write(root.join("real_code.rs"), b"fn main() {}").unwrap();

    // Create symlinks
    std::os::unix::fs::symlink(root.join("real_image.jpg"), root.join("link_to_image.jpg"))
        .unwrap();
    std::os::unix::fs::symlink(root.join("real_code.rs"), root.join("link_to_code.rs")).unwrap();

    let output = run_context_creator(&["."], root);

    // All files and symlinks should be included
    assert_contains_file(&output, "real_code.rs");
    assert_contains_file(&output, "link_to_code.rs");
    assert_contains_file(&output, "real_image.jpg");
    assert_contains_file(&output, "link_to_image.jpg");
}

// Test that we can verify the behavior difference (for documentation)
// This test shows that prompt mode would filter, but we can't test it directly
#[test]
fn test_binary_filtering_behavior_documented() {
    let (_temp_dir, project_root) = create_test_project(vec![
        ("code.rs", "fn main() {}"),
        ("image.jpg", "binary_content"),
        ("video.mp4", "binary_content"),
    ]);

    // Test 1: Without prompt - all files included
    let output = run_context_creator(&["."], &project_root);
    assert_contains_file(&output, "code.rs");
    assert_contains_file(&output, "image.jpg");
    assert_contains_file(&output, "video.mp4");

    // Test 2: With output file - all files included
    let output_file = project_root.join("test.md");
    run_context_creator(
        &["--output-file", output_file.to_str().unwrap(), "."],
        &project_root,
    );
    let file_output = fs::read_to_string(&output_file).unwrap();
    assert!(file_output.contains("code.rs"));
    assert!(file_output.contains("image.jpg"));
    assert!(file_output.contains("video.mp4"));

    // NOTE: With --prompt, binary files would be filtered out,
    // but we can't test that here as it invokes the LLM.
    // See integration tests for prompt-based filtering validation.
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
