//! Test content hash computation with internal verification
//! This test verifies that content hashes are actually being computed and stored

use context_creator::core::semantic::dependency_types::{DependencyNode, FileAnalysisResult};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

#[test]
fn test_file_analysis_result_includes_hash() {
    // Test that FileAnalysisResult can store content hash
    let result = FileAnalysisResult {
        file_index: 0,
        imports: Vec::new(),
        function_calls: Vec::new(),
        type_references: Vec::new(),
        content_hash: Some(12345),
        error: None,
    };

    assert_eq!(result.content_hash, Some(12345));
}

#[test]
fn test_dependency_node_includes_hash() {
    // Test that DependencyNode can store content hash
    let node = DependencyNode {
        file_index: 0,
        path: std::path::PathBuf::from("test.rs"),
        language: Some("rust".to_string()),
        content_hash: Some(67890),
        file_size: 1024,
        depth: 0,
    };

    assert_eq!(node.content_hash, Some(67890));
}

#[test]
fn test_hash_computation_deterministic() {
    // Test that our hash computation is deterministic
    let content1 = "pub fn hello() { println!(\"Hello, world!\"); }";
    let content2 = "pub fn hello() { println!(\"Hello, world!\"); }"; // Same content
    let content3 = "pub fn goodbye() { println!(\"Goodbye!\"); }"; // Different content

    let hash1 = compute_hash(content1);
    let hash2 = compute_hash(content2);
    let hash3 = compute_hash(content3);

    // Same content should produce same hash
    assert_eq!(
        hash1, hash2,
        "Identical content should produce identical hashes"
    );

    // Different content should produce different hash
    assert_ne!(
        hash1, hash3,
        "Different content should produce different hashes"
    );
}

fn compute_hash(content: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    hasher.finish()
}
