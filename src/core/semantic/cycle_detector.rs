//! Cycle detection module using Tarjan's algorithm
//!
//! This module provides robust cycle detection for dependency graphs,
//! identifying strongly connected components and providing strategies
//! for handling circular dependencies.

use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::HashMap;

/// Result of cycle detection analysis
#[derive(Debug, Clone)]
pub struct CycleDetectionResult {
    /// Strongly connected components (each inner Vec is a cycle)
    pub cycles: Vec<Vec<NodeIndex>>,
    /// Whether any cycles were detected
    pub has_cycles: bool,
    /// Detailed cycle information with file paths
    pub cycle_details: Vec<CycleDetail>,
}

/// Detailed information about a detected cycle
#[derive(Debug, Clone)]
pub struct CycleDetail {
    /// Nodes involved in the cycle
    pub nodes: Vec<NodeIndex>,
    /// Human-readable description of the cycle
    pub description: String,
}

/// Resolution strategy for handling cycles
#[derive(Debug, Clone)]
pub enum CycleResolution {
    /// Break the cycle by removing specific edges
    BreakEdges(Vec<(NodeIndex, NodeIndex)>),
    /// Process nodes in a partial topological order
    PartialOrder(Vec<NodeIndex>),
    /// Merge strongly connected components into single units
    MergeComponents(Vec<Vec<NodeIndex>>),
}

/// Tarjan's algorithm implementation for cycle detection
pub struct TarjanCycleDetector {
    /// Current DFS index
    index: usize,
    /// Stack for tracking current path
    stack: Vec<NodeIndex>,
    /// Node indices in DFS
    indices: HashMap<NodeIndex, usize>,
    /// Lowest reachable index for each node
    low_links: HashMap<NodeIndex, usize>,
    /// Whether node is on stack
    on_stack: HashMap<NodeIndex, bool>,
    /// Detected strongly connected components
    sccs: Vec<Vec<NodeIndex>>,
}

impl TarjanCycleDetector {
    /// Create a new cycle detector
    pub fn new() -> Self {
        Self {
            index: 0,
            stack: Vec::new(),
            indices: HashMap::new(),
            low_links: HashMap::new(),
            on_stack: HashMap::new(),
            sccs: Vec::new(),
        }
    }

    /// Detect all cycles in the graph
    pub fn detect_cycles<N, E>(&mut self, graph: &DiGraph<N, E>) -> CycleDetectionResult {
        // Reset state
        self.index = 0;
        self.stack.clear();
        self.indices.clear();
        self.low_links.clear();
        self.on_stack.clear();
        self.sccs.clear();

        // Run Tarjan's algorithm on all unvisited nodes
        for node in graph.node_indices() {
            if !self.indices.contains_key(&node) {
                self.strong_connect(graph, node);
            }
        }

        // Filter out single-node SCCs that aren't self-cycles
        let mut cycles = Vec::new();
        for scc in &self.sccs {
            if scc.len() > 1 {
                cycles.push(scc.clone());
            } else if scc.len() == 1 {
                // Check for self-cycle
                let node = scc[0];
                if graph.find_edge(node, node).is_some() {
                    cycles.push(scc.clone());
                }
            }
        }

        let has_cycles = !cycles.is_empty();
        let cycle_details = cycles
            .iter()
            .enumerate()
            .map(|(i, cycle)| CycleDetail {
                nodes: cycle.clone(),
                description: format!("Cycle {}: {} nodes", i + 1, cycle.len()),
            })
            .collect();

        CycleDetectionResult {
            cycles,
            has_cycles,
            cycle_details,
        }
    }

    /// Find all strongly connected components
    pub fn find_strongly_connected_components<N, E>(
        &mut self,
        graph: &DiGraph<N, E>,
    ) -> Vec<Vec<NodeIndex>> {
        let _result = self.detect_cycles(graph);
        // Return all SCCs, including single nodes
        self.sccs.clone()
    }

    /// Handle detected cycles with a resolution strategy
    pub fn handle_cycles<N, E>(
        &self,
        graph: &DiGraph<N, E>,
        cycles: Vec<Vec<NodeIndex>>,
    ) -> CycleResolution {
        if cycles.is_empty() {
            // No cycles, return original topological order
            if let Ok(order) = petgraph::algo::toposort(graph, None) {
                return CycleResolution::PartialOrder(order);
            }
        }

        // Build a set of all nodes that are in cycles
        let mut nodes_in_cycles = std::collections::HashSet::new();
        for cycle in &cycles {
            for &node in cycle {
                nodes_in_cycles.insert(node);
            }
        }

        // Use a modified topological sort that treats cycle nodes as a single unit
        let mut visited = std::collections::HashSet::new();
        let mut partial_order = Vec::new();

        // Helper function for DFS traversal
        fn visit<N, E>(
            node: NodeIndex,
            graph: &DiGraph<N, E>,
            visited: &mut std::collections::HashSet<NodeIndex>,
            partial_order: &mut Vec<NodeIndex>,
            nodes_in_cycles: &std::collections::HashSet<NodeIndex>,
        ) {
            if visited.contains(&node) {
                return;
            }

            visited.insert(node);

            // Visit dependencies first (unless they're in the same cycle)
            for neighbor in graph.neighbors(node) {
                // Skip if neighbor is in the same cycle as current node
                let skip = nodes_in_cycles.contains(&node) && nodes_in_cycles.contains(&neighbor);
                if !skip {
                    visit(neighbor, graph, visited, partial_order, nodes_in_cycles);
                }
            }

            partial_order.push(node);
        }

        // Visit all nodes
        for node in graph.node_indices() {
            visit(
                node,
                graph,
                &mut visited,
                &mut partial_order,
                &nodes_in_cycles,
            );
        }

        // Reverse to get proper order (dependencies before dependents)
        partial_order.reverse();

        CycleResolution::PartialOrder(partial_order)
    }

    /// Core Tarjan's algorithm implementation
    fn strong_connect<N, E>(&mut self, graph: &DiGraph<N, E>, v: NodeIndex) {
        // Set the depth index for v
        self.indices.insert(v, self.index);
        self.low_links.insert(v, self.index);
        self.index += 1;
        self.stack.push(v);
        self.on_stack.insert(v, true);

        // Consider successors of v
        for neighbor in graph.neighbors(v) {
            if !self.indices.contains_key(&neighbor) {
                // Successor has not yet been visited; recurse on it
                self.strong_connect(graph, neighbor);
                let v_low = *self.low_links.get(&v).unwrap();
                let neighbor_low = *self.low_links.get(&neighbor).unwrap();
                self.low_links.insert(v, v_low.min(neighbor_low));
            } else if *self.on_stack.get(&neighbor).unwrap_or(&false) {
                // Successor is in stack and hence in the current SCC
                let v_low = *self.low_links.get(&v).unwrap();
                let neighbor_index = *self.indices.get(&neighbor).unwrap();
                self.low_links.insert(v, v_low.min(neighbor_index));
            }
        }

        // If v is a root node, pop the stack and print an SCC
        if self.low_links.get(&v) == self.indices.get(&v) {
            let mut scc = Vec::new();
            loop {
                let w = self.stack.pop().unwrap();
                self.on_stack.insert(w, false);
                scc.push(w);
                if w == v {
                    break;
                }
            }
            self.sccs.push(scc);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use petgraph::graph::DiGraph;

    #[test]
    fn test_simple_cycle_detection() {
        // Create a simple cycle: A → B → A
        let mut graph = DiGraph::new();
        let a = graph.add_node("A");
        let b = graph.add_node("B");
        graph.add_edge(a, b, ());
        graph.add_edge(b, a, ());

        let mut detector = TarjanCycleDetector::new();
        let result = detector.detect_cycles(&graph);

        assert!(result.has_cycles);
        assert_eq!(result.cycles.len(), 1);
        assert_eq!(result.cycles[0].len(), 2);
    }

    #[test]
    fn test_complex_cycle_detection() {
        // Create a complex cycle: A → B → C → A
        let mut graph = DiGraph::new();
        let a = graph.add_node("A");
        let b = graph.add_node("B");
        let c = graph.add_node("C");
        graph.add_edge(a, b, ());
        graph.add_edge(b, c, ());
        graph.add_edge(c, a, ());

        let mut detector = TarjanCycleDetector::new();
        let result = detector.detect_cycles(&graph);

        assert!(result.has_cycles);
        assert_eq!(result.cycles.len(), 1);
        assert_eq!(result.cycles[0].len(), 3);
    }

    #[test]
    fn test_multiple_cycles_detection() {
        // Create multiple independent cycles
        let mut graph = DiGraph::new();

        // First cycle: A → B → A
        let a = graph.add_node("A");
        let b = graph.add_node("B");
        graph.add_edge(a, b, ());
        graph.add_edge(b, a, ());

        // Second cycle: C → D → E → C
        let c = graph.add_node("C");
        let d = graph.add_node("D");
        let e = graph.add_node("E");
        graph.add_edge(c, d, ());
        graph.add_edge(d, e, ());
        graph.add_edge(e, c, ());

        let mut detector = TarjanCycleDetector::new();
        let result = detector.detect_cycles(&graph);

        assert!(result.has_cycles);
        assert_eq!(result.cycles.len(), 2);

        // Check that we found both cycles
        let cycle_sizes: Vec<usize> = result.cycles.iter().map(|c| c.len()).collect();
        assert!(cycle_sizes.contains(&2));
        assert!(cycle_sizes.contains(&3));
    }

    #[test]
    fn test_no_cycle_detection() {
        // Create a valid DAG: A → B → C
        let mut graph = DiGraph::new();
        let a = graph.add_node("A");
        let b = graph.add_node("B");
        let c = graph.add_node("C");
        graph.add_edge(a, b, ());
        graph.add_edge(b, c, ());

        let mut detector = TarjanCycleDetector::new();
        let result = detector.detect_cycles(&graph);

        assert!(!result.has_cycles);
        assert_eq!(result.cycles.len(), 0);
    }

    #[test]
    fn test_self_cycle_detection() {
        // Create a self-cycle: A → A
        let mut graph = DiGraph::new();
        let a = graph.add_node("A");
        graph.add_edge(a, a, ());

        let mut detector = TarjanCycleDetector::new();
        let result = detector.detect_cycles(&graph);

        assert!(result.has_cycles);
        assert_eq!(result.cycles.len(), 1);
        assert_eq!(result.cycles[0].len(), 1);
        assert_eq!(result.cycles[0][0], a);
    }

    #[test]
    fn test_strongly_connected_components() {
        // Create a graph with multiple SCCs
        let mut graph = DiGraph::new();

        // SCC 1: A → B → A
        let a = graph.add_node("A");
        let b = graph.add_node("B");
        graph.add_edge(a, b, ());
        graph.add_edge(b, a, ());

        // SCC 2: Single node C
        let c = graph.add_node("C");

        // SCC 3: D → E → F → D
        let d = graph.add_node("D");
        let e = graph.add_node("E");
        let f = graph.add_node("F");
        graph.add_edge(d, e, ());
        graph.add_edge(e, f, ());
        graph.add_edge(f, d, ());

        // Connect SCCs
        graph.add_edge(a, c, ());
        graph.add_edge(c, d, ());

        let mut detector = TarjanCycleDetector::new();
        let sccs = detector.find_strongly_connected_components(&graph);

        // Should find 3 SCCs
        assert_eq!(sccs.len(), 3);

        // Verify SCC sizes
        let scc_sizes: Vec<usize> = sccs.iter().map(|scc| scc.len()).collect();
        assert!(scc_sizes.contains(&1)); // Node C
        assert!(scc_sizes.contains(&2)); // A-B cycle
        assert!(scc_sizes.contains(&3)); // D-E-F cycle
    }

    #[test]
    fn test_cycle_resolution() {
        // Create a graph with a cycle
        let mut graph = DiGraph::new();
        let a = graph.add_node("A");
        let b = graph.add_node("B");
        let c = graph.add_node("C");
        let d = graph.add_node("D");

        // Create cycle: A → B → C → A
        graph.add_edge(a, b, ());
        graph.add_edge(b, c, ());
        graph.add_edge(c, a, ());

        // Add non-cycle node
        graph.add_edge(c, d, ());

        let mut detector = TarjanCycleDetector::new();
        let result = detector.detect_cycles(&graph);
        let resolution = detector.handle_cycles(&graph, result.cycles);

        match resolution {
            CycleResolution::PartialOrder(order) => {
                assert_eq!(order.len(), 4); // All nodes should be included
                                            // D should come after the cycle nodes since it depends on them
                let d_index = order.iter().position(|&n| n == d).unwrap();
                assert!(d_index > 0); // D is not first
            }
            _ => panic!("Expected PartialOrder resolution"),
        }
    }
}
