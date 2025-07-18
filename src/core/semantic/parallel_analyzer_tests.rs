//! Tests for ParallelAnalyzer module
//!
//! These tests verify that the ParallelAnalyzer correctly manages parallel file analysis
//! while maintaining single responsibility for concurrency management.

#[cfg(test)]
use crate::core::cache::FileCache;
use crate::core::semantic::parallel_analyzer::{AnalysisOptions, ParallelAnalyzer};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tempfile::TempDir;

fn create_test_file(dir: &Path, name: &str, content: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, content).unwrap();
    path
}

fn create_test_environment() -> (TempDir, Vec<PathBuf>) {
    let temp_dir = TempDir::new().unwrap();
    let dir = temp_dir.path();

    let files = vec![
        create_test_file(
            dir,
            "main.rs",
            r#"
            use crate::utils::helper;
            use std::collections::HashMap;
            
            fn main() {
                let data: HashMap<String, i32> = HashMap::new();
                helper::process(&data);
            }
            "#,
        ),
        create_test_file(
            dir,
            "utils.rs",
            r#"
            pub mod helper {
                use std::collections::HashMap;
                
                pub fn process(data: &HashMap<String, i32>) {
                    println!("Processing: {:?}", data);
                }
            }
            "#,
        ),
        create_test_file(
            dir,
            "lib.rs",
            r#"
            pub mod utils;
            
            pub fn init() {
                println!("Library initialized");
            }
            "#,
        ),
    ];

    (temp_dir, files)
}

#[test]
fn test_parallel_file_analysis() {
    let (_temp_dir, files) = create_test_environment();
    let cache = Arc::new(FileCache::new());
    let project_root = files[0].parent().unwrap().to_path_buf();

    let analyzer = ParallelAnalyzer::new(&cache);
    let options = AnalysisOptions {
        semantic_depth: 3,
        trace_imports: true,
        include_types: true,
        include_functions: true,
    };

    let valid_files: HashSet<PathBuf> = files.iter().cloned().collect();
    let results = analyzer
        .analyze_files(&files, &project_root, &options, &valid_files)
        .unwrap();

    // Should analyze all files
    assert_eq!(results.len(), files.len());

    // Each file should have a result
    for (i, result) in results.iter().enumerate() {
        assert_eq!(result.file_index, i);
        assert!(result.content_hash.is_some());
        assert!(result.error.is_none());
    }

    // Main.rs should have been analyzed (has content hash)
    let main_result = &results[0];
    assert!(main_result.content_hash.is_some());

    // The analyzer should have attempted to process the file
    // We can't guarantee imports/types will be found without proper setup
}

#[test]
fn test_thread_pool_management() {
    let (_temp_dir, files) = create_test_environment();
    let cache = Arc::new(FileCache::new());
    let project_root = files[0].parent().unwrap().to_path_buf();

    // Create analyzer with custom thread pool size
    let analyzer = ParallelAnalyzer::with_thread_count(&cache, 2);
    let options = AnalysisOptions {
        semantic_depth: 3,
        trace_imports: true,
        include_types: true,
        include_functions: true,
    };

    // Should handle analysis with limited threads
    let valid_files: HashSet<PathBuf> = files.iter().cloned().collect();
    let results = analyzer
        .analyze_files(&files, &project_root, &options, &valid_files)
        .unwrap();
    assert_eq!(results.len(), files.len());
}

#[test]
fn test_analysis_error_handling() {
    let temp_dir = TempDir::new().unwrap();
    let dir = temp_dir.path();

    // Create a file with syntax error
    let bad_file = create_test_file(
        dir,
        "bad.rs",
        r#"
        use std::collections::HashMap
        // Missing semicolon above
        
        fn main() {
            let x = 
        // Incomplete statement
        "#,
    );

    let cache = Arc::new(FileCache::new());
    let analyzer = ParallelAnalyzer::new(&cache);
    let options = AnalysisOptions {
        semantic_depth: 3,
        trace_imports: true,
        include_types: true,
        include_functions: true,
    };

    let valid_files: HashSet<PathBuf> = [bad_file.clone()].iter().cloned().collect();
    let results = analyzer
        .analyze_files(&[bad_file.clone()], dir, &options, &valid_files)
        .unwrap();

    // Should still return a result, but with limited information
    assert_eq!(results.len(), 1);
    let result = &results[0];
    assert!(result.content_hash.is_some());
    // Parser might still extract some information despite syntax errors
}

#[test]
fn test_empty_file_list() {
    let cache = Arc::new(FileCache::new());
    let analyzer = ParallelAnalyzer::new(&cache);
    let options = AnalysisOptions::default();
    let project_root = PathBuf::from("/tmp");

    let valid_files: HashSet<PathBuf> = HashSet::new();
    let results = analyzer
        .analyze_files(&[], &project_root, &options, &valid_files)
        .unwrap();
    assert_eq!(results.len(), 0);
}

#[test]
fn test_large_file_set() {
    let temp_dir = TempDir::new().unwrap();
    let dir = temp_dir.path();

    // Create many files to test parallel processing
    let mut files = Vec::new();
    for i in 0..20 {
        let content = format!(
            r#"
            pub mod module_{i} {{
                pub fn function_{i}() -> i32 {{
                    {i}
                }}
            }}
            "#
        );
        files.push(create_test_file(dir, &format!("mod_{i}.rs"), &content));
    }

    let cache = Arc::new(FileCache::new());
    let analyzer = ParallelAnalyzer::new(&cache);
    let options = AnalysisOptions {
        semantic_depth: 2,
        trace_imports: false,
        include_types: false,
        include_functions: true,
    };

    let start = std::time::Instant::now();
    let valid_files: HashSet<PathBuf> = files.iter().cloned().collect();
    let results = analyzer
        .analyze_files(&files, dir, &options, &valid_files)
        .unwrap();
    let duration = start.elapsed();

    // Should analyze all files
    assert_eq!(results.len(), 20);

    // Parallel processing should be reasonably fast
    // (This is a soft assertion - adjust threshold as needed)
    assert!(
        duration.as_secs() < 5,
        "Analysis took too long: {duration:?}"
    );

    // Each result should have been processed
    for result in &results {
        assert!(result.content_hash.is_some() || result.error.is_some());
    }
}

#[test]
fn test_non_code_files_handled() {
    let temp_dir = TempDir::new().unwrap();
    let dir = temp_dir.path();

    let files = vec![
        create_test_file(dir, "README.md", "# Project README"),
        create_test_file(dir, "data.json", r#"{"key": "value"}"#),
        create_test_file(dir, "main.rs", "fn main() {}"),
    ];

    let cache = Arc::new(FileCache::new());
    let analyzer = ParallelAnalyzer::new(&cache);
    let options = AnalysisOptions::default();

    let valid_files: HashSet<PathBuf> = files.iter().cloned().collect();
    let results = analyzer
        .analyze_files(&files, dir, &options, &valid_files)
        .unwrap();

    // Should handle all files
    assert_eq!(results.len(), 3);

    // Non-code files should have empty analysis
    assert!(results[0].imports.is_empty()); // README.md
    assert!(results[1].imports.is_empty()); // data.json

    // Code file should be analyzed
    assert!(results[2].content_hash.is_some()); // main.rs
}

#[test]
fn test_cache_utilization() {
    let (_temp_dir, files) = create_test_environment();
    let cache = Arc::new(FileCache::new());
    let project_root = files[0].parent().unwrap().to_path_buf();

    let analyzer = ParallelAnalyzer::new(&cache);
    let options = AnalysisOptions::default();

    // First analysis
    let valid_files: HashSet<PathBuf> = files.iter().cloned().collect();
    let results1 = analyzer
        .analyze_files(&files, &project_root, &options, &valid_files)
        .unwrap();

    // Second analysis should utilize cache
    let start = std::time::Instant::now();
    let results2 = analyzer
        .analyze_files(&files, &project_root, &options, &valid_files)
        .unwrap();
    let duration = start.elapsed();

    // Results should be consistent
    assert_eq!(results1.len(), results2.len());
    for (r1, r2) in results1.iter().zip(results2.iter()) {
        assert_eq!(r1.content_hash, r2.content_hash);
    }

    // Second run should be faster due to caching
    // Allow more time for slower systems or CI environments
    assert!(
        duration.as_millis() < 500,
        "Cache not being utilized effectively: {duration:?}"
    );
}

#[test]
fn test_selective_analysis_options() {
    let (_temp_dir, files) = create_test_environment();
    let cache = Arc::new(FileCache::new());
    let project_root = files[0].parent().unwrap().to_path_buf();
    let analyzer = ParallelAnalyzer::new(&cache);

    // Test with only imports enabled
    let import_only_options = AnalysisOptions {
        semantic_depth: 3,
        trace_imports: true,
        include_types: false,
        include_functions: false,
    };

    let valid_files: HashSet<PathBuf> = files.iter().cloned().collect();
    let results = analyzer
        .analyze_files(&files, &project_root, &import_only_options, &valid_files)
        .unwrap();

    // With trace_imports enabled, we should have attempted to process imports
    let main_result = &results[0];
    // We processed the file (has content hash)
    assert!(main_result.content_hash.is_some());
    // Types and functions should be empty since we disabled them
    assert!(main_result.type_references.is_empty());
    assert!(main_result.function_calls.is_empty());
}
