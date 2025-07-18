//! Test cycle detection using Kahn's algorithm

use context_creator::cli::Config;
use context_creator::core::cache::FileCache;
use context_creator::core::semantic_graph::perform_semantic_analysis_graph;
use context_creator::core::walker::{walk_directory, WalkOptions};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_cycle_detection_no_cycle() {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();

    // Create git directory
    fs::create_dir_all(base_path.join(".git")).unwrap();

    // Create files with no circular dependencies
    fs::create_dir_all(base_path.join("src")).unwrap();
    fs::write(base_path.join("src/main.rs"), "use crate::utils::helper;\n").unwrap();
    fs::write(base_path.join("src/utils.rs"), "use crate::models::User;\n").unwrap();
    fs::write(base_path.join("src/models.rs"), "pub struct User;\n").unwrap();

    let walk_options = WalkOptions {
        max_file_size: Some(10 * 1024 * 1024),
        follow_links: false,
        include_hidden: false,
        parallel: false,
        ignore_file: ".context-creator-ignore".to_string(),
        ignore_patterns: vec![],
        include_patterns: vec![],
        custom_priorities: vec![],
        filter_binary_files: false,
    };

    let mut files = walk_directory(base_path, walk_options).unwrap();
    // Filter to only our test files
    files.retain(|f| f.path.extension().is_some_and(|ext| ext == "rs"));

    let config = Config {
        trace_imports: true,
        include_types: true,
        include_callers: false,
        semantic_depth: 3,
        ..Default::default()
    };

    let cache = FileCache::new();

    // Should not fail with cycle detection
    let result = perform_semantic_analysis_graph(&mut files, &config, &cache);
    assert!(result.is_ok(), "Should not detect cycle in acyclic graph");
}

#[test]
fn test_cycle_detection_with_cycle() {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();

    // Create git directory
    fs::create_dir_all(base_path.join(".git")).unwrap();

    // Create files with circular dependencies
    fs::create_dir_all(base_path.join("src")).unwrap();
    fs::write(base_path.join("src/a.rs"), "use crate::b::B;\n").unwrap();
    fs::write(base_path.join("src/b.rs"), "use crate::c::C;\n").unwrap();
    fs::write(base_path.join("src/c.rs"), "use crate::a::A;\n").unwrap(); // Cycle: A -> B -> C -> A

    let walk_options = WalkOptions {
        max_file_size: Some(10 * 1024 * 1024),
        follow_links: false,
        include_hidden: false,
        parallel: false,
        ignore_file: ".context-creator-ignore".to_string(),
        ignore_patterns: vec![],
        include_patterns: vec![],
        custom_priorities: vec![],
        filter_binary_files: false,
    };

    let mut files = walk_directory(base_path, walk_options).unwrap();
    // Filter to only our test files
    files.retain(|f| f.path.extension().is_some_and(|ext| ext == "rs"));

    let config = Config {
        trace_imports: true,
        include_types: true,
        include_callers: false,
        semantic_depth: 3,
        ..Default::default()
    };

    let cache = FileCache::new();

    // Should detect cycle and handle it gracefully
    let result = perform_semantic_analysis_graph(&mut files, &config, &cache);
    // We expect this to succeed but with warning about cycle
    assert!(
        result.is_ok(),
        "Should handle cycle gracefully with warning"
    );
}

#[test]
fn test_self_referential_cycle() {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();

    // Create git directory
    fs::create_dir_all(base_path.join(".git")).unwrap();

    // Create file that imports itself
    fs::create_dir_all(base_path.join("src")).unwrap();
    fs::write(
        base_path.join("src/recursive.rs"),
        "use crate::recursive::Recursive;\n",
    )
    .unwrap();

    let walk_options = WalkOptions {
        max_file_size: Some(10 * 1024 * 1024),
        follow_links: false,
        include_hidden: false,
        parallel: false,
        ignore_file: ".context-creator-ignore".to_string(),
        ignore_patterns: vec![],
        include_patterns: vec![],
        custom_priorities: vec![],
        filter_binary_files: false,
    };

    let mut files = walk_directory(base_path, walk_options).unwrap();
    // Filter to only our test files
    files.retain(|f| f.path.extension().is_some_and(|ext| ext == "rs"));

    let config = Config {
        trace_imports: true,
        include_types: true,
        include_callers: false,
        semantic_depth: 3,
        ..Default::default()
    };

    let cache = FileCache::new();

    // Should detect self-referential cycle
    let result = perform_semantic_analysis_graph(&mut files, &config, &cache);
    assert!(
        result.is_ok(),
        "Should handle self-referential cycle gracefully"
    );
}
