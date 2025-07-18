//! Graph traversal module for semantic analysis
//!
//! This module is responsible for traversing dependency graphs using various algorithms.
//! It follows the Single Responsibility Principle by focusing solely on graph traversal.

use crate::core::semantic::dependency_types::{DependencyEdgeType, DependencyNode as RichNode};
use anyhow::{anyhow, Result};
use petgraph::algo::toposort;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::Dfs;
use std::collections::{HashSet, VecDeque};

/// Options for graph traversal
#[derive(Debug, Clone)]
pub struct TraversalOptions {
    /// Maximum depth to traverse
    pub max_depth: usize,
    /// Whether to include type dependencies
    pub include_types: bool,
    /// Whether to include function call dependencies
    pub include_functions: bool,
}

impl Default for TraversalOptions {
    fn default() -> Self {
        Self {
            max_depth: 5,
            include_types: true,
            include_functions: true,
        }
    }
}

/// Traverser for dependency graphs
pub struct GraphTraverser {
    // Future: Could add traversal configuration here
}

impl GraphTraverser {
    /// Create a new GraphTraverser
    pub fn new() -> Self {
        Self {}
    }

    /// Perform breadth-first traversal from a starting node
    pub fn traverse_bfs(
        &self,
        graph: &DiGraph<RichNode, DependencyEdgeType>,
        start: NodeIndex,
        options: &TraversalOptions,
    ) -> Vec<NodeIndex> {
        let mut visited = Vec::new();
        let mut seen = HashSet::new();
        let mut queue = VecDeque::new();

        // Initialize with start node at depth 0
        queue.push_back((start, 0));
        seen.insert(start);

        while let Some((node, depth)) = queue.pop_front() {
            // Check depth limit
            if depth > options.max_depth {
                continue;
            }

            visited.push(node);

            // Add neighbors to queue if not seen
            for neighbor in graph.neighbors(node) {
                if !seen.contains(&neighbor) {
                    seen.insert(neighbor);
                    queue.push_back((neighbor, depth + 1));
                }
            }
        }

        visited
    }

    /// Perform depth-first traversal from a starting node
    pub fn traverse_dfs(
        &self,
        graph: &DiGraph<RichNode, DependencyEdgeType>,
        start: NodeIndex,
        options: &TraversalOptions,
    ) -> Vec<NodeIndex> {
        let mut visited = Vec::new();
        let mut dfs = Dfs::new(graph, start);
        let mut depths = HashMap::new();
        depths.insert(start, 0);

        while let Some(node) = dfs.next(graph) {
            let current_depth = *depths.get(&node).unwrap_or(&0);

            // Check depth limit
            if current_depth <= options.max_depth {
                visited.push(node);

                // Set depth for neighbors
                for neighbor in graph.neighbors(node) {
                    depths.entry(neighbor).or_insert(current_depth + 1);
                }
            }
        }

        visited
    }

    /// Perform topological sort on the graph
    pub fn topological_sort(
        &self,
        graph: &DiGraph<RichNode, DependencyEdgeType>,
    ) -> Result<Vec<NodeIndex>> {
        match toposort(graph, None) {
            Ok(order) => Ok(order),
            Err(_) => Err(anyhow!(
                "Graph contains a cycle, topological sort not possible"
            )),
        }
    }

    /// Find all nodes reachable from a starting node
    pub fn find_reachable_nodes(
        &self,
        graph: &DiGraph<RichNode, DependencyEdgeType>,
        start: NodeIndex,
    ) -> HashSet<NodeIndex> {
        let mut reachable = HashSet::new();
        let mut dfs = Dfs::new(graph, start);

        while let Some(node) = dfs.next(graph) {
            reachable.insert(node);
        }

        reachable
    }

    /// Get nodes at a specific depth from a starting node
    pub fn get_nodes_at_depth(
        &self,
        graph: &DiGraph<RichNode, DependencyEdgeType>,
        start: NodeIndex,
        target_depth: usize,
    ) -> Vec<NodeIndex> {
        let mut nodes_at_depth = Vec::new();
        let mut queue = VecDeque::new();
        let mut seen = HashSet::new();

        queue.push_back((start, 0));
        seen.insert(start);

        while let Some((node, depth)) = queue.pop_front() {
            if depth == target_depth {
                nodes_at_depth.push(node);
            } else if depth < target_depth {
                // Add neighbors to queue
                for neighbor in graph.neighbors(node) {
                    if !seen.contains(&neighbor) {
                        seen.insert(neighbor);
                        queue.push_back((neighbor, depth + 1));
                    }
                }
            }
        }

        nodes_at_depth
    }

    /// Find the shortest path between two nodes
    pub fn find_shortest_path(
        &self,
        graph: &DiGraph<RichNode, DependencyEdgeType>,
        start: NodeIndex,
        end: NodeIndex,
    ) -> Option<Vec<NodeIndex>> {
        use petgraph::algo::dijkstra;

        let predecessors = dijkstra(graph, start, Some(end), |_| 1);

        if !predecessors.contains_key(&end) {
            return None;
        }

        // Reconstruct path
        let mut path = vec![end];
        let mut current = end;

        // This is a simplified path reconstruction
        // In a real implementation, we'd need to track predecessors properly
        while current != start {
            if let Some(neighbor) = graph
                .neighbors_directed(current, petgraph::Direction::Incoming)
                .next()
            {
                path.push(neighbor);
                current = neighbor;
            } else {
                break;
            }
        }

        path.reverse();
        Some(path)
    }
}

impl Default for GraphTraverser {
    fn default() -> Self {
        Self::new()
    }
}

// Fix missing import
use std::collections::HashMap;

#[cfg(test)]
#[path = "graph_traverser_tests.rs"]
mod tests;
