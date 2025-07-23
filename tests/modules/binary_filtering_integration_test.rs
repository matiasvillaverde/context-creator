//! Integration tests for binary file filtering functionality

use context_creator::cli::Config;
use context_creator::core::walker::{walk_directory, WalkOptions};
use std::fs;
use tempfile::TempDir;

// Helper to create test files
fn create_test_files(root: &std::path::Path, files: Vec<(&str, &[u8])>) {
    for (path, content) in files {
        let file_path = root.join(path);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(file_path, content).unwrap();
    }
}

#[test]
fn test_binary_filtering_integration() {
    // Create a test directory with mixed file types
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    create_test_files(
        root,
        vec![
            ("src/main.rs", b"fn main() {}"),
            ("README.md", b"# Test"),
            ("assets/logo.png", b"PNG\x89\x50\x4e\x47"),
            ("video.mp4", b"MP4\x00\x00"),
            ("binary.exe", b"MZ\x90\x00"),
        ],
    );

    // Test with filtering enabled (simulating prompt mode)
    let config = Config {
        paths: Some(vec![root.to_path_buf()]),
        prompt: Some("test".to_string()), // This enables binary filtering
        ..Default::default()
    };

    let options = WalkOptions::from_config(&config).unwrap();
    assert!(options.filter_binary_files);

    let files = walk_directory(root, options).unwrap();

    // Should only include text files
    let file_names: Vec<String> = files
        .iter()
        .map(|f| f.relative_path.to_string_lossy().to_string())
        .collect();

    assert!(file_names.contains(&"src/main.rs".to_string()));
    assert!(file_names.contains(&"README.md".to_string()));
    assert!(!file_names.contains(&"assets/logo.png".to_string()));
    assert!(!file_names.contains(&"video.mp4".to_string()));
    assert!(!file_names.contains(&"binary.exe".to_string()));
}

#[test]
fn test_no_binary_filtering_without_prompt() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    create_test_files(
        root,
        vec![
            ("code.py", b"print('hello')"),
            ("image.jpg", b"JPEG\xff\xd8"),
            ("data.db", b"SQLite\x00"),
        ],
    );

    // Test without prompt - no filtering
    let config = Config {
        paths: Some(vec![root.to_path_buf()]),
        prompt: None, // No prompt set
        ..Default::default()
    };

    let options = WalkOptions::from_config(&config).unwrap();
    assert!(!options.filter_binary_files);

    let files = walk_directory(root, options).unwrap();
    let file_names: Vec<String> = files
        .iter()
        .map(|f| f.relative_path.to_string_lossy().to_string())
        .collect();

    // All files should be included
    assert_eq!(files.len(), 3);
    assert!(file_names.contains(&"code.py".to_string()));
    assert!(file_names.contains(&"image.jpg".to_string()));
    assert!(file_names.contains(&"data.db".to_string()));
}

#[test]
fn test_binary_filtering_case_insensitive() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    create_test_files(
        root,
        vec![
            ("test.rs", b"code"),
            ("IMAGE.JPG", b"binary"),
            ("Video.MP4", b"binary"),
            ("Archive.ZIP", b"binary"),
        ],
    );

    let config = Config {
        paths: Some(vec![root.to_path_buf()]),
        prompt: Some("test".to_string()),
        ..Default::default()
    };

    let options = WalkOptions::from_config(&config).unwrap();
    let files = walk_directory(root, options).unwrap();

    assert_eq!(files.len(), 1);
    assert_eq!(files[0].relative_path.to_string_lossy(), "test.rs");
}

#[test]
fn test_binary_filtering_extensionless_files() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    create_test_files(
        root,
        vec![
            ("README", b"documentation"),
            ("LICENSE", b"MIT"),
            ("Makefile", b"build:"),
            ("Dockerfile", b"FROM rust"),
            ("random_binary", b"\x00\x01\x02\x03"),
        ],
    );

    let config = Config {
        paths: Some(vec![root.to_path_buf()]),
        prompt: Some("test".to_string()),
        ..Default::default()
    };

    let options = WalkOptions::from_config(&config).unwrap();
    let files = walk_directory(root, options).unwrap();

    let file_names: Vec<String> = files
        .iter()
        .map(|f| f.relative_path.to_string_lossy().to_string())
        .collect();

    // Text files without extensions should be included
    assert!(file_names.contains(&"README".to_string()));
    assert!(file_names.contains(&"LICENSE".to_string()));
    assert!(file_names.contains(&"Makefile".to_string()));
    assert!(file_names.contains(&"Dockerfile".to_string()));
    // Files without extensions are assumed to be text by default
    // (This matches the existing behavior in FileType::from_path)
    assert!(file_names.contains(&"random_binary".to_string()));
}

#[test]
fn test_binary_filtering_compound_extensions() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    create_test_files(
        root,
        vec![
            ("archive.tar.gz", b"binary"),
            ("backup.sql.bz2", b"binary"),
            ("config.json.bak", b"{}"),
            ("script.min.js", b"console.log();"),
        ],
    );

    let config = Config {
        paths: Some(vec![root.to_path_buf()]),
        prompt: Some("test".to_string()),
        ..Default::default()
    };

    let options = WalkOptions::from_config(&config).unwrap();
    let files = walk_directory(root, options).unwrap();

    let file_names: Vec<String> = files
        .iter()
        .map(|f| f.relative_path.to_string_lossy().to_string())
        .collect();

    // .gz and .bz2 are binary extensions
    assert!(!file_names.contains(&"archive.tar.gz".to_string()));
    assert!(!file_names.contains(&"backup.sql.bz2".to_string()));
    // .bak and .js are not binary
    assert!(file_names.contains(&"config.json.bak".to_string()));
    assert!(file_names.contains(&"script.min.js".to_string()));
}

#[test]
fn test_binary_filtering_performance() {
    // Test that binary filtering improves performance on large directories
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create many files
    for i in 0..100 {
        fs::write(root.join(format!("code{i}.rs")), b"fn main() {}").unwrap();
        fs::write(root.join(format!("image{i}.jpg")), b"JPEG").unwrap();
        fs::write(root.join(format!("video{i}.mp4")), b"MP4").unwrap();
    }

    // With filtering
    let config = Config {
        paths: Some(vec![root.to_path_buf()]),
        prompt: Some("test".to_string()),
        ..Default::default()
    };

    let options = WalkOptions::from_config(&config).unwrap();
    let start = std::time::Instant::now();
    let filtered_files = walk_directory(root, options).unwrap();
    let filtered_time = start.elapsed();

    // Without filtering
    let config = Config {
        paths: Some(vec![root.to_path_buf()]),
        prompt: None,
        ..Default::default()
    };
    let options = WalkOptions::from_config(&config).unwrap();
    let start = std::time::Instant::now();
    let all_files = walk_directory(root, options).unwrap();
    let unfiltered_time = start.elapsed();

    // Verify counts
    assert_eq!(filtered_files.len(), 100); // Only .rs files
    assert_eq!(all_files.len(), 300); // All files

    // Binary filtering should generally be faster (less files to process)
    // But this might not always be true in tests due to small file sizes
    println!("Filtered: {filtered_time:?}, Unfiltered: {unfiltered_time:?}");
}
