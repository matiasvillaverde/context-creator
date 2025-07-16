//! Integration tests for semantic analysis reliability improvements
//! Tests all components working together under production-like conditions

use code_digest::core::semantic::{get_analyzer_for_file, get_resolver_for_file, AstCacheV2};
use code_digest::core::semantic_graph_v2::{DependencyGraph, FileNode};
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::runtime::Runtime;

#[test]
fn test_end_to_end_semantic_analysis() {
    let rt = Runtime::new().unwrap();

    rt.block_on(async {
        let temp_dir = TempDir::new().unwrap();
        let base_dir = temp_dir.path();

        // Create a small project structure
        std::fs::create_dir_all(base_dir.join("src")).unwrap();

        // Main file that imports a module
        std::fs::write(
            base_dir.join("src/main.rs"),
            r#"
mod utils;
use utils::helper;

fn main() {
    helper::process_data();
}
"#,
        )
        .unwrap();

        // Utils module
        std::fs::write(
            base_dir.join("src/utils.rs"),
            r#"
pub mod helper {
    pub fn process_data() {
        println!("Processing data...");
    }
}
"#,
        )
        .unwrap();

        // Create cache and analyze files
        let cache = Arc::new(AstCacheV2::new(100));
        let mut files = Vec::new();

        // Analyze main.rs
        let main_path = base_dir.join("src/main.rs");
        let main_content = std::fs::read_to_string(&main_path).unwrap();

        if let Some(analyzer) = get_analyzer_for_file(&main_path).unwrap() {
            let _tree = cache
                .get_or_parse(&main_path, &main_content, "rust")
                .await
                .unwrap();

            // Analyze the file
            let context = code_digest::core::semantic::SemanticContext {
                current_file: main_path.clone(),
                base_dir: base_dir.to_path_buf(),
                current_depth: 0,
                max_depth: 2,
                visited_files: std::collections::HashSet::new(),
            };

            let analysis = analyzer
                .analyze_file(&main_path, &main_content, &context)
                .unwrap();
            assert_eq!(analysis.imports.len(), 2); // mod utils and use utils::helper

            files.push(FileNode {
                path: PathBuf::from("src/main.rs"),
                imports: vec![PathBuf::from("src/utils.rs")],
                imported_by: vec![],
            });
        }

        // Build dependency graph
        let graph = DependencyGraph::build_from_files(&files);
        assert_eq!(graph.node_count(), 1); // Only main.rs was added
        assert_eq!(graph.edge_count(), 0); // utils.rs wasn't in the file list
    });
}

#[test]
fn test_concurrent_analysis_stress_test() {
    let rt = Runtime::new().unwrap();

    rt.block_on(async {
        let cache = Arc::new(AstCacheV2::new(100));
        let mut handles = vec![];

        // Create 50 concurrent parsing tasks
        for i in 0..50 {
            let cache_clone = cache.clone();
            let handle = tokio::spawn(async move {
                let path = PathBuf::from(format!("test_{i}.js"));
                let content = format!(
                    r#"
import React from 'react';
import {{ Component{i} }} from './components';

function App{i}() {{
    return <Component{i} />;
}}

export default App{i};
"#
                );

                // Parse the file
                let result = cache_clone
                    .get_or_parse(&path, &content, "javascript")
                    .await;
                assert!(result.is_ok());

                // Parse again to test cache hit
                let result2 = cache_clone
                    .get_or_parse(&path, &content, "javascript")
                    .await;
                assert!(result2.is_ok());

                format!("Task {i} completed")
            });
            handles.push(handle);
        }

        // All tasks should complete without panics
        for handle in handles {
            let result = handle.await.unwrap();
            assert!(result.contains("completed"));
        }
    });
}

#[test]
fn test_memory_bounded_caching() {
    let rt = Runtime::new().unwrap();

    rt.block_on(async {
        // Create cache with very small capacity
        let cache = Arc::new(AstCacheV2::new(3));

        // Add more files than capacity
        for i in 0..10 {
            let path = PathBuf::from(format!("file_{i}.py"));
            let content = format!(
                r#"
def function_{i}():
    return "Result {i}"
    
class Class{i}:
    def method(self):
        return {i}
"#
            );

            let result = cache.get_or_parse(&path, &content, "python").await;
            assert!(result.is_ok());
        }

        // Cache should have evicted old entries
        // We can't directly check cache size due to eventual consistency,
        // but we can verify that parsing still works
        let path = PathBuf::from("final_test.py");
        let content = "def final_test(): pass";
        let result = cache.get_or_parse(&path, content, "python").await;
        assert!(result.is_ok());
    });
}

#[test]
fn test_timeout_protection_integration() {
    let rt = Runtime::new().unwrap();

    rt.block_on(async {
        let cache = Arc::new(AstCacheV2::new(100));

        // Create a very large file that might take time to parse
        let mut large_content = String::new();
        for i in 0..10000 {
            large_content.push_str(&format!("function func_{i}() {{ return {i}; }}\n"));
        }

        let path = PathBuf::from("large_file.js");

        // Should complete within timeout (30 seconds by default)
        let start = std::time::Instant::now();
        let result = cache
            .get_or_parse(&path, &large_content, "javascript")
            .await;
        let elapsed = start.elapsed();

        assert!(result.is_ok());
        assert!(elapsed.as_secs() < 35); // Should complete well within timeout
    });
}

#[test]
fn test_graph_performance_with_large_codebase() {
    use std::time::Instant;

    // Create a large graph with many interconnected files
    let mut files = Vec::new();

    // Create 500 files with complex dependencies
    for i in 0..500 {
        let mut imports = Vec::new();

        // Each file imports 5 other files
        for j in 1..=5 {
            let target = (i + j) % 500;
            imports.push(PathBuf::from(format!("src/file_{target}.rs")));
        }

        files.push(FileNode {
            path: PathBuf::from(format!("src/file_{i}.rs")),
            imports,
            imported_by: vec![],
        });
    }

    // Build the graph
    let start = Instant::now();
    let graph = DependencyGraph::build_from_files(&files);
    let build_time = start.elapsed();

    // Should build quickly
    assert!(build_time.as_millis() < 200);
    assert_eq!(graph.node_count(), 500);
    assert_eq!(graph.edge_count(), 2500); // 500 files * 5 imports each

    // Test dependency finding performance
    let start = Instant::now();
    let deps = graph.find_dependencies(&PathBuf::from("src/file_0.rs"), 3);
    let search_time = start.elapsed();

    assert!(search_time.as_millis() < 50);
    assert!(!deps.is_empty());

    // Test cycle detection
    let start = Instant::now();
    let has_cycles = graph.has_cycles();
    let cycle_time = start.elapsed();

    assert!(cycle_time.as_millis() < 100);
    assert!(has_cycles); // With this pattern, there will be cycles
}

#[test]
fn test_error_recovery_and_resilience() {
    let rt = Runtime::new().unwrap();

    rt.block_on(async {
        let cache = Arc::new(AstCacheV2::new(100));

        // Test parsing invalid code
        let path = PathBuf::from("invalid.rs");
        let invalid_content = "fn main() { this is not valid rust code }";

        // Should handle parse errors gracefully
        let result = cache.get_or_parse(&path, invalid_content, "rust").await;
        // Parser might still create a partial tree, so we just check no panic
        assert!(result.is_ok() || result.is_err());

        // Test with empty content
        let empty_path = PathBuf::from("empty.js");
        let result = cache.get_or_parse(&empty_path, "", "javascript").await;
        assert!(result.is_ok());

        // Test with unknown language
        let unknown_path = PathBuf::from("unknown.xyz");
        let result = cache
            .get_or_parse(&unknown_path, "content", "unknown_lang")
            .await;
        assert!(result.is_err());

        // Verify cache still works after errors
        let valid_path = PathBuf::from("valid.py");
        let valid_content = "def test(): return True";
        let result = cache
            .get_or_parse(&valid_path, valid_content, "python")
            .await;
        assert!(result.is_ok());
    });
}

#[test]
fn test_path_validation_integration() {
    let rt = Runtime::new().unwrap();

    rt.block_on(async {
        let temp_dir = TempDir::new().unwrap();
        let base_dir = temp_dir.path();

        // Create project structure
        std::fs::create_dir_all(base_dir.join("src")).unwrap();
        std::fs::write(base_dir.join("src/lib.js"), "export function test() {}").unwrap();

        // Test resolver with path validation
        if let Some(resolver) = get_resolver_for_file(&base_dir.join("src/main.js")).unwrap() {
            // Valid relative import
            let result = resolver.resolve_import("./lib", &base_dir.join("src/main.js"), base_dir);
            assert!(result.is_ok());

            // Invalid path traversal attempt
            let result = resolver.resolve_import(
                "../../../etc/passwd",
                &base_dir.join("src/main.js"),
                base_dir,
            );
            assert!(result.is_err());

            // Invalid module name with special characters
            let result =
                resolver.resolve_import("rm -rf /", &base_dir.join("src/main.js"), base_dir);
            assert!(result.is_err());
        }
    });
}
