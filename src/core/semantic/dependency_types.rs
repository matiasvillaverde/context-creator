//! Rich dependency graph types for semantic analysis

use std::path::PathBuf;

/// Edge types for the dependency graph
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DependencyEdgeType {
    /// File imports another file
    Import {
        /// The specific symbols imported (if available)
        symbols: Vec<String>,
    },
    /// File calls functions from another file
    FunctionCall {
        /// The function name being called
        function_name: String,
        /// Module containing the function
        module: Option<String>,
    },
    /// Type reference to another file
    TypeReference {
        /// The type name being referenced
        type_name: String,
        /// Whether it's a generic parameter
        is_generic: bool,
    },
    /// Inheritance relationship
    Inheritance {
        /// The base type being extended
        base_type: String,
    },
    /// Interface implementation
    InterfaceImplementation {
        /// The interface being implemented
        interface_name: String,
    },
}

/// Node metadata for the dependency graph
#[derive(Debug, Clone)]
pub struct DependencyNode {
    /// File index in the original files vector
    pub file_index: usize,
    /// Absolute path to the file
    pub path: PathBuf,
    /// Programming language of the file
    pub language: Option<String>,
    /// Hash of file content for cache invalidation
    pub content_hash: Option<u64>,
    /// Size of the file in bytes
    pub file_size: u64,
    /// Depth in the dependency graph (for BFS)
    pub depth: usize,
}

/// Result of parallel file analysis
#[derive(Debug, Clone)]
pub struct FileAnalysisResult {
    /// File index
    pub file_index: usize,
    /// Import relationships found
    pub imports: Vec<(PathBuf, DependencyEdgeType)>,
    /// Function calls made
    pub function_calls: Vec<crate::core::semantic::analyzer::FunctionCall>,
    /// Type references used
    pub type_references: Vec<crate::core::semantic::analyzer::TypeReference>,
    /// Function definitions exported
    pub exported_functions: Vec<crate::core::semantic::analyzer::FunctionDefinition>,
    /// Content hash for cache invalidation
    pub content_hash: Option<u64>,
    /// Error if analysis failed
    pub error: Option<String>,
}
