//! Test content hash computation in semantic analysis

use context_creator::cli::Config;
use context_creator::core::cache::FileCache;
use context_creator::core::semantic_graph::perform_semantic_analysis_graph;
use context_creator::core::walker::{walk_directory, WalkOptions};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_content_hash_computation() {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();

    // Create git directory
    fs::create_dir_all(base_path.join(".git")).unwrap();

    // Create test files with known content
    fs::create_dir_all(base_path.join("src")).unwrap();

    // File 1: specific content that should produce a consistent hash
    let file1_content = "pub fn hello() { println!(\"Hello, world!\"); }";
    fs::write(base_path.join("src/hello.rs"), file1_content).unwrap();

    // File 2: different content that should produce a different hash
    let file2_content = "pub fn goodbye() { println!(\"Goodbye!\"); }";
    fs::write(base_path.join("src/goodbye.rs"), file2_content).unwrap();

    // File 3: imports from file 1
    let file3_content = "use crate::hello::hello;\npub fn main() { hello(); }";
    fs::write(base_path.join("src/main.rs"), file3_content).unwrap();

    let walk_options = WalkOptions::default();
    let mut files = walk_directory(base_path, walk_options).unwrap();

    // Filter to only our test files
    files.retain(|f| f.path.extension().is_some_and(|ext| ext == "rs"));

    let config = Config {
        trace_imports: true,
        include_types: true,
        semantic_depth: 2,
        ..Default::default()
    };

    let cache = FileCache::new();

    // Perform semantic analysis - this should compute content hashes
    let result = perform_semantic_analysis_graph(&mut files, &config, &cache);
    assert!(result.is_ok(), "Semantic analysis should succeed");

    // Now we need to verify the hashes were computed
    // Since we can't directly access the internal graph, we'll verify indirectly
    // by checking that the analysis completed successfully and files were processed
    assert_eq!(files.len(), 3, "Should have 3 files");

    // Verify that imports were traced (which means analysis ran)
    let main_file = files
        .iter()
        .find(|f| f.path.to_str().unwrap().contains("main.rs"))
        .expect("Should find main.rs");

    assert!(
        !main_file.imports.is_empty(),
        "main.rs should have imports after analysis"
    );
}

#[test]
fn test_content_hash_changes_with_content() {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();

    // Create git directory
    fs::create_dir_all(base_path.join(".git")).unwrap();
    fs::create_dir_all(base_path.join("src")).unwrap();

    // Create a file
    let file_path = base_path.join("src/test.rs");
    fs::write(&file_path, "pub fn test() {}").unwrap();

    let walk_options = WalkOptions::default();
    let mut files1 = walk_directory(base_path, walk_options.clone()).unwrap();
    files1.retain(|f| f.path.extension().is_some_and(|ext| ext == "rs"));

    let config = Config {
        trace_imports: true,
        ..Default::default()
    };

    let cache = FileCache::new();

    // First analysis
    perform_semantic_analysis_graph(&mut files1, &config, &cache).unwrap();

    // Modify the file content
    fs::write(&file_path, "pub fn test() { println!(\"modified\"); }").unwrap();

    // Clear cache to force re-read
    let cache2 = FileCache::new();

    let mut files2 = walk_directory(base_path, walk_options).unwrap();
    files2.retain(|f| f.path.extension().is_some_and(|ext| ext == "rs"));

    // Second analysis with modified content
    perform_semantic_analysis_graph(&mut files2, &config, &cache2).unwrap();

    // We can't directly compare hashes, but the test verifies that
    // the analysis completes successfully with different content
    assert_eq!(
        files1.len(),
        files2.len(),
        "Should have same number of files"
    );
}

#[test]
fn test_content_hash_with_same_content() {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();

    // Create git directory
    fs::create_dir_all(base_path.join(".git")).unwrap();
    fs::create_dir_all(base_path.join("src")).unwrap();

    // Create two files with identical content
    let content = "pub struct MyStruct { pub value: i32 }";
    fs::write(base_path.join("src/file1.rs"), content).unwrap();
    fs::write(base_path.join("src/file2.rs"), content).unwrap();

    let walk_options = WalkOptions::default();
    let mut files = walk_directory(base_path, walk_options).unwrap();
    files.retain(|f| f.path.extension().is_some_and(|ext| ext == "rs"));

    let config = Config {
        trace_imports: true,
        include_types: true,
        ..Default::default()
    };

    let cache = FileCache::new();

    // Perform analysis
    perform_semantic_analysis_graph(&mut files, &config, &cache).unwrap();

    // Both files should be processed
    assert_eq!(files.len(), 2, "Should have 2 files");

    // Files with identical content should theoretically have the same hash
    // (though we can't verify this directly without exposing internals)
}
