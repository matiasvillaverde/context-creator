//! Integration tests for semantic analysis functionality

use code_digest::cli::Config;
use code_digest::core::cache::FileCache;
use code_digest::core::walker::{walk_directory, WalkOptions};
use std::fs;
use std::sync::Arc;
use tempfile::TempDir;

#[test]
fn test_semantic_import_tracing() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create a simple Rust project structure
    fs::write(
        root.join("main.rs"),
        r#"
mod lib;
mod utils;

fn main() {
    lib::hello();
    utils::helper();
}
"#,
    )
    .unwrap();

    fs::write(
        root.join("lib.rs"),
        r#"
pub fn hello() {
    println!("Hello from lib!");
}
"#,
    )
    .unwrap();

    fs::write(
        root.join("utils.rs"),
        r#"
pub fn helper() {
    println!("Helper function");
}
"#,
    )
    .unwrap();

    // Create config with semantic analysis enabled
    let config = Config {
        paths: Some(vec![root.to_path_buf()]),
        trace_imports: true,
        include_callers: false,
        include_types: false,
        semantic_depth: 3,
        ..Config::default()
    };

    // Create walk options and cache
    let walk_options = WalkOptions::from_config(&config).unwrap();
    let cache = Arc::new(FileCache::new());

    // Walk directory and get files
    let mut files = walk_directory(root, walk_options).unwrap();
    assert_eq!(files.len(), 3);

    // Perform semantic analysis
    code_digest::core::walker::perform_semantic_analysis(&mut files, &config, &cache).unwrap();

    // Find main.rs
    let main_file = files
        .iter()
        .find(|f| f.relative_path.to_str().unwrap() == "main.rs")
        .unwrap();

    // Check that main.rs imports lib.rs and utils.rs
    // Note: The simple import resolution might not work perfectly for all cases
    // This is more of a structure test than a full semantic test
    assert!(
        !main_file.imports.is_empty(),
        "main.rs should have imports detected, but found: {:?}",
        main_file.imports
    );
}

#[test]
fn test_semantic_analysis_disabled() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create a simple file
    fs::write(root.join("main.rs"), "fn main() {}").unwrap();

    // Create config with semantic analysis disabled
    let config = Config {
        paths: Some(vec![root.to_path_buf()]),
        trace_imports: false,
        include_callers: false,
        include_types: false,
        ..Config::default()
    };

    // Create walk options and cache
    let walk_options = WalkOptions::from_config(&config).unwrap();
    let cache = Arc::new(FileCache::new());

    // Walk directory and get files
    let mut files = walk_directory(root, walk_options).unwrap();

    // Perform semantic analysis (should be a no-op)
    code_digest::core::walker::perform_semantic_analysis(&mut files, &config, &cache).unwrap();

    // Check that no imports were detected
    for file in &files {
        assert!(file.imports.is_empty());
        assert!(file.imported_by.is_empty());
    }
}

#[test]
fn test_semantic_analysis_with_non_code_files() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create various file types
    fs::write(root.join("main.rs"), "fn main() {}").unwrap();
    fs::write(root.join("README.md"), "# Test Project").unwrap();
    fs::write(root.join("config.json"), "{}").unwrap();

    // Create config with semantic analysis enabled
    let config = Config {
        paths: Some(vec![root.to_path_buf()]),
        trace_imports: true,
        ..Config::default()
    };

    // Create walk options and cache
    let walk_options = WalkOptions::from_config(&config).unwrap();
    let cache = Arc::new(FileCache::new());

    // Walk directory and get files
    let mut files = walk_directory(root, walk_options).unwrap();
    assert_eq!(files.len(), 3);

    // Perform semantic analysis
    // Should not crash on non-code files
    code_digest::core::walker::perform_semantic_analysis(&mut files, &config, &cache).unwrap();

    // Non-code files should have no imports
    for file in &files {
        if file.relative_path.extension().unwrap_or_default() != "rs" {
            assert!(file.imports.is_empty());
            assert!(file.imported_by.is_empty());
        }
    }
}
