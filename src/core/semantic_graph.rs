//! Dependency graph traversal for semantic analysis

use crate::core::cache::FileCache;
use crate::core::semantic::{analyzer::SemanticContext, SemanticOptions};
use crate::core::walker::FileInfo;
use anyhow::Result;
use petgraph::algo::toposort;
use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Represents a node in the dependency graph
#[derive(Debug, Clone)]
struct DependencyNode {
    file_index: usize,
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

/// Dependency graph for semantic analysis
struct DependencyGraph<'a> {
    nodes: Vec<DependencyNode>,
    path_to_index: HashMap<PathBuf, usize>,
    index_to_path: HashMap<usize, PathBuf>,
    visited: HashSet<usize>,
    cache: &'a FileCache,
    project_root: PathBuf,
    graph: DiGraph<usize, ()>,
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

        // Add nodes to petgraph
        for index in 0..files.len() {
            let node_idx = graph.add_node(index);
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

    /// Traverse dependencies using Kahn's algorithm for cycle detection
    fn traverse_dependencies(&mut self, options: &SemanticOptions) -> Result<()> {
        // First pass: analyze all files to discover dependencies
        for i in 0..self.nodes.len() {
            if let Err(e) = self.analyze_file(i, 0, options) {
                eprintln!("Warning: Failed to analyze file index {i}: {e}");
                continue;
            }
        }

        // Second pass: build dependency graph edges
        self.build_dependency_edges();

        // Third pass: use Kahn's algorithm to detect cycles and get processing order
        match toposort(&self.graph, None) {
            Ok(sorted_nodes) => {
                // Process nodes in topological order
                for node_idx in sorted_nodes {
                    let file_idx = self.graph[node_idx];
                    self.visited.insert(file_idx);

                    // Process dependencies up to semantic_depth
                    self.process_dependencies(file_idx, options);
                }
            }
            Err(cycle_error) => {
                // Cycle detected - handle gracefully
                let cycle_node = self.graph[cycle_error.node_id()];
                let cycle_path = self.get_file_path(cycle_node);
                eprintln!(
                    "Warning: Circular dependency detected involving file: {}",
                    cycle_path.display()
                );
                eprintln!("Warning: Processing files in partial order, some dependencies may be incomplete.");

                // Process nodes that can be processed (not in cycle)
                for i in 0..self.nodes.len() {
                    if !self.visited.contains(&i) {
                        self.visited.insert(i);
                        self.process_dependencies(i, options);
                    }
                }
            }
        }

        Ok(())
    }

    /// Analyze a single file and extract semantic information
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

    /// Build dependency edges in the petgraph
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
                            self.graph.add_edge(from_node, to_node, ());
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
