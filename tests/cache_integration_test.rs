//! Integration tests for FileCache in the processing pipeline

use code_digest::core::cache::FileCache;
use std::fs;
use std::sync::Arc;
use tempfile::TempDir;

#[test]
fn test_single_file_read_per_file() {
    // Create test files
    let temp_dir = TempDir::new().unwrap();
    let test_files = create_test_files(&temp_dir, 10);

    // Create a shared cache
    let cache = Arc::new(FileCache::new());

    // Simulate multiple components reading files
    for file_path in &test_files {
        // Simulate walker reading
        let _content1 = cache.get_or_load(file_path).unwrap();

        // Simulate token counter reading
        let _content2 = cache.get_or_load(file_path).unwrap();

        // Simulate digest generator reading
        let _content3 = cache.get_or_load(file_path).unwrap();
    }

    // Each file should only be in cache once
    assert_eq!(cache.stats().entries, test_files.len());
}

#[test]
fn test_cache_prevents_redundant_io() {
    // This test verifies the cache behavior
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    fs::write(&file_path, "test content").unwrap();

    let cache = FileCache::new();

    // First read
    let content1 = cache.get_or_load(&file_path).unwrap();

    // Subsequent reads should return same Arc
    for _ in 0..10 {
        let content = cache.get_or_load(&file_path).unwrap();
        assert!(Arc::ptr_eq(&content1, &content));
    }
}

fn create_test_files(temp_dir: &TempDir, count: usize) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();

    for i in 0..count {
        let file_path = temp_dir.path().join(format!("file_{}.txt", i));
        fs::write(&file_path, format!("Content of file {}", i)).unwrap();
        files.push(file_path);
    }

    files
}
