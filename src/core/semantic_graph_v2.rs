//! Optimized dependency graph using petgraph for O(V+E) performance
//! Replaces the O(n²) implementation with efficient graph algorithms

use petgraph::algo::{has_path_connecting, tarjan_scc};
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

/// Simplified file node for testing and graph construction
#[derive(Debug, Clone)]
pub struct FileNode {
    pub path: PathBuf,
    pub imports: Vec<PathBuf>,
    pub imported_by: Vec<PathBuf>,
}

/// Type of relationship between files
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationshipType {
    Import,
    FunctionCall,
    TypeReference,
}

/// Optimized dependency graph using petgraph
pub struct DependencyGraph {
    /// The directed graph structure
    graph: DiGraph<PathBuf, RelationshipType>,
    /// Map from file path to node index for O(1) lookups
    path_to_node: HashMap<PathBuf, NodeIndex>,
}

impl DependencyGraph {
    /// Build a dependency graph from a list of files
    /// Time complexity: O(V + E) where V is files and E is relationships
    pub fn build_from_files(files: &[FileNode]) -> Self {
        let mut graph = DiGraph::new();
        let mut path_to_node = HashMap::new();

        // First pass: Add all nodes - O(V)
        for file in files {
            let node = graph.add_node(file.path.clone());
            path_to_node.insert(file.path.clone(), node);
        }

        // Second pass: Add edges - O(E)
        for file in files {
            if let Some(&from_node) = path_to_node.get(&file.path) {
                // Add import edges
                for import_path in &file.imports {
                    if let Some(&to_node) = path_to_node.get(import_path) {
                        graph.add_edge(from_node, to_node, RelationshipType::Import);
                    }
                }
            }
        }

        Self {
            graph,
            path_to_node,
        }
    }

    /// Find all dependencies of a file up to a certain depth
    /// Uses custom BFS for efficient traversal - O(V + E)
    pub fn find_dependencies(&self, file_path: &PathBuf, max_depth: usize) -> Vec<PathBuf> {
        let mut dependencies = Vec::new();

        if let Some(&start_node) = self.path_to_node.get(file_path) {
            let mut visited = HashSet::new();
            let mut queue = std::collections::VecDeque::new();

            // Start with the initial node at depth 0
            queue.push_back((start_node, 0));

            while let Some((node, depth)) = queue.pop_front() {
                if depth > max_depth || visited.contains(&node) {
                    continue;
                }

                visited.insert(node);
                dependencies.push(self.graph[node].clone());

                // Add neighbors to queue with incremented depth
                if depth < max_depth {
                    for neighbor in self.graph.neighbors(node) {
                        if !visited.contains(&neighbor) {
                            queue.push_back((neighbor, depth + 1));
                        }
                    }
                }
            }
        }

        dependencies
    }

    /// Find reverse dependencies (files that import this file)
    /// O(V + E) using reverse graph traversal
    pub fn find_reverse_dependencies(&self, file_path: &PathBuf, max_depth: usize) -> Vec<PathBuf> {
        let mut dependencies = Vec::new();

        if let Some(&start_node) = self.path_to_node.get(file_path) {
            let mut visited = HashSet::new();
            let mut queue = vec![(start_node, 0)];

            while let Some((node, depth)) = queue.pop() {
                if depth > max_depth || visited.contains(&node) {
                    continue;
                }

                visited.insert(node);
                dependencies.push(self.graph[node].clone());

                // Traverse incoming edges (reverse dependencies)
                for edge in self
                    .graph
                    .edges_directed(node, petgraph::Direction::Incoming)
                {
                    let source = edge.source();
                    if !visited.contains(&source) && depth < max_depth {
                        queue.push((source, depth + 1));
                    }
                }
            }
        }

        dependencies
    }

    /// Check if there's a path between two files
    /// O(V + E) using DFS
    pub fn has_path(&self, from: &PathBuf, to: &PathBuf) -> bool {
        if let (Some(&from_node), Some(&to_node)) =
            (self.path_to_node.get(from), self.path_to_node.get(to))
        {
            has_path_connecting(&self.graph, from_node, to_node, None)
        } else {
            false
        }
    }

    /// Detect if the graph has cycles
    /// O(V + E) using Tarjan's algorithm
    pub fn has_cycles(&self) -> bool {
        let sccs = tarjan_scc(&self.graph);
        // If any strongly connected component has more than one node, there's a cycle
        sccs.iter().any(|scc| scc.len() > 1)
    }

    /// Find all cycles in the graph
    /// O(V + E) using Tarjan's strongly connected components
    pub fn find_cycles(&self) -> Vec<Vec<PathBuf>> {
        let sccs = tarjan_scc(&self.graph);

        sccs.into_iter()
            .filter(|scc| scc.len() > 1)
            .map(|scc| {
                scc.into_iter()
                    .map(|node| self.graph[node].clone())
                    .collect()
            })
            .collect()
    }

    /// Get the number of nodes in the graph
    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    /// Get the number of edges in the graph
    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }

    /// Get all files that directly import the given file
    pub fn get_direct_importers(&self, file_path: &PathBuf) -> Vec<PathBuf> {
        if let Some(&node) = self.path_to_node.get(file_path) {
            self.graph
                .neighbors_directed(node, petgraph::Direction::Incoming)
                .map(|n| self.graph[n].clone())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get all files directly imported by the given file
    pub fn get_direct_imports(&self, file_path: &PathBuf) -> Vec<PathBuf> {
        if let Some(&node) = self.path_to_node.get(file_path) {
            self.graph
                .neighbors_directed(node, petgraph::Direction::Outgoing)
                .map(|n| self.graph[n].clone())
                .collect()
        } else {
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_graph_construction() {
        let files = vec![
            FileNode {
                path: PathBuf::from("a.rs"),
                imports: vec![PathBuf::from("b.rs")],
                imported_by: vec![],
            },
            FileNode {
                path: PathBuf::from("b.rs"),
                imports: vec![],
                imported_by: vec![],
            },
        ];

        let graph = DependencyGraph::build_from_files(&files);

        assert_eq!(graph.node_count(), 2);
        assert_eq!(graph.edge_count(), 1);

        assert!(graph.has_path(&PathBuf::from("a.rs"), &PathBuf::from("b.rs")));
        assert!(!graph.has_path(&PathBuf::from("b.rs"), &PathBuf::from("a.rs")));
    }

    #[test]
    fn test_cycle_detection() {
        let files = vec![
            FileNode {
                path: PathBuf::from("a.rs"),
                imports: vec![PathBuf::from("b.rs")],
                imported_by: vec![],
            },
            FileNode {
                path: PathBuf::from("b.rs"),
                imports: vec![PathBuf::from("a.rs")],
                imported_by: vec![],
            },
        ];

        let graph = DependencyGraph::build_from_files(&files);

        assert!(graph.has_cycles());
        let cycles = graph.find_cycles();
        assert_eq!(cycles.len(), 1);
        assert_eq!(cycles[0].len(), 2);
    }

    #[test]
    fn test_reverse_dependencies() {
        let files = vec![
            FileNode {
                path: PathBuf::from("utils.rs"),
                imports: vec![],
                imported_by: vec![],
            },
            FileNode {
                path: PathBuf::from("lib.rs"),
                imports: vec![PathBuf::from("utils.rs")],
                imported_by: vec![],
            },
            FileNode {
                path: PathBuf::from("main.rs"),
                imports: vec![PathBuf::from("lib.rs"), PathBuf::from("utils.rs")],
                imported_by: vec![],
            },
        ];

        let graph = DependencyGraph::build_from_files(&files);

        let utils_importers = graph.get_direct_importers(&PathBuf::from("utils.rs"));
        assert_eq!(utils_importers.len(), 2); // lib.rs and main.rs

        let lib_importers = graph.get_direct_importers(&PathBuf::from("lib.rs"));
        assert_eq!(lib_importers.len(), 1); // main.rs
    }
}
