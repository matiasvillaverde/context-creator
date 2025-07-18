//! Graph construction module for semantic analysis
//!
//! This module is responsible for building dependency graphs from file information.
//! It follows the Single Responsibility Principle by focusing solely on graph construction.

use crate::core::semantic::dependency_types::{
    DependencyEdgeType, DependencyNode as RichNode, FileAnalysisResult,
};
use crate::core::walker::FileInfo;
use anyhow::Result;
use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Builder for constructing dependency graphs
pub struct GraphBuilder {
    // Future: Could add configuration options here
}

impl GraphBuilder {
    /// Create a new GraphBuilder
    pub fn new() -> Self {
        Self {}
    }

    /// Build a dependency graph from file information
    pub fn build(
        &self,
        files: &[FileInfo],
    ) -> Result<(
        DiGraph<RichNode, DependencyEdgeType>,
        HashMap<PathBuf, NodeIndex>,
    )> {
        let mut graph = DiGraph::new();
        let mut node_map = HashMap::new();

        // Create nodes for each file
        for (index, file) in files.iter().enumerate() {
            let rich_node = RichNode {
                file_index: index,
                path: file.path.clone(),
                language: Self::detect_language(&file.path),
                content_hash: None, // Will be filled during analysis
                file_size: file.size,
                depth: 0,
            };

            let node_idx = graph.add_node(rich_node);
            // Only store the last occurrence if there are duplicates
            let _ = node_map.insert(file.path.clone(), node_idx);
        }

        Ok((graph, node_map))
    }

    /// Add a dependency edge to the graph
    pub fn add_edge(
        &self,
        graph: &mut DiGraph<RichNode, DependencyEdgeType>,
        from: NodeIndex,
        to: NodeIndex,
        edge_type: DependencyEdgeType,
    ) {
        // Avoid self-loops
        if from != to {
            let _ = graph.add_edge(from, to, edge_type);
        }
    }

    /// Build edges from file import information
    pub fn build_edges_from_imports(
        &self,
        graph: &mut DiGraph<RichNode, DependencyEdgeType>,
        files: &[FileInfo],
        node_map: &HashMap<PathBuf, NodeIndex>,
    ) {
        for file in files {
            if let Some(&from_idx) = node_map.get(&file.path) {
                for import_path in &file.imports {
                    if let Some(&to_idx) = node_map.get(import_path) {
                        let edge_type = DependencyEdgeType::Import {
                            symbols: Vec::new(), // Basic import without symbol information
                        };
                        self.add_edge(graph, from_idx, to_idx, edge_type);
                    }
                }
            }
        }
    }

    /// Build edges from parallel analysis results
    pub fn build_edges_from_analysis(
        &self,
        graph: &mut DiGraph<RichNode, DependencyEdgeType>,
        analysis_results: &[FileAnalysisResult],
        path_to_index: &HashMap<PathBuf, usize>,
        node_map: &HashMap<PathBuf, NodeIndex>,
    ) {
        for result in analysis_results {
            let file_index = result.file_index;

            // Find the source node
            let source_path = path_to_index
                .iter()
                .find(|(_, &idx)| idx == file_index)
                .map(|(path, _)| path.clone());

            if let Some(source_path) = source_path {
                if let Some(&from_idx) = node_map.get(&source_path) {
                    // Update node with content hash
                    if let Some(hash) = result.content_hash {
                        graph[from_idx].content_hash = Some(hash);
                    }

                    // Add import edges
                    for (import_path, edge_type) in &result.imports {
                        // Try to find the target in our node map
                        for (path, &to_idx) in node_map {
                            if path
                                .to_string_lossy()
                                .contains(&import_path.to_string_lossy().to_string())
                            {
                                self.add_edge(graph, from_idx, to_idx, edge_type.clone());
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    /// Detect programming language from file extension
    fn detect_language(path: &Path) -> Option<String> {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| match ext {
                "rs" => "rust",
                "py" => "python",
                "js" | "mjs" => "javascript",
                "ts" | "tsx" => "typescript",
                "jsx" => "javascript",
                "go" => "go",
                "java" => "java",
                "cpp" | "cc" | "cxx" => "cpp",
                "c" => "c",
                "rb" => "ruby",
                "php" => "php",
                "swift" => "swift",
                "kt" => "kotlin",
                "scala" => "scala",
                "r" => "r",
                "sh" | "bash" => "shell",
                "yaml" | "yml" => "yaml",
                "json" => "json",
                "xml" => "xml",
                "html" | "htm" => "html",
                "css" | "scss" | "sass" => "css",
                _ => ext,
            })
            .map(String::from)
    }
}

impl Default for GraphBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "graph_builder_tests.rs"]
mod tests;
