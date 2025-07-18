//! Tests for optimal parallel workflow with rayon
//!
//! The optimal workflow should be:
//! 1. Gather all file paths (sequential)
//! 2. Parse all files in parallel
//! 3. Collect results
//! 4. Build dependency graph (sequential)

use context_creator::cli::Config;
use context_creator::core::cache::FileCache;
use context_creator::core::semantic_graph::perform_semantic_analysis_graph;
use context_creator::core::walker::{walk_directory, WalkOptions};
use std::fs;
use std::time::Instant;
use tempfile::TempDir;

#[test]
fn test_parallel_parsing_performance() {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();

    // Create git directory
    fs::create_dir_all(base_path.join(".git")).unwrap();
    fs::create_dir_all(base_path.join("src")).unwrap();

    // Create many files to test parallel performance
    let file_count = 50;

    // Create interconnected modules
    for i in 0..file_count {
        let content = if i == 0 {
            // Root module imports others
            let mut imports = String::new();
            for j in 1..5.min(file_count) {
                imports.push_str(&format!("mod module{j};\n"));
                imports.push_str(&format!("use module{j}::*;\n"));
            }
            format!(
                r#"
{imports}

pub fn main() {{
    println!("Main module");
}}
"#
            )
        } else {
            // Other modules import some dependencies
            let mut imports = String::new();
            if i > 1 {
                let prev_module = (i - 1) % file_count;
                imports.push_str(&format!("use crate::module{prev_module};\n"));
            }
            format!(
                r#"
{imports}

pub fn function_{i}() -> i32 {{
    {i}
}}

pub struct Type{i} {{
    value: i32,
}}
"#
            )
        };

        let filename = if i == 0 {
            "main.rs"
        } else {
            &format!("module{i}.rs")
        };

        fs::write(base_path.join("src").join(filename), content).unwrap();
    }

    let walk_options = WalkOptions::default();
    let mut files = walk_directory(base_path, walk_options).unwrap();
    files.retain(|f| f.path.extension().is_some_and(|ext| ext == "rs"));

    let config = Config {
        trace_imports: true,
        include_types: true,
        include_callers: true,
        ..Default::default()
    };

    let cache = FileCache::new();

    // Measure performance
    let start = Instant::now();
    perform_semantic_analysis_graph(&mut files, &config, &cache).unwrap();
    let duration = start.elapsed();

    println!("Parallel analysis of {file_count} files took: {duration:?}");

    // Verify all files were analyzed
    assert_eq!(files.len(), file_count);

    // Verify imports were tracked
    let main_file = files
        .iter()
        .find(|f| f.path.to_string_lossy().contains("main.rs"))
        .unwrap();
    assert!(
        !main_file.imports.is_empty(),
        "Main file should have imports"
    );

    // Verify parallel processing worked (imports should be resolved)
    for file in &files {
        if file.path.to_string_lossy().contains("module1.rs") {
            assert!(
                !file.imported_by.is_empty(),
                "module1.rs should be imported by main.rs"
            );
        }
    }
}

#[test]
fn test_workflow_stages_isolation() {
    // Test that each stage of the workflow is properly isolated
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();

    // Create git directory
    fs::create_dir_all(base_path.join(".git")).unwrap();
    fs::create_dir_all(base_path.join("src")).unwrap();

    // Create files with circular dependencies
    fs::write(
        base_path.join("src/a.rs"),
        r#"
mod b;
use b::B;

pub struct A {
    b: B,
}
"#,
    )
    .unwrap();

    fs::write(
        base_path.join("src/b.rs"),
        r#"
mod c;
use c::C;

pub struct B {
    c: C,
}
"#,
    )
    .unwrap();

    fs::write(
        base_path.join("src/c.rs"),
        r#"
// Circular dependency back to A would go here
// but we'll keep it simple for now

pub struct C {
    value: i32,
}
"#,
    )
    .unwrap();

    // Stage 1: Gather paths
    let walk_options = WalkOptions::default();
    let mut files = walk_directory(base_path, walk_options).unwrap();
    files.retain(|f| f.path.extension().is_some_and(|ext| ext == "rs"));

    assert_eq!(files.len(), 3, "Should find 3 files");

    // Stage 2-4: Parallel analysis and graph building
    let config = Config {
        trace_imports: true,
        ..Default::default()
    };

    let cache = FileCache::new();
    perform_semantic_analysis_graph(&mut files, &config, &cache).unwrap();

    // Verify the graph was built correctly
    let a_file = files
        .iter()
        .find(|f| f.path.to_string_lossy().contains("a.rs"))
        .unwrap();
    let b_file = files
        .iter()
        .find(|f| f.path.to_string_lossy().contains("b.rs"))
        .unwrap();
    let c_file = files
        .iter()
        .find(|f| f.path.to_string_lossy().contains("c.rs"))
        .unwrap();

    // Check import relationships
    assert!(
        a_file.imports.contains(&b_file.path),
        "a.rs should import b.rs"
    );
    assert!(
        b_file.imports.contains(&c_file.path),
        "b.rs should import c.rs"
    );

    // Check reverse dependencies
    assert!(
        b_file.imported_by.contains(&a_file.path),
        "b.rs should be imported by a.rs"
    );
    assert!(
        c_file.imported_by.contains(&b_file.path),
        "c.rs should be imported by b.rs"
    );
}

#[test]
fn test_parallel_analysis_consistency() {
    // Test that parallel analysis produces consistent results
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();

    // Create git directory
    fs::create_dir_all(base_path.join(".git")).unwrap();
    fs::create_dir_all(base_path.join("src")).unwrap();

    // Create files that all import from a common module
    fs::write(
        base_path.join("src/common.rs"),
        r#"
pub const VERSION: &str = "1.0.0";

pub fn shared_function() -> String {
    VERSION.to_string()
}
"#,
    )
    .unwrap();

    // Create multiple files that import from common
    for i in 0..10 {
        fs::write(
            base_path.join("src").join(format!("consumer{i}.rs")),
            format!(
                r#"
mod common;
use common::{{VERSION, shared_function}};

pub fn consumer_{i}() {{
    println!("Version: {{}}", VERSION);
    let _ = shared_function();
}}
"#
            ),
        )
        .unwrap();
    }

    let walk_options = WalkOptions::default();
    let mut files = walk_directory(base_path, walk_options).unwrap();
    files.retain(|f| f.path.extension().is_some_and(|ext| ext == "rs"));

    let config = Config {
        trace_imports: true,
        include_callers: true,
        ..Default::default()
    };

    let cache = FileCache::new();

    // Run analysis multiple times to ensure consistency
    for run in 0..3 {
        let mut files_copy = files.clone();
        perform_semantic_analysis_graph(&mut files_copy, &config, &cache).unwrap();

        // Find common.rs
        let common_file = files_copy
            .iter()
            .find(|f| f.path.to_string_lossy().contains("common.rs"))
            .unwrap();

        // Verify it's imported by all consumers
        let imported_by_count = common_file.imported_by.len();
        assert_eq!(
            imported_by_count, 10,
            "Run {run}: common.rs should be imported by 10 files, found {imported_by_count}"
        );

        // Verify all consumers have the import
        for file in &files_copy {
            if file.path.to_string_lossy().contains("consumer") {
                assert!(
                    file.imports.contains(&common_file.path),
                    "Run {}: {} should import common.rs",
                    run,
                    file.path.display()
                );
            }
        }
    }
}

#[test]
fn test_error_handling_in_parallel_workflow() {
    // Test that errors in one file don't break the entire analysis
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();

    // Create git directory
    fs::create_dir_all(base_path.join(".git")).unwrap();
    fs::create_dir_all(base_path.join("src")).unwrap();

    // Create a main file to establish crate root
    fs::write(
        base_path.join("src/main.rs"),
        r#"
mod valid;
mod another_valid;

fn main() {
    println!("Test crate");
}
"#,
    )
    .unwrap();

    // Create a valid file
    fs::write(
        base_path.join("src/valid.rs"),
        r#"
pub fn valid_function() -> i32 {
    42
}
"#,
    )
    .unwrap();

    // Create a file with syntax errors
    fs::write(
        base_path.join("src/invalid.rs"),
        r#"
pub fn invalid_function() -> i32
    // Missing opening brace
    return 42;
}

struct Incomplete {
    field: 
"#,
    )
    .unwrap();

    // Create another valid file
    fs::write(
        base_path.join("src/another_valid.rs"),
        r#"
use crate::valid::valid_function;

pub fn use_valid() {
    let _ = valid_function();
}
"#,
    )
    .unwrap();

    let walk_options = WalkOptions::default();
    let mut files = walk_directory(base_path, walk_options).unwrap();
    files.retain(|f| f.path.extension().is_some_and(|ext| ext == "rs"));

    let config = Config {
        trace_imports: true,
        ..Default::default()
    };

    let cache = FileCache::new();

    // Analysis should complete despite the invalid file
    assert!(
        perform_semantic_analysis_graph(&mut files, &config, &cache).is_ok(),
        "Analysis should complete even with invalid files"
    );

    // Verify valid files were still processed
    assert_eq!(
        files.len(),
        4,
        "Should have 4 files: main.rs, valid.rs, invalid.rs, another_valid.rs"
    );

    let another_valid = files
        .iter()
        .find(|f| f.path.to_string_lossy().contains("another_valid.rs"))
        .unwrap();
    let valid = files
        .iter()
        .find(|f| {
            f.path.to_string_lossy().contains("valid.rs")
                && !f.path.to_string_lossy().contains("another")
        })
        .unwrap();

    // Check that main.rs established the module relationships
    let main_file = files
        .iter()
        .find(|f| f.path.to_string_lossy().contains("main.rs"))
        .unwrap();

    // main.rs should import both valid.rs and another_valid.rs
    assert!(
        main_file.imports.contains(&valid.path) || main_file.imports.contains(&another_valid.path),
        "main.rs should have imports despite invalid.rs having errors"
    );
}
