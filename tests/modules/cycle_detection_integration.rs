//! Integration tests for cycle detection in dependency graphs

use context_creator::core::semantic::cycle_detector::{CycleResolution, TarjanCycleDetector};
use petgraph::graph::DiGraph;
use std::collections::HashMap;
use std::path::PathBuf;

/// Mock node type for testing file dependencies
#[derive(Debug, Clone)]
struct FileNode {
    path: PathBuf,
    imports: Vec<String>,
}

/// Create a test graph representing file dependencies
fn create_file_dependency_graph() -> (
    DiGraph<FileNode, ()>,
    HashMap<String, petgraph::graph::NodeIndex>,
) {
    let mut graph = DiGraph::new();
    let mut name_to_node = HashMap::new();

    // Create nodes
    let auth_node = graph.add_node(FileNode {
        path: PathBuf::from("src/auth/mod.rs"),
        imports: vec!["database".to_string(), "user".to_string()],
    });
    name_to_node.insert("auth".to_string(), auth_node);

    let database_node = graph.add_node(FileNode {
        path: PathBuf::from("src/database/mod.rs"),
        imports: vec!["user".to_string()],
    });
    name_to_node.insert("database".to_string(), database_node);

    let user_node = graph.add_node(FileNode {
        path: PathBuf::from("src/user/mod.rs"),
        imports: vec!["auth".to_string()], // This creates a cycle
    });
    name_to_node.insert("user".to_string(), user_node);

    let api_node = graph.add_node(FileNode {
        path: PathBuf::from("src/api/mod.rs"),
        imports: vec!["auth".to_string()],
    });
    name_to_node.insert("api".to_string(), api_node);

    // Add edges based on imports
    let _ = graph.add_edge(auth_node, database_node, ());
    let _ = graph.add_edge(auth_node, user_node, ());
    let _ = graph.add_edge(database_node, user_node, ());
    let _ = graph.add_edge(user_node, auth_node, ()); // Creates cycle: auth → user → auth
    let _ = graph.add_edge(api_node, auth_node, ());

    (graph, name_to_node)
}

#[test]
fn test_real_world_cycle_detection() {
    let (graph, name_to_node) = create_file_dependency_graph();
    let mut detector = TarjanCycleDetector::new();
    let result = detector.detect_cycles(&graph);

    assert!(result.has_cycles, "Should detect circular dependency");
    assert_eq!(result.cycles.len(), 1, "Should find exactly one cycle");

    // Verify the cycle contains auth and user nodes
    let cycle = &result.cycles[0];
    assert!(cycle.contains(&name_to_node["auth"]));
    assert!(cycle.contains(&name_to_node["user"]));
}

#[test]
fn test_cycle_breaking_strategy() {
    let (graph, _) = create_file_dependency_graph();
    let mut detector = TarjanCycleDetector::new();
    let result = detector.detect_cycles(&graph);
    let resolution = detector.handle_cycles(&graph, result.cycles);

    match resolution {
        CycleResolution::PartialOrder(order) => {
            // All nodes should be included in the partial order
            assert_eq!(order.len(), graph.node_count());

            // Verify that nodes appear only once
            let mut seen = std::collections::HashSet::new();
            for node in &order {
                assert!(seen.insert(node), "Node appears twice in partial order");
            }
        }
        _ => panic!("Expected PartialOrder resolution"),
    }
}

#[test]
fn test_complex_multi_cycle_scenario() {
    let mut graph = DiGraph::new();
    let mut name_to_node = HashMap::new();

    // Create a more complex scenario with multiple intertwined cycles
    // Module A imports B and C
    let a = graph.add_node(FileNode {
        path: PathBuf::from("src/module_a.rs"),
        imports: vec!["module_b".to_string(), "module_c".to_string()],
    });
    name_to_node.insert("module_a".to_string(), a);

    // Module B imports D
    let b = graph.add_node(FileNode {
        path: PathBuf::from("src/module_b.rs"),
        imports: vec!["module_d".to_string()],
    });
    name_to_node.insert("module_b".to_string(), b);

    // Module C imports D
    let c = graph.add_node(FileNode {
        path: PathBuf::from("src/module_c.rs"),
        imports: vec!["module_d".to_string()],
    });
    name_to_node.insert("module_c".to_string(), c);

    // Module D imports A (creating cycle)
    let d = graph.add_node(FileNode {
        path: PathBuf::from("src/module_d.rs"),
        imports: vec!["module_a".to_string()],
    });
    name_to_node.insert("module_d".to_string(), d);

    // Module E imports F
    let e = graph.add_node(FileNode {
        path: PathBuf::from("src/module_e.rs"),
        imports: vec!["module_f".to_string()],
    });
    name_to_node.insert("module_e".to_string(), e);

    // Module F imports E (creating another cycle)
    let f = graph.add_node(FileNode {
        path: PathBuf::from("src/module_f.rs"),
        imports: vec!["module_e".to_string()],
    });
    name_to_node.insert("module_f".to_string(), f);

    // Add edges
    let _ = graph.add_edge(a, b, ());
    let _ = graph.add_edge(a, c, ());
    let _ = graph.add_edge(b, d, ());
    let _ = graph.add_edge(c, d, ());
    let _ = graph.add_edge(d, a, ()); // Cycle: A → B → D → A and A → C → D → A
    let _ = graph.add_edge(e, f, ());
    let _ = graph.add_edge(f, e, ()); // Cycle: E → F → E

    let mut detector = TarjanCycleDetector::new();
    let result = detector.detect_cycles(&graph);

    assert!(result.has_cycles, "Should detect cycles");
    // Should find 2 separate cycles
    assert_eq!(result.cycles.len(), 2, "Should find two separate cycles");

    // Verify cycle sizes
    let cycle_sizes: Vec<usize> = result.cycles.iter().map(|c| c.len()).collect();
    assert!(cycle_sizes.contains(&2), "Should have E-F cycle (size 2)");
    assert!(
        cycle_sizes.contains(&4),
        "Should have A-B-C-D cycle (size 4)"
    );
}

#[test]
fn test_cycle_reporting() {
    let (graph, _name_to_node) = create_file_dependency_graph();
    let mut detector = TarjanCycleDetector::new();
    let result = detector.detect_cycles(&graph);

    // Test that cycle details are properly populated
    assert!(!result.cycle_details.is_empty());

    for detail in &result.cycle_details {
        assert!(!detail.nodes.is_empty());
        assert!(!detail.description.is_empty());
        assert!(detail.description.contains("Cycle"));
        assert!(detail.description.contains("nodes"));
    }

    // Verify we can access the actual file paths from the cycle
    for cycle in &result.cycles {
        for &node_idx in cycle {
            let file_node = &graph[node_idx];
            // Verify the path field is accessible
            assert!(!file_node.path.to_string_lossy().is_empty());
            // Verify we can iterate over imports (shows the field is used)
            for import in &file_node.imports {
                assert!(!import.is_empty());
            }
        }
    }
}

#[test]
fn test_no_cycles_in_dag() {
    let mut graph = DiGraph::new();

    // Create a proper DAG structure
    let root = graph.add_node(FileNode {
        path: PathBuf::from("src/main.rs"),
        imports: vec!["lib".to_string()],
    });

    let lib = graph.add_node(FileNode {
        path: PathBuf::from("src/lib.rs"),
        imports: vec!["utils".to_string(), "config".to_string()],
    });

    let utils = graph.add_node(FileNode {
        path: PathBuf::from("src/utils.rs"),
        imports: vec!["config".to_string()],
    });

    let config = graph.add_node(FileNode {
        path: PathBuf::from("src/config.rs"),
        imports: vec![],
    });

    // Add edges (no cycles)
    let _ = graph.add_edge(root, lib, ());
    let _ = graph.add_edge(lib, utils, ());
    let _ = graph.add_edge(lib, config, ());
    let _ = graph.add_edge(utils, config, ());

    let mut detector = TarjanCycleDetector::new();
    let result = detector.detect_cycles(&graph);

    assert!(!result.has_cycles, "DAG should have no cycles");
    assert_eq!(result.cycles.len(), 0);

    // Test that we can get a proper topological order
    let resolution = detector.handle_cycles(&graph, result.cycles);
    match resolution {
        CycleResolution::PartialOrder(order) => {
            assert_eq!(order.len(), 4);
            // Verify the order respects dependencies
            // root should come before lib
            let root_pos = order.iter().position(|&n| n == root).unwrap();
            let lib_pos = order.iter().position(|&n| n == lib).unwrap();
            assert!(root_pos < lib_pos, "root should come before lib");

            // lib should come before its dependencies were visited (but order within dependencies may vary)
            // This is a valid topological order as long as all edges are respected
        }
        _ => panic!("Expected PartialOrder for DAG"),
    }
}
