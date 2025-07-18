//! Tests for GraphTraverser module
//!
//! These tests verify that the GraphTraverser correctly traverses dependency graphs
//! using various algorithms while maintaining single responsibility for traversal logic.

#[cfg(test)]
use crate::core::semantic::dependency_types::{DependencyEdgeType, DependencyNode as RichNode};
use crate::core::semantic::graph_traverser::{GraphTraverser, TraversalOptions};
use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::HashSet;
use std::path::PathBuf;

fn create_test_node(path: &str, index: usize) -> RichNode {
    RichNode {
        file_index: index,
        path: PathBuf::from(path),
        language: Some("rust".to_string()),
        content_hash: None,
        file_size: 1000,
        depth: 0,
    }
}

fn create_simple_graph() -> (DiGraph<RichNode, DependencyEdgeType>, Vec<NodeIndex>) {
    let mut graph = DiGraph::new();

    let node_a = graph.add_node(create_test_node("src/a.rs", 0));
    let node_b = graph.add_node(create_test_node("src/b.rs", 1));
    let node_c = graph.add_node(create_test_node("src/c.rs", 2));
    let node_d = graph.add_node(create_test_node("src/d.rs", 3));

    // Create dependencies: A -> B -> C, A -> D
    graph.add_edge(
        node_a,
        node_b,
        DependencyEdgeType::Import { symbols: vec![] },
    );
    graph.add_edge(
        node_b,
        node_c,
        DependencyEdgeType::Import { symbols: vec![] },
    );
    graph.add_edge(
        node_a,
        node_d,
        DependencyEdgeType::Import { symbols: vec![] },
    );

    (graph, vec![node_a, node_b, node_c, node_d])
}

#[test]
fn test_bfs_traversal() {
    let (graph, nodes) = create_simple_graph();
    let traverser = GraphTraverser::new();

    let options = TraversalOptions {
        max_depth: 3,
        include_types: true,
        include_functions: true,
    };

    let visited = traverser.traverse_bfs(&graph, nodes[0], &options);

    // BFS from A should visit all nodes
    assert_eq!(visited.len(), 4);

    // Verify BFS order (level by level)
    let order: Vec<_> = visited.into_iter().collect();
    assert_eq!(order[0], nodes[0]); // A first

    // B and D should be visited before C (they're at depth 1)
    let depth_1: HashSet<_> = vec![nodes[1], nodes[3]].into_iter().collect();
    assert!(depth_1.contains(&order[1]));
    assert!(depth_1.contains(&order[2]));

    // C should be last (depth 2)
    assert_eq!(order[3], nodes[2]);
}

#[test]
fn test_dfs_traversal() {
    let (graph, nodes) = create_simple_graph();
    let traverser = GraphTraverser::new();

    let options = TraversalOptions {
        max_depth: 3,
        include_types: true,
        include_functions: true,
    };

    let visited = traverser.traverse_dfs(&graph, nodes[0], &options);

    // DFS from A should visit all nodes
    assert_eq!(visited.len(), 4);
    assert!(visited.contains(&nodes[0]));
    assert!(visited.contains(&nodes[1]));
    assert!(visited.contains(&nodes[2]));
    assert!(visited.contains(&nodes[3]));
}

#[test]
fn test_topological_sort() {
    let (graph, nodes) = create_simple_graph();
    let traverser = GraphTraverser::new();

    let sorted = traverser.topological_sort(&graph).unwrap();

    // Should include all nodes
    assert_eq!(sorted.len(), 4);

    // Verify topological order constraints
    let pos_a = sorted.iter().position(|&n| n == nodes[0]).unwrap();
    let pos_b = sorted.iter().position(|&n| n == nodes[1]).unwrap();
    let pos_c = sorted.iter().position(|&n| n == nodes[2]).unwrap();
    let pos_d = sorted.iter().position(|&n| n == nodes[3]).unwrap();

    // A must come before B and D
    assert!(pos_a < pos_b);
    assert!(pos_a < pos_d);

    // B must come before C
    assert!(pos_b < pos_c);
}

#[test]
fn test_find_reachable_nodes() {
    let (graph, nodes) = create_simple_graph();
    let traverser = GraphTraverser::new();

    // From A, all nodes are reachable
    let reachable_from_a = traverser.find_reachable_nodes(&graph, nodes[0]);
    assert_eq!(reachable_from_a.len(), 4);

    // From B, only B and C are reachable
    let reachable_from_b = traverser.find_reachable_nodes(&graph, nodes[1]);
    assert_eq!(reachable_from_b.len(), 2);
    assert!(reachable_from_b.contains(&nodes[1]));
    assert!(reachable_from_b.contains(&nodes[2]));

    // From C, only C is reachable (no outgoing edges)
    let reachable_from_c = traverser.find_reachable_nodes(&graph, nodes[2]);
    assert_eq!(reachable_from_c.len(), 1);
    assert!(reachable_from_c.contains(&nodes[2]));
}

#[test]
fn test_max_depth_limit() {
    let (graph, nodes) = create_simple_graph();
    let traverser = GraphTraverser::new();

    // Limit depth to 1
    let options = TraversalOptions {
        max_depth: 1,
        include_types: true,
        include_functions: true,
    };

    let visited = traverser.traverse_bfs(&graph, nodes[0], &options);

    // Should only visit A, B, and D (not C which is at depth 2)
    assert_eq!(visited.len(), 3);
    assert!(visited.contains(&nodes[0])); // A
    assert!(visited.contains(&nodes[1])); // B
    assert!(visited.contains(&nodes[3])); // D
    assert!(!visited.contains(&nodes[2])); // C should not be visited
}

#[test]
fn test_cyclic_graph_handling() {
    let mut graph = DiGraph::new();

    let node_a = graph.add_node(create_test_node("src/a.rs", 0));
    let node_b = graph.add_node(create_test_node("src/b.rs", 1));
    let node_c = graph.add_node(create_test_node("src/c.rs", 2));

    // Create cycle: A -> B -> C -> A
    graph.add_edge(
        node_a,
        node_b,
        DependencyEdgeType::Import { symbols: vec![] },
    );
    graph.add_edge(
        node_b,
        node_c,
        DependencyEdgeType::Import { symbols: vec![] },
    );
    graph.add_edge(
        node_c,
        node_a,
        DependencyEdgeType::Import { symbols: vec![] },
    );

    let traverser = GraphTraverser::new();

    // BFS should handle cycles without infinite loop
    let options = TraversalOptions {
        max_depth: 10,
        include_types: true,
        include_functions: true,
    };

    let visited = traverser.traverse_bfs(&graph, node_a, &options);
    assert_eq!(visited.len(), 3); // Should visit each node exactly once

    // Topological sort should detect the cycle
    let sorted = traverser.topological_sort(&graph);
    assert!(sorted.is_err());
    assert!(sorted.unwrap_err().to_string().contains("cycle"));
}

#[test]
fn test_disconnected_graph() {
    let mut graph = DiGraph::new();

    // Create two disconnected components
    let node_a = graph.add_node(create_test_node("src/a.rs", 0));
    let node_b = graph.add_node(create_test_node("src/b.rs", 1));
    let node_c = graph.add_node(create_test_node("src/c.rs", 2));
    let node_d = graph.add_node(create_test_node("src/d.rs", 3));

    // Component 1: A -> B
    graph.add_edge(
        node_a,
        node_b,
        DependencyEdgeType::Import { symbols: vec![] },
    );

    // Component 2: C -> D (disconnected from A-B)
    graph.add_edge(
        node_c,
        node_d,
        DependencyEdgeType::Import { symbols: vec![] },
    );

    let traverser = GraphTraverser::new();

    // From A, should only reach A and B
    let reachable_from_a = traverser.find_reachable_nodes(&graph, node_a);
    assert_eq!(reachable_from_a.len(), 2);
    assert!(reachable_from_a.contains(&node_a));
    assert!(reachable_from_a.contains(&node_b));

    // Topological sort should still work with disconnected components
    let sorted = traverser.topological_sort(&graph).unwrap();
    assert_eq!(sorted.len(), 4);
}

#[test]
fn test_empty_graph() {
    let graph: DiGraph<RichNode, DependencyEdgeType> = DiGraph::new();
    let traverser = GraphTraverser::new();

    // Topological sort of empty graph should return empty vec
    let sorted = traverser.topological_sort(&graph).unwrap();
    assert_eq!(sorted.len(), 0);
}

#[test]
fn test_single_node_graph() {
    let mut graph = DiGraph::new();
    let node = graph.add_node(create_test_node("src/main.rs", 0));

    let traverser = GraphTraverser::new();

    let options = TraversalOptions {
        max_depth: 1,
        include_types: true,
        include_functions: true,
    };

    // BFS should visit just the single node
    let visited = traverser.traverse_bfs(&graph, node, &options);
    assert_eq!(visited.len(), 1);
    assert!(visited.contains(&node));

    // Topological sort should work
    let sorted = traverser.topological_sort(&graph).unwrap();
    assert_eq!(sorted.len(), 1);
    assert_eq!(sorted[0], node);
}
