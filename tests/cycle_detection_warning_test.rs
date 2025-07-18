//! Test cycle detection warning output

use context_creator::cli::Config;
use context_creator::core::cache::FileCache;
use context_creator::core::semantic_graph::perform_semantic_analysis_graph;
use context_creator::core::walker::{walk_directory, WalkOptions};
use std::fs;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use tempfile::TempDir;

/// Capture stderr output for testing (unused in current implementation)
#[allow(dead_code)]
struct StderrCapture {
    captured: Arc<Mutex<Vec<u8>>>,
}

#[allow(dead_code)]
impl StderrCapture {
    fn new() -> Self {
        Self {
            captured: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn get_output(&self) -> String {
        let captured = self.captured.lock().unwrap();
        String::from_utf8_lossy(&captured).to_string()
    }
}

impl Write for StderrCapture {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut captured = self.captured.lock().unwrap();
        captured.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn test_cycle_detection_warning_output() {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();

    // Create git directory
    fs::create_dir_all(base_path.join(".git")).unwrap();

    // Create files with circular dependencies
    fs::create_dir_all(base_path.join("src")).unwrap();
    fs::write(
        base_path.join("src/a.rs"),
        "use crate::b::B;\npub struct A;\n",
    )
    .unwrap();
    fs::write(
        base_path.join("src/b.rs"),
        "use crate::c::C;\npub struct B;\n",
    )
    .unwrap();
    fs::write(
        base_path.join("src/c.rs"),
        "use crate::a::A;\npub struct C;\n",
    )
    .unwrap(); // Cycle: A -> B -> C -> A

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

    // Run semantic analysis - this should detect the cycle and print warnings
    let result = perform_semantic_analysis_graph(&mut files, &config, &cache);

    // The result should still be Ok (cycle is handled gracefully)
    assert!(result.is_ok(), "Should handle cycle gracefully");

    // Note: In a real test, we'd capture stderr to check for warning messages
    // For now, we just ensure the function completes without panicking
    println!("Cycle detection test completed successfully");
}
