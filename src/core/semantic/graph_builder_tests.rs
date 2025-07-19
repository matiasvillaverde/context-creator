//! Tests for GraphBuilder module
//!
//! These tests verify that the GraphBuilder correctly constructs dependency graphs
//! from file information, maintaining single responsibility for graph construction.

#[cfg(test)]
use crate::core::semantic::dependency_types::DependencyEdgeType;
use crate::core::semantic::graph_builder::GraphBuilder;
use crate::core::walker::FileInfo;
use crate::utils::file_ext::FileType;
use petgraph::visit::EdgeRef;
use std::path::PathBuf;

fn create_test_file_info(path: &str, size: u64) -> FileInfo {
    FileInfo {
        path: PathBuf::from(path),
        relative_path: PathBuf::from(path),
        size,
        file_type: FileType::Rust,
        priority: 1.0,
        imports: Vec::new(),
        imported_by: Vec::new(),
        function_calls: Vec::new(),
        type_references: Vec::new(),
        exported_functions: Vec::new(),
    }
}

#[test]
fn test_build_graph_from_files() {
    let files = vec![
        create_test_file_info("src/main.rs", 1000),
        create_test_file_info("src/lib.rs", 2000),
        create_test_file_info("src/utils.rs", 500),
    ];

    let builder = GraphBuilder::new();
    let (graph, node_map) = builder.build(&files).unwrap();

    // Verify graph structure
    assert_eq!(graph.node_count(), 3);
    assert_eq!(node_map.len(), 3);

    // Verify nodes contain correct information
    for (path, &node_idx) in &node_map {
        let node = &graph[node_idx];
        assert_eq!(&node.path, path);
        assert!(node.language.is_some());
    }
}

#[test]
fn test_add_dependency_edge() {
    let files = vec![
        create_test_file_info("src/main.rs", 1000),
        create_test_file_info("src/lib.rs", 2000),
    ];

    let builder = GraphBuilder::new();
    let (mut graph, node_map) = builder.build(&files).unwrap();

    // Add an import edge
    let main_idx = node_map[&PathBuf::from("src/main.rs")];
    let lib_idx = node_map[&PathBuf::from("src/lib.rs")];

    let edge_type = DependencyEdgeType::Import {
        symbols: vec!["foo".to_string(), "bar".to_string()],
    };

    builder.add_edge(&mut graph, main_idx, lib_idx, edge_type.clone());

    // Verify edge was added
    assert_eq!(graph.edge_count(), 1);
    let edge = graph.edges(main_idx).next().unwrap();
    assert_eq!(edge.target(), lib_idx);

    match edge.weight() {
        DependencyEdgeType::Import { symbols } => {
            assert_eq!(symbols.len(), 2);
            assert!(symbols.contains(&"foo".to_string()));
            assert!(symbols.contains(&"bar".to_string()));
        }
        _ => panic!("Expected Import edge type"),
    }
}

#[test]
fn test_graph_structure_integrity() {
    let mut files = vec![
        create_test_file_info("src/main.rs", 1000),
        create_test_file_info("src/module/mod.rs", 500),
        create_test_file_info("src/module/submod.rs", 300),
    ];

    // Set up imports to create dependencies
    files[0].imports.push(PathBuf::from("src/module/mod.rs"));
    files[1].imports.push(PathBuf::from("src/module/submod.rs"));

    let builder = GraphBuilder::new();
    let (graph, node_map) = builder.build(&files).unwrap();

    // Build edges based on imports
    let mut graph_with_edges = graph.clone();
    builder.build_edges_from_imports(&mut graph_with_edges, &files, &node_map);

    // Verify all nodes are present
    assert_eq!(graph_with_edges.node_count(), 3);

    // Verify edges were created based on imports
    assert!(graph_with_edges.edge_count() >= 2);

    // Verify no self-loops
    for node_idx in graph_with_edges.node_indices() {
        let has_self_loop = graph_with_edges
            .edges(node_idx)
            .any(|edge| edge.target() == node_idx);
        assert!(!has_self_loop, "Graph should not have self-loops");
    }
}

#[test]
fn test_language_detection() {
    let files = vec![
        create_test_file_info("src/main.rs", 1000),
        create_test_file_info("scripts/build.py", 500),
        create_test_file_info("web/app.js", 2000),
        create_test_file_info("web/types.ts", 1500),
    ];

    let builder = GraphBuilder::new();
    let (graph, _) = builder.build(&files).unwrap();

    // Verify language detection
    for node in graph.node_weights() {
        match node.path.to_str().unwrap() {
            path if path.ends_with(".rs") => {
                assert_eq!(node.language.as_deref(), Some("rust"));
            }
            path if path.ends_with(".py") => {
                assert_eq!(node.language.as_deref(), Some("python"));
            }
            path if path.ends_with(".js") => {
                assert_eq!(node.language.as_deref(), Some("javascript"));
            }
            path if path.ends_with(".ts") => {
                assert_eq!(node.language.as_deref(), Some("typescript"));
            }
            _ => {}
        }
    }
}

#[test]
fn test_empty_file_list() {
    let files = vec![];
    let builder = GraphBuilder::new();
    let (graph, node_map) = builder.build(&files).unwrap();

    assert_eq!(graph.node_count(), 0);
    assert_eq!(node_map.len(), 0);
}

#[test]
fn test_duplicate_paths_handled() {
    let files = vec![
        create_test_file_info("src/main.rs", 1000),
        create_test_file_info("src/main.rs", 2000), // Duplicate path
        create_test_file_info("src/lib.rs", 1500),
    ];

    let builder = GraphBuilder::new();
    let (graph, node_map) = builder.build(&files).unwrap();

    // Should handle duplicates gracefully
    assert_eq!(graph.node_count(), 3); // All files included
    assert_eq!(node_map.len(), 2); // But only 2 unique paths in map
}

#[test]
fn test_circular_dependency_creation() {
    let mut files = vec![
        create_test_file_info("src/a.rs", 1000),
        create_test_file_info("src/b.rs", 1000),
        create_test_file_info("src/c.rs", 1000),
    ];

    // Create circular dependency: A -> B -> C -> A
    files[0].imports.push(PathBuf::from("src/b.rs"));
    files[1].imports.push(PathBuf::from("src/c.rs"));
    files[2].imports.push(PathBuf::from("src/a.rs"));

    let builder = GraphBuilder::new();
    let (mut graph, node_map) = builder.build(&files).unwrap();
    builder.build_edges_from_imports(&mut graph, &files, &node_map);

    // Verify the circular dependency exists
    assert_eq!(graph.edge_count(), 3);

    // The builder should create the edges, cycle detection is GraphTraverser's responsibility
    let a_idx = node_map[&PathBuf::from("src/a.rs")];
    let b_idx = node_map[&PathBuf::from("src/b.rs")];
    let c_idx = node_map[&PathBuf::from("src/c.rs")];

    // Verify edges exist
    assert!(graph.find_edge(a_idx, b_idx).is_some());
    assert!(graph.find_edge(b_idx, c_idx).is_some());
    assert!(graph.find_edge(c_idx, a_idx).is_some());
}
