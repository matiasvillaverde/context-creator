//! Dependency graph traversal for semantic analysis with parallel processing

use crate::core::cache::FileCache;
use crate::core::semantic::cycle_detector::{CycleResolution, TarjanCycleDetector};
use crate::core::semantic::{analyzer::SemanticContext, SemanticOptions};
use crate::core::walker::FileInfo;
use anyhow::Result;
use petgraph::graph::{DiGraph, NodeIndex};
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::core::semantic::dependency_types::{
    DependencyEdgeType, DependencyNode as RichNode, FileAnalysisResult,
};

/// Legacy node structure for compatibility
#[derive(Debug, Clone)]
struct DependencyNode {
    file_index: usize,
    #[allow(dead_code)]
    depth: usize,
    imports: Vec<PathBuf>,
    #[allow(dead_code)]
    imported_by: Vec<PathBuf>,
    function_calls: Vec<crate::core::semantic::analyzer::FunctionCall>,
    type_references: Vec<crate::core::semantic::analyzer::TypeReference>,
}

/// Performs sophisticated semantic analysis with proper dependency graph traversal
pub fn perform_semantic_analysis_graph(
    files: &mut [FileInfo],
    config: &crate::cli::Config,
    cache: &FileCache,
) -> Result<()> {
    // Skip if no semantic analysis is requested
    if !config.trace_imports && !config.include_callers && !config.include_types {
        return Ok(());
    }

    let semantic_options = SemanticOptions::from_config(config);

    // Build initial dependency graph
    let mut graph = DependencyGraph::new(files, &semantic_options, cache)?;

    // Perform breadth-first traversal up to semantic_depth
    graph.traverse_dependencies(&semantic_options)?;

    // Apply the discovered dependencies back to the files
    graph.apply_to_files(files);

    Ok(())
}

/// Dependency graph for semantic analysis with parallel processing
struct DependencyGraph<'a> {
    nodes: Vec<DependencyNode>,
    path_to_index: HashMap<PathBuf, usize>,
    index_to_path: HashMap<usize, PathBuf>,
    visited: HashSet<usize>,
    cache: &'a FileCache,
    project_root: PathBuf,
    graph: DiGraph<RichNode, DependencyEdgeType>,
    node_indices: HashMap<usize, NodeIndex>,
}

impl<'a> DependencyGraph<'a> {
    /// Create a new dependency graph from the initial set of files
    fn new(files: &[FileInfo], _options: &SemanticOptions, cache: &'a FileCache) -> Result<Self> {
        let mut nodes = Vec::new();
        let mut path_to_index = HashMap::new();
        let mut index_to_path = HashMap::new();

        // Create nodes for all files
        for (index, file) in files.iter().enumerate() {
            path_to_index.insert(file.path.clone(), index);
            index_to_path.insert(index, file.path.clone());
            nodes.push(DependencyNode {
                file_index: index,
                depth: 0,
                imports: Vec::new(),
                imported_by: Vec::new(),
                function_calls: Vec::new(),
                type_references: Vec::new(),
            });
        }

        // Detect project root from first file
        let project_root = if let Some(first_file) = files.first() {
            Self::detect_project_root(&first_file.path)
        } else {
            // Fallback to current directory
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
        };

        let mut graph = DiGraph::new();
        let mut node_indices = HashMap::new();

        // Add rich nodes to petgraph with metadata
        for (index, file) in files.iter().enumerate() {
            let rich_node = RichNode {
                file_index: index,
                path: file.path.clone(),
                language: Self::detect_language(&file.path),
                content_hash: None, // Will be computed during analysis
                file_size: file.size,
                depth: 0,
            };
            let node_idx = graph.add_node(rich_node);
            node_indices.insert(index, node_idx);
        }

        Ok(Self {
            nodes,
            path_to_index,
            index_to_path,
            visited: HashSet::new(),
            cache,
            project_root,
            graph,
            node_indices,
        })
    }

    /// Detect the project root directory using git root or fallback methods
    fn detect_project_root(start_path: &Path) -> PathBuf {
        // Start from the directory containing the file
        let mut current = start_path.parent().unwrap_or(start_path);

        // First try to find git root
        loop {
            if current.join(".git").exists() {
                return current.to_path_buf();
            }
            if let Some(parent) = current.parent() {
                current = parent;
            } else {
                break;
            }
        }

        // If no .git found, output a warning message
        eprintln!("Warning: No .git directory found for file: {start_path:?}");
        eprintln!("Warning: --include-types functionality requires a git repository to properly detect project root.");
        eprintln!("Warning: Falling back to project marker detection...");

        // Fallback: Look for common project markers
        current = start_path.parent().unwrap_or(start_path);
        loop {
            // Check for Rust project markers
            if current.join("Cargo.toml").exists() {
                return current.to_path_buf();
            }
            // Check for Node.js project markers
            if current.join("package.json").exists() {
                return current.to_path_buf();
            }
            // Check for Python project markers
            if current.join("pyproject.toml").exists() || current.join("setup.py").exists() {
                return current.to_path_buf();
            }
            // Check for generic project markers
            if current.join("README.md").exists() || current.join("readme.md").exists() {
                return current.to_path_buf();
            }

            if let Some(parent) = current.parent() {
                current = parent;
            } else {
                break;
            }
        }

        // Final fallback: Look for common project structure indicators
        // Go up from the file directory and look for directories that indicate project root
        current = start_path.parent().unwrap_or(start_path);
        while let Some(parent) = current.parent() {
            // If we find a directory that contains both 'src' and other common directories,
            // it's likely the project root
            if parent.join("src").exists()
                && (parent.join("shared").exists()
                    || parent.join("lib").exists()
                    || parent.join("tests").exists())
            {
                return parent.to_path_buf();
            }
            current = parent;
        }

        // Ultimate fallback: use the directory containing the start path
        eprintln!(
            "Warning: Could not detect project root. Using directory containing file: {:?}",
            start_path.parent().unwrap_or(start_path)
        );
        start_path.parent().unwrap_or(start_path).to_path_buf()
    }

    /// Traverse dependencies using parallel processing and Kahn's algorithm
    fn traverse_dependencies(&mut self, options: &SemanticOptions) -> Result<()> {
        // Phase 1: Parallel file analysis
        let analysis_results = self.parallel_analyze_files(options)?;

        // Phase 2: Build dependency graph from analysis results
        self.build_dependency_graph_from_results(&analysis_results);

        // Phase 3: Use Tarjan's algorithm to detect cycles and get processing order
        let mut cycle_detector = TarjanCycleDetector::new();
        let cycle_result = cycle_detector.detect_cycles(&self.graph);

        if cycle_result.has_cycles {
            // Report all detected cycles
            eprintln!(
                "Warning: {} circular dependencies detected:",
                cycle_result.cycles.len()
            );
            for (i, cycle) in cycle_result.cycles.iter().enumerate() {
                eprintln!("\nCycle {}:", i + 1);
                for &node_idx in cycle {
                    let node = &self.graph[node_idx];
                    eprintln!("  - {}", node.path.display());
                }
            }
            eprintln!("\nWarning: Processing files in partial order, some dependencies may be incomplete.");
        }

        // Get resolution strategy
        let resolution = cycle_detector.handle_cycles(&self.graph, cycle_result.cycles);

        match resolution {
            CycleResolution::PartialOrder(node_order) => {
                // Process nodes in the computed order
                for node_idx in node_order {
                    let rich_node = &self.graph[node_idx];
                    let file_idx = rich_node.file_index;
                    self.visited.insert(file_idx);

                    // Process dependencies up to semantic_depth
                    self.process_dependencies(file_idx, options);
                }
            }
            CycleResolution::BreakEdges(_) => {
                // For future implementation of edge-breaking strategy
                unimplemented!("Edge breaking strategy not yet implemented");
            }
            CycleResolution::MergeComponents(_) => {
                // For future implementation of component merging strategy
                unimplemented!("Component merging strategy not yet implemented");
            }
        }

        Ok(())
    }

    /// Analyze a single file and extract semantic information
    #[allow(dead_code)]
    fn analyze_file(
        &mut self,
        file_idx: usize,
        depth: usize,
        options: &SemanticOptions,
    ) -> Result<()> {
        // Get file info from the original files vector
        let _file_path = self.nodes[file_idx].file_index;

        // Skip if file type is not a programming language
        // This check would use the actual FileInfo from the files vector

        // Get the appropriate analyzer
        let analyzer =
            match crate::core::semantic::get_analyzer_for_file(self.get_file_path(file_idx))? {
                Some(analyzer) => analyzer,
                None => return Ok(()), // No analyzer for this file type
            };

        // Read file content
        let content = self.cache.get_or_load(self.get_file_path(file_idx))?;

        // Create context using project root instead of file's parent directory
        let context = SemanticContext::new(
            self.get_file_path(file_idx).to_path_buf(),
            self.project_root.clone(),
            options.semantic_depth,
        );

        // Perform analysis
        let analysis_result =
            analyzer.analyze_file(self.get_file_path(file_idx), &content, &context)?;

        // Process imports - we need import info for type expansion even if trace_imports is false
        // Store raw import module paths for file expansion
        let mut raw_imports = Vec::new();
        for import in &analysis_result.imports {
            // Convert module path to a simple PathBuf for now
            // This will be used by file_expander to find type definition files
            raw_imports.push(PathBuf::from(&import.module));
        }

        if options.trace_imports {
            let mut resolved_imports = Vec::new();

            if let Ok(Some(resolver)) =
                crate::core::semantic::get_resolver_for_file(self.get_file_path(file_idx))
            {
                for import in &analysis_result.imports {
                    match resolver.resolve_import(
                        &import.module,
                        self.get_file_path(file_idx),
                        &self.project_root,
                    ) {
                        Ok(resolved) => {
                            if !resolved.is_external
                                && self.path_to_index.contains_key(&resolved.path)
                            {
                                resolved_imports.push(resolved.path);
                            }
                        }
                        Err(_) => {
                            // Simple fallback resolution
                            if let Some(resolved) = self.simple_resolve(&import.module, file_idx) {
                                resolved_imports.push(resolved);
                            }
                        }
                    }
                }
            }

            // Use resolved imports if available, otherwise use raw imports
            self.nodes[file_idx].imports = if !resolved_imports.is_empty() {
                resolved_imports
            } else {
                raw_imports
            };
        } else {
            // Even if trace_imports is false, we need import info for type expansion
            self.nodes[file_idx].imports = raw_imports;
        }

        // Store function calls and type references
        if options.include_callers {
            self.nodes[file_idx].function_calls = analysis_result.function_calls;
        }

        if options.include_types {
            self.nodes[file_idx].type_references = analysis_result.type_references;
        }

        self.nodes[file_idx].depth = depth;

        Ok(())
    }

    /// Get file path for a given index
    fn get_file_path(&self, file_idx: usize) -> &Path {
        self.index_to_path
            .get(&file_idx)
            .map(|p| p.as_path())
            .unwrap_or(Path::new(""))
    }

    /// Simple import resolution fallback
    fn simple_resolve(&self, module: &str, from_idx: usize) -> Option<PathBuf> {
        let from_path = self.get_file_path(from_idx);
        let parent = from_path.parent()?;

        // Try common patterns
        for ext in &["rs", "py", "js", "ts"] {
            let candidate = parent.join(format!("{module}.{ext}"));
            if self.path_to_index.contains_key(&candidate) {
                return Some(candidate);
            }
        }

        None
    }

    /// Parallel analysis of all files
    fn parallel_analyze_files(&self, options: &SemanticOptions) -> Result<Vec<FileAnalysisResult>> {
        let cache = self.cache;
        let project_root = &self.project_root;

        // Create thread-safe error collector
        let errors = Arc::new(Mutex::new(Vec::new()));

        // Parallel analysis using rayon
        let results: Vec<FileAnalysisResult> = (0..self.nodes.len())
            .into_par_iter()
            .map(|file_idx| {
                let result = self.analyze_file_parallel(file_idx, options, cache, project_root);

                match result {
                    Ok(analysis) => analysis,
                    Err(e) => {
                        let error_msg = format!("Failed to analyze file index {file_idx}: {e}");
                        errors.lock().unwrap().push(error_msg.clone());
                        FileAnalysisResult {
                            file_index: file_idx,
                            imports: Vec::new(),
                            function_calls: Vec::new(),
                            type_references: Vec::new(),
                            content_hash: None,
                            error: Some(error_msg),
                        }
                    }
                }
            })
            .collect();

        // Print any errors that occurred
        let errors = errors.lock().unwrap();
        for error in errors.iter() {
            eprintln!("Warning: {error}");
        }

        Ok(results)
    }

    /// Analyze a single file (thread-safe version for parallel processing)
    fn analyze_file_parallel(
        &self,
        file_idx: usize,
        options: &SemanticOptions,
        cache: &FileCache,
        project_root: &Path,
    ) -> Result<FileAnalysisResult> {
        let file_path = self.get_file_path(file_idx);

        // Get the appropriate analyzer
        let analyzer = match crate::core::semantic::get_analyzer_for_file(file_path)? {
            Some(analyzer) => analyzer,
            None => {
                // No analyzer for this file type
                return Ok(FileAnalysisResult {
                    file_index: file_idx,
                    imports: Vec::new(),
                    function_calls: Vec::new(),
                    type_references: Vec::new(),
                    content_hash: None,
                    error: None,
                });
            }
        };

        // Read file content
        let content = cache.get_or_load(file_path)?;

        // Compute content hash
        let content_hash = {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            content.hash(&mut hasher);
            hasher.finish()
        };

        // Create context
        let context = SemanticContext::new(
            file_path.to_path_buf(),
            project_root.to_path_buf(),
            options.semantic_depth,
        );

        // Perform analysis
        let analysis_result = analyzer.analyze_file(file_path, &content, &context)?;

        // Build import edges with types
        let mut typed_imports = Vec::new();
        for import in &analysis_result.imports {
            let edge_type = DependencyEdgeType::Import {
                symbols: import.items.clone(),
            };
            typed_imports.push((PathBuf::from(&import.module), edge_type));
        }

        Ok(FileAnalysisResult {
            file_index: file_idx,
            imports: typed_imports,
            function_calls: analysis_result.function_calls,
            type_references: analysis_result.type_references,
            content_hash: Some(content_hash),
            error: None,
        })
    }

    /// Build dependency graph from parallel analysis results
    fn build_dependency_graph_from_results(&mut self, results: &[FileAnalysisResult]) {
        for result in results {
            let file_idx = result.file_index;

            // Store analysis results in nodes
            self.nodes[file_idx].function_calls = result.function_calls.clone();
            self.nodes[file_idx].type_references = result.type_references.clone();

            // Update RichNode in graph with content hash
            if let Some(&node_idx) = self.node_indices.get(&file_idx) {
                self.graph[node_idx].content_hash = result.content_hash;
            }

            // Process imports and build edges
            let mut resolved_imports = Vec::new();

            for (import_path, edge_type) in &result.imports {
                // Try to resolve the import
                if let Ok(Some(resolver)) =
                    crate::core::semantic::get_resolver_for_file(self.get_file_path(file_idx))
                {
                    match resolver.resolve_import(
                        &import_path.to_string_lossy(),
                        self.get_file_path(file_idx),
                        &self.project_root,
                    ) {
                        Ok(resolved) => {
                            if !resolved.is_external
                                && self.path_to_index.contains_key(&resolved.path)
                            {
                                resolved_imports.push(resolved.path.clone());

                                // Add typed edge to graph
                                if let (Some(&from_idx), Some(&to_idx)) = (
                                    self.node_indices.get(&file_idx),
                                    self.path_to_index
                                        .get(&resolved.path)
                                        .and_then(|idx| self.node_indices.get(idx)),
                                ) {
                                    self.graph.add_edge(from_idx, to_idx, edge_type.clone());
                                }
                            }
                        }
                        Err(_) => {
                            // Fallback resolution
                            if let Some(resolved) =
                                self.simple_resolve(&import_path.to_string_lossy(), file_idx)
                            {
                                resolved_imports.push(resolved);
                            }
                        }
                    }
                }
            }

            self.nodes[file_idx].imports = resolved_imports;
        }
    }

    /// Detect language from file extension
    fn detect_language(path: &Path) -> Option<String> {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| match ext {
                "rs" => "rust",
                "py" => "python",
                "js" | "mjs" => "javascript",
                "ts" | "tsx" => "typescript",
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
                _ => ext,
            })
            .map(String::from)
    }

    /// Build dependency edges in the petgraph (legacy method kept for compatibility)
    #[allow(dead_code)]
    fn build_dependency_edges(&mut self) {
        for (file_idx, node) in self.nodes.iter().enumerate() {
            for import_path in &node.imports {
                if let Some(&dep_idx) = self.path_to_index.get(import_path) {
                    if file_idx != dep_idx {
                        // Avoid self-loops
                        if let (Some(&from_node), Some(&to_node)) = (
                            self.node_indices.get(&file_idx),
                            self.node_indices.get(&dep_idx),
                        ) {
                            // Add edge from file to its dependency
                            let edge_type = DependencyEdgeType::Import {
                                symbols: Vec::new(),
                            };
                            self.graph.add_edge(from_node, to_node, edge_type);
                        }
                    }
                }
            }
        }
    }

    /// Process dependencies for a specific file
    fn process_dependencies(&mut self, _file_idx: usize, _options: &SemanticOptions) {
        // Additional processing logic can go here
        // For now, dependencies are already stored in nodes from analyze_file
    }

    /// Apply the discovered dependencies back to the files
    fn apply_to_files(&self, files: &mut [FileInfo]) {
        // Update imports and reverse dependencies
        for node in &self.nodes {
            let file = &mut files[node.file_index];

            // Update imports
            for import_path in &node.imports {
                if !file.imports.contains(import_path) {
                    file.imports.push(import_path.clone());
                }
            }

            // Update function calls
            file.function_calls = node.function_calls.clone();

            // Update type references
            file.type_references = node.type_references.clone();
        }

        // Build reverse dependencies - collect first, then update
        let mut reverse_deps: Vec<(usize, PathBuf)> = Vec::new();
        for file in files.iter() {
            for import in &file.imports {
                if let Some(&imported_idx) = self.path_to_index.get(import) {
                    let importing_path = file.path.clone();
                    reverse_deps.push((imported_idx, importing_path));
                }
            }
        }

        // Apply reverse dependencies
        for (imported_idx, importing_path) in reverse_deps {
            if !files[imported_idx].imported_by.contains(&importing_path) {
                files[imported_idx].imported_by.push(importing_path);
            }
        }

        // Process function calls to build caller relationships
        if !files.is_empty() {
            self.process_function_calls(files);
        }

        // Process type references to build type relationships
        if !files.is_empty() {
            self.process_type_references(files);
        }
    }

    /// Process function calls to determine caller relationships
    fn process_function_calls(&self, files: &mut [FileInfo]) {
        // Build module name to file index mapping
        let mut module_to_files: HashMap<String, Vec<usize>> = HashMap::new();

        for (idx, file) in files.iter().enumerate() {
            if let Some(stem) = file.path.file_stem() {
                let module_name = stem.to_string_lossy().to_string();
                module_to_files
                    .entry(module_name.clone())
                    .or_default()
                    .push(idx);

                // Handle index files
                if stem == "mod" || stem == "index" || stem == "__init__" {
                    if let Some(parent) = file.path.parent() {
                        if let Some(parent_name) = parent.file_name() {
                            let parent_module = parent_name.to_string_lossy().to_string();
                            module_to_files.entry(parent_module).or_default().push(idx);
                        }
                    }
                }
            }
        }

        // Find caller relationships - collect first, then update
        let mut relationships: Vec<(usize, usize)> = Vec::new();

        for (caller_idx, file) in files.iter().enumerate() {
            for func_call in &file.function_calls {
                if let Some(module) = &func_call.module {
                    if let Some(file_indices) = module_to_files.get(module) {
                        for &called_idx in file_indices {
                            if called_idx != caller_idx {
                                relationships.push((caller_idx, called_idx));
                            }
                        }
                    }
                }
            }
        }

        // Apply relationships
        for (caller_idx, called_idx) in relationships {
            let called_path = files[called_idx].path.clone();
            if !files[caller_idx].imports.contains(&called_path) {
                files[caller_idx].imports.push(called_path.clone());
            }

            let caller_path = files[caller_idx].path.clone();
            if !files[called_idx].imported_by.contains(&caller_path) {
                files[called_idx].imported_by.push(caller_path);
            }
        }
    }

    /// Process type references to determine type relationships
    fn process_type_references(&self, files: &mut [FileInfo]) {
        // Build type name to file index mapping
        let mut type_to_files: HashMap<String, Vec<(usize, PathBuf)>> = HashMap::new();

        for (idx, file) in files.iter().enumerate() {
            if let Some(stem) = file.path.file_stem() {
                let type_name = stem.to_string_lossy().to_string();

                // Add capitalized version
                let capitalized = capitalize_first(&type_name);
                type_to_files
                    .entry(capitalized)
                    .or_default()
                    .push((idx, file.path.clone()));

                // Add original name
                type_to_files
                    .entry(type_name)
                    .or_default()
                    .push((idx, file.path.clone()));
            }
        }

        // Update type references with definition paths
        for file in files.iter_mut() {
            for type_ref in &mut file.type_references {
                // Skip if already has definition path or is external
                if type_ref.definition_path.is_some() || type_ref.is_external {
                    continue;
                }

                // Try to find the definition file
                if let Some(file_info) = type_to_files.get(&type_ref.name) {
                    // Use the first match that's not the current file
                    for (_def_idx, def_path) in file_info {
                        if &file.path != def_path {
                            type_ref.definition_path = Some(def_path.clone());
                            break;
                        }
                    }
                }
            }
        }

        // Find type usage relationships - collect first, then update
        let mut relationships: Vec<(usize, usize)> = Vec::new();

        for (user_idx, file) in files.iter().enumerate() {
            for type_ref in &file.type_references {
                if let Some(file_info) = type_to_files.get(&type_ref.name) {
                    for &(def_idx, _) in file_info {
                        if def_idx != user_idx {
                            relationships.push((user_idx, def_idx));
                        }
                    }
                }
            }
        }

        // Apply relationships
        for (user_idx, def_idx) in relationships {
            let def_path = files[def_idx].path.clone();
            if !files[user_idx].imports.contains(&def_path) {
                files[user_idx].imports.push(def_path.clone());
            }

            let user_path = files[user_idx].path.clone();
            if !files[def_idx].imported_by.contains(&user_path) {
                files[def_idx].imported_by.push(user_path);
            }
        }
    }
}

/// Capitalize the first letter of a string
fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}
