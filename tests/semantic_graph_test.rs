//! Tests for optimized semantic graph algorithms

use code_digest::core::semantic_graph_v2::{DependencyGraph, FileNode};
use std::path::PathBuf;

#[test]
fn test_graph_construction() {
    let files = vec![
        FileNode {
            path: PathBuf::from("main.rs"),
            imports: vec![PathBuf::from("lib.rs"), PathBuf::from("utils.rs")],
            imported_by: vec![],
        },
        FileNode {
            path: PathBuf::from("lib.rs"),
            imports: vec![PathBuf::from("utils.rs")],
            imported_by: vec![],
        },
        FileNode {
            path: PathBuf::from("utils.rs"),
            imports: vec![],
            imported_by: vec![],
        },
    ];

    let graph = DependencyGraph::build_from_files(&files);

    // Check graph structure
    assert_eq!(graph.node_count(), 3);
    assert_eq!(graph.edge_count(), 3); // main->lib, main->utils, lib->utils
}

#[test]
fn test_find_dependencies() {
    let files = vec![
        FileNode {
            path: PathBuf::from("a.rs"),
            imports: vec![PathBuf::from("b.rs")],
            imported_by: vec![],
        },
        FileNode {
            path: PathBuf::from("b.rs"),
            imports: vec![PathBuf::from("c.rs")],
            imported_by: vec![],
        },
        FileNode {
            path: PathBuf::from("c.rs"),
            imports: vec![PathBuf::from("d.rs")],
            imported_by: vec![],
        },
        FileNode {
            path: PathBuf::from("d.rs"),
            imports: vec![],
            imported_by: vec![],
        },
    ];

    let graph = DependencyGraph::build_from_files(&files);

    // Test finding dependencies with depth limit
    let deps_1 = graph.find_dependencies(&PathBuf::from("a.rs"), 1);
    assert_eq!(deps_1.len(), 2); // a.rs itself + b.rs

    let deps_2 = graph.find_dependencies(&PathBuf::from("a.rs"), 2);
    assert_eq!(deps_2.len(), 3); // a.rs + b.rs + c.rs

    let deps_all = graph.find_dependencies(&PathBuf::from("a.rs"), 10);
    assert_eq!(deps_all.len(), 4); // All files
}

#[test]
fn test_circular_dependency_detection() {
    let files = vec![
        FileNode {
            path: PathBuf::from("a.rs"),
            imports: vec![PathBuf::from("b.rs")],
            imported_by: vec![],
        },
        FileNode {
            path: PathBuf::from("b.rs"),
            imports: vec![PathBuf::from("c.rs")],
            imported_by: vec![],
        },
        FileNode {
            path: PathBuf::from("c.rs"),
            imports: vec![PathBuf::from("a.rs")], // Creates cycle
            imported_by: vec![],
        },
    ];

    let graph = DependencyGraph::build_from_files(&files);

    // Check for cycles
    assert!(graph.has_cycles());

    let cycles = graph.find_cycles();
    assert_eq!(cycles.len(), 1);
    assert_eq!(cycles[0].len(), 3); // a -> b -> c -> a
}

#[test]
fn test_performance_with_large_graph() {
    use std::time::Instant;

    // Create a large graph with 1000 files
    let mut files = Vec::new();
    for i in 0..1000 {
        let mut imports = Vec::new();
        // Each file imports the next 3 files (if they exist)
        for j in 1..=3 {
            if i + j < 1000 {
                imports.push(PathBuf::from(format!("file_{}.rs", i + j)));
            }
        }

        files.push(FileNode {
            path: PathBuf::from(format!("file_{}.rs", i)),
            imports,
            imported_by: vec![],
        });
    }

    let start = Instant::now();
    let graph = DependencyGraph::build_from_files(&files);
    let build_time = start.elapsed();

    // Building should be fast (under 100ms for 1000 files)
    assert!(build_time.as_millis() < 100);

    // Finding dependencies should also be fast
    let start = Instant::now();
    let deps = graph.find_dependencies(&PathBuf::from("file_0.rs"), 5);
    let search_time = start.elapsed();

    assert!(search_time.as_millis() < 10);
    assert!(!deps.is_empty());
}
