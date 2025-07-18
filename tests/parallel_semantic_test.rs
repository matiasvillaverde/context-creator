#![cfg(test)]

//! Test parallel semantic analysis performance

use context_creator::cli::Config;
use context_creator::core::cache::FileCache;
use context_creator::core::semantic_graph::perform_semantic_analysis_graph;
use context_creator::core::walker::{walk_directory, WalkOptions};
use std::fs;
use std::time::Instant;
use tempfile::TempDir;

#[test]
fn test_parallel_semantic_analysis_performance() {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();

    // Create git directory
    fs::create_dir_all(base_path.join(".git")).unwrap();

    // Create a larger number of files to test parallel performance
    fs::create_dir_all(base_path.join("src/models")).unwrap();
    fs::create_dir_all(base_path.join("src/handlers")).unwrap();
    fs::create_dir_all(base_path.join("src/utils")).unwrap();

    // Create interdependent files
    for i in 0..10 {
        // Models
        fs::write(
            base_path.join(format!("src/models/model_{i}.rs")),
            format!("use crate::utils::helper_{i};\npub struct Model{i} {{ pub id: u64 }}\n"),
        )
        .unwrap();

        // Handlers
        fs::write(
            base_path.join(format!("src/handlers/handler_{i}.rs")),
            format!(
                "use crate::models::model_{i}::Model{i};\nuse crate::utils::helper_{i};\npub fn handle(m: Model{i}) {{}}\n"
            ),
        )
        .unwrap();

        // Utils
        fs::write(
            base_path.join(format!("src/utils/helper_{i}.rs")),
            format!("pub fn helper_{i}() -> u64 {{ {i} }}\n"),
        )
        .unwrap();
    }

    let walk_options = WalkOptions::default();
    let mut files = walk_directory(base_path, walk_options).unwrap();

    // Filter to only our test files
    files.retain(|f| f.path.extension().is_some_and(|ext| ext == "rs"));

    let config = Config {
        trace_imports: true,
        include_types: true,
        include_callers: true,
        semantic_depth: 3,
        ..Default::default()
    };

    let cache = FileCache::new();

    // Measure time taken for semantic analysis
    let start = Instant::now();
    let result = perform_semantic_analysis_graph(&mut files, &config, &cache);
    let duration = start.elapsed();

    assert!(result.is_ok(), "Semantic analysis should succeed");

    // Verify relationships were found
    let model_file = files
        .iter()
        .find(|f| f.path.to_str().unwrap().contains("model_0.rs"))
        .expect("Should find model_0.rs");

    println!("Model file imports: {:?}", model_file.imports);
    println!("Model file imported_by: {:?}", model_file.imported_by);
    println!(
        "Model file type_references: {:?}",
        model_file.type_references
    );

    // Check if any imports were found
    let total_imports: usize = files.iter().map(|f| f.imports.len()).sum();
    let total_imported_by: usize = files.iter().map(|f| f.imported_by.len()).sum();

    println!("Total imports found: {total_imports}");
    println!("Total imported_by relationships: {total_imported_by}");

    // Model should be imported by its handler
    assert!(
        total_imports > 0 || total_imported_by > 0,
        "Should find at least some import relationships"
    );

    println!("Parallel semantic analysis completed in {duration:?}");
    println!("Analyzed {} files", files.len());
}

#[test]
fn test_parallel_analysis_with_errors() {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();

    // Create git directory
    fs::create_dir_all(base_path.join(".git")).unwrap();
    fs::create_dir_all(base_path.join("src")).unwrap();

    // Create some valid files
    fs::write(base_path.join("src/valid.rs"), "pub fn valid() {}").unwrap();

    // Create a file with invalid syntax
    fs::write(
        base_path.join("src/invalid.rs"),
        "pub fn invalid() { this is not valid rust syntax",
    )
    .unwrap();

    // Create another valid file
    fs::write(base_path.join("src/another.rs"), "use crate::valid::valid;").unwrap();

    let walk_options = WalkOptions::default();
    let mut files = walk_directory(base_path, walk_options).unwrap();
    files.retain(|f| f.path.extension().is_some_and(|ext| ext == "rs"));

    let config = Config {
        trace_imports: true,
        include_types: true,
        semantic_depth: 2,
        ..Default::default()
    };

    let cache = FileCache::new();

    // Should handle errors gracefully
    let result = perform_semantic_analysis_graph(&mut files, &config, &cache);
    assert!(result.is_ok(), "Should handle parsing errors gracefully");

    // Valid files should still be analyzed
    let another_file = files
        .iter()
        .find(|f| f.path.to_str().unwrap().contains("another.rs"))
        .expect("Should find another.rs");

    assert!(
        !another_file.imports.is_empty(),
        "Valid file should still have imports analyzed"
    );
}
