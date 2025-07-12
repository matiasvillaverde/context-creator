//! Performance tests to verify optimization requirements

use code_digest::core::cache::FileCache;
use code_digest::core::digest::{generate_markdown, DigestOptions};
use code_digest::core::prioritizer::prioritize_files;
use code_digest::core::walker::{walk_directory, WalkOptions};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;
use tempfile::TempDir;

/// Create a test project with many files
fn create_large_test_project(base_dir: &Path, file_count: usize) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();

    // Create directory structure
    fs::create_dir_all(base_dir.join("src/core")).unwrap();
    fs::create_dir_all(base_dir.join("src/utils")).unwrap();
    fs::create_dir_all(base_dir.join("tests")).unwrap();
    fs::create_dir_all(base_dir.join("docs")).unwrap();

    // Generate files with realistic content
    for i in 0..file_count {
        let (path, content) = match i % 10 {
            0 => (
                format!("src/core/module_{}.rs", i),
                format!("//! Module {}\n\nuse std::collections::HashMap;\n\npub struct Module{} {{\n    data: HashMap<String, String>,\n}}\n\nimpl Module{} {{\n    pub fn new() -> Self {{\n        Self {{ data: HashMap::new() }}\n    }}\n}}\n", i, i, i),
            ),
            1 => (
                format!("src/utils/helper_{}.rs", i),
                format!("//! Helper {}\n\npub fn process_data(input: &str) -> String {{\n    input.trim().to_uppercase()\n}}\n\n#[cfg(test)]\nmod tests {{\n    use super::*;\n    \n    #[test]\n    fn test_process() {{\n        assert_eq!(process_data(\"hello\"), \"HELLO\");\n    }}\n}}\n", i),
            ),
            2 => (
                format!("tests/test_{}.rs", i),
                format!("#[test]\nfn test_module_{}() {{\n    let result = 2 + 2;\n    assert_eq!(result, 4);\n}}\n", i),
            ),
            3 => (
                format!("docs/doc_{}.md", i),
                format!("# Documentation {}\n\nThis is documentation for module {}.\n\n## Usage\n\n```rust\nlet module = Module::new();\n```\n", i, i),
            ),
            _ => (
                format!("src/file_{}.rs", i),
                format!("//! File {}\n\nconst DATA: &str = \"{}\";\n\npub fn get_data() -> &'static str {{\n    DATA\n}}\n", i, "x".repeat(100)),
            ),
        };

        let file_path = base_dir.join(&path);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&file_path, content).unwrap();
        files.push(file_path);
    }

    files
}

#[test]
fn test_performance_1000_files_under_1_second() {
    // Create test directory with 1000 files
    let temp_dir = TempDir::new().unwrap();
    let _files = create_large_test_project(temp_dir.path(), 1000);

    // Measure end-to-end processing time
    let start = Instant::now();

    // Walk directory
    let walk_options = WalkOptions::default();
    let files = walk_directory(temp_dir.path(), walk_options).unwrap();
    assert_eq!(files.len(), 1000, "Should find all 1000 files");

    // Create cache
    let cache = Arc::new(FileCache::new());

    // Prioritize files with token limit
    let digest_options = DigestOptions {
        max_tokens: Some(100_000),
        include_tree: true,
        include_stats: true,
        group_by_type: false,
        sort_by_priority: true,
        file_header_template: "## {path}".to_string(),
        doc_header_template: "# Code Digest".to_string(),
        include_toc: true,
    };

    let prioritized_files = prioritize_files(files, &digest_options, cache.clone()).unwrap();

    // Generate markdown
    let _markdown = generate_markdown(prioritized_files, digest_options, cache).unwrap();

    let elapsed = start.elapsed();

    // Verify performance requirement: processing 1000 files should take <1 second
    assert!(
        elapsed.as_secs_f64() < 1.0,
        "Processing 1000 files took {:.3}s, which exceeds the 1 second requirement",
        elapsed.as_secs_f64()
    );

    println!("✅ Performance test passed: 1000 files processed in {:.3}s", elapsed.as_secs_f64());
}

#[test]
fn test_cache_eliminates_redundant_io() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.txt");
    fs::write(&test_file, "test content").unwrap();

    let cache = Arc::new(FileCache::new());

    // First read - loads from disk
    let content1 = cache.get_or_load(&test_file).unwrap();
    let stats1 = cache.stats();
    assert_eq!(stats1.entries, 1);

    // Subsequent reads - should hit cache
    for _ in 0..10 {
        let content = cache.get_or_load(&test_file).unwrap();
        assert!(Arc::ptr_eq(&content1, &content), "Should return same Arc");
    }

    let stats2 = cache.stats();
    assert_eq!(stats2.entries, 1, "Cache should still have only 1 entry");

    println!("✅ Cache test passed: 10 reads returned the same Arc reference");
}
