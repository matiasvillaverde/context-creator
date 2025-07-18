//! Dependency graph coordination for semantic analysis
//!
//! This module has been refactored to use separate modules for different responsibilities:
//! - GraphBuilder: Constructs the dependency graph
//! - GraphTraverser: Handles graph traversal algorithms
//! - ParallelAnalyzer: Manages parallel file analysis
//!
//! This module now serves as a thin coordination layer that maintains backward compatibility.

use crate::core::cache::FileCache;
use crate::core::semantic::cycle_detector::{CycleResolution, TarjanCycleDetector};
use crate::core::semantic::graph_builder::GraphBuilder;
use crate::core::semantic::graph_traverser::GraphTraverser;
use crate::core::semantic::parallel_analyzer::{AnalysisOptions, ParallelAnalyzer};
use crate::core::semantic::SemanticOptions;
use crate::core::walker::FileInfo;
use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

/// Performs sophisticated semantic analysis with proper dependency graph traversal
/// This is the main entry point that maintains backward compatibility
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

    // Detect project root from first file
    let project_root = if let Some(first_file) = files.first() {
        detect_project_root(&first_file.path)
    } else {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    };

    // Step 1: Parallel file analysis
    let analyzer = ParallelAnalyzer::new(cache);
    let analysis_options = AnalysisOptions {
        semantic_depth: semantic_options.semantic_depth,
        trace_imports: semantic_options.trace_imports,
        include_types: semantic_options.include_types,
        include_functions: semantic_options.include_callers,
    };

    let file_paths: Vec<_> = files.iter().map(|f| f.path.clone()).collect();
    let valid_files: HashSet<PathBuf> = files
        .iter()
        .map(|f| f.path.canonicalize().unwrap_or_else(|_| f.path.clone()))
        .collect();
    let analysis_results =
        analyzer.analyze_files(&file_paths, &project_root, &analysis_options, &valid_files)?;

    // Step 2: Build dependency graph
    let builder = GraphBuilder::new();
    let (mut graph, node_map) = builder.build(files)?;

    // Create path to index mapping
    let path_to_index: HashMap<PathBuf, usize> = files
        .iter()
        .enumerate()
        .map(|(i, f)| (f.path.clone(), i))
        .collect();

    // Add edges from analysis results
    builder.build_edges_from_analysis(&mut graph, &analysis_results, &path_to_index, &node_map);

    // Step 3: Detect and handle cycles
    let mut cycle_detector = TarjanCycleDetector::new();
    let cycle_result = cycle_detector.detect_cycles(&graph);

    if cycle_result.has_cycles {
        // Report all detected cycles
        eprintln!(
            "Warning: {} circular dependencies detected:",
            cycle_result.cycles.len()
        );
        for (i, cycle) in cycle_result.cycles.iter().enumerate() {
            let cycle_num = i + 1;
            eprintln!("\nCycle {cycle_num}:");
            for &node_idx in cycle {
                let node = &graph[node_idx];
                let path = node.path.display();
                eprintln!("  - {path}");
            }
        }
        eprintln!(
            "\nWarning: Processing files in partial order, some dependencies may be incomplete."
        );
    }

    // Step 4: Traverse graph and apply results
    let _traverser = GraphTraverser::new();
    let resolution = cycle_detector.handle_cycles(&graph, cycle_result.cycles);

    match resolution {
        CycleResolution::PartialOrder(node_order) => {
            // Process nodes in the computed order
            for &node_idx in &node_order {
                let rich_node = &graph[node_idx];
                let file_idx = rich_node.file_index;

                // Apply analysis results to files
                if file_idx < analysis_results.len() {
                    let result = &analysis_results[file_idx];
                    let file = &mut files[file_idx];

                    // Update function calls
                    file.function_calls = result.function_calls.clone();

                    // Update type references
                    file.type_references = result.type_references.clone();
                }
            }
        }
        CycleResolution::BreakEdges(_) => {
            unimplemented!("Edge breaking strategy not yet implemented");
        }
        CycleResolution::MergeComponents(_) => {
            unimplemented!("Component merging strategy not yet implemented");
        }
    }

    // Step 5: Apply import relationships
    apply_import_relationships(files, &analysis_results, &path_to_index);

    // Step 6: Process function calls and type references
    process_function_calls(files);
    process_type_references(files);

    Ok(())
}

/// Detect the project root directory
fn detect_project_root(start_path: &std::path::Path) -> PathBuf {
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

    // Fallback: Look for common project markers
    current = start_path.parent().unwrap_or(start_path);
    loop {
        if current.join("Cargo.toml").exists()
            || current.join("package.json").exists()
            || current.join("pyproject.toml").exists()
            || current.join("setup.py").exists()
        {
            return current.to_path_buf();
        }

        if let Some(parent) = current.parent() {
            current = parent;
        } else {
            break;
        }
    }

    // Ultimate fallback
    start_path.parent().unwrap_or(start_path).to_path_buf()
}

/// Apply import relationships from analysis results
fn apply_import_relationships(
    files: &mut [FileInfo],
    analysis_results: &[crate::core::semantic::dependency_types::FileAnalysisResult],
    path_to_index: &HashMap<PathBuf, usize>,
) {
    // First pass: collect all imports
    for result in analysis_results {
        if result.file_index < files.len() {
            let file = &mut files[result.file_index];

            // Add resolved imports
            for (import_path, _) in &result.imports {
                if !file.imports.contains(import_path) {
                    file.imports.push(import_path.clone());
                }
            }
        }
    }

    // Second pass: build reverse dependencies
    let mut reverse_deps: Vec<(usize, PathBuf)> = Vec::new();
    for file in files.iter() {
        for import in &file.imports {
            if let Some(&imported_idx) = path_to_index.get(import) {
                reverse_deps.push((imported_idx, file.path.clone()));
            }
        }
    }

    // Apply reverse dependencies
    for (imported_idx, importing_path) in reverse_deps {
        if imported_idx < files.len() && !files[imported_idx].imported_by.contains(&importing_path)
        {
            files[imported_idx].imported_by.push(importing_path);
        }
    }
}

/// Process function calls to determine caller relationships
fn process_function_calls(files: &mut [FileInfo]) {
    use std::collections::HashMap;

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

    // Find caller relationships
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
fn process_type_references(files: &mut [FileInfo]) {
    use std::collections::HashMap;

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

    // Find type usage relationships
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

/// Capitalize the first letter of a string
fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}
