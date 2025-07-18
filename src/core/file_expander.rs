//! File expansion logic for semantic analysis features
//!
//! This module handles expanding the file list based on semantic relationships
//! discovered during analysis (imports, type references, function calls).

use crate::cli::Config;
use crate::core::cache::FileCache;
use crate::core::semantic::path_validator::validate_import_path;
use crate::core::walker::FileInfo;
use crate::utils::error::ContextCreatorError;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Detect the project root directory using git root or fallback methods
fn detect_project_root(start_path: &Path) -> PathBuf {
    // First try to find git root
    let mut current = start_path;
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
    current = start_path;
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

    // Ultimate fallback: use the start path itself
    start_path.to_path_buf()
}

/// Expand file list based on semantic relationships
///
/// This function takes the initial set of files and expands it to include
/// files that define types, export functions, or are imported by the initial files.
pub fn expand_file_list(
    mut files_map: HashMap<PathBuf, FileInfo>,
    config: &Config,
    cache: &Arc<FileCache>,
) -> Result<HashMap<PathBuf, FileInfo>, ContextCreatorError> {
    // If no semantic features are enabled, return as-is
    if !config.trace_imports && !config.include_callers && !config.include_types {
        return Ok(files_map);
    }

    // Detect the project root for secure path validation
    let project_root = if let Some((first_path, _)) = files_map.iter().next() {
        detect_project_root(first_path)
    } else {
        // If no files, use current directory
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    };

    // Create work queue and visited set for BFS traversal
    let mut work_queue = VecDeque::new();
    let mut visited_paths = HashSet::new();
    let mut files_to_add = Vec::new();

    // Initialize with files that have semantic relationships
    for (path, file_info) in &files_map {
        visited_paths.insert(path.clone());

        // Queue files based on enabled features (depth 0 for initial files)
        if config.include_types && !file_info.type_references.is_empty() {
            work_queue.push_back((path.clone(), file_info.clone(), ExpansionReason::Types, 0));
        }
        if config.trace_imports && !file_info.imports.is_empty() {
            work_queue.push_back((path.clone(), file_info.clone(), ExpansionReason::Imports, 0));
        }
        if config.include_callers && !file_info.function_calls.is_empty() {
            work_queue.push_back((path.clone(), file_info.clone(), ExpansionReason::Callers, 0));
        }
    }

    // Process work queue
    // Note: Cycle prevention is handled by visited_paths HashSet which tracks all processed files.
    // This prevents infinite loops in cases like A→B→C→A by not revisiting already processed files.
    while let Some((source_path, source_file, reason, depth)) = work_queue.pop_front() {
        // Check if we've exceeded the semantic depth limit
        if depth >= config.semantic_depth {
            continue;
        }

        match reason {
            ExpansionReason::Types => {
                // Process type references
                for type_ref in &source_file.type_references {
                    // Skip external types
                    if type_ref.is_external {
                        continue;
                    }

                    // If we have a definition path, add it
                    if let Some(ref def_path) = type_ref.definition_path {
                        if !visited_paths.contains(def_path) && def_path.exists() {
                            // Validate the path for security using the project root
                            match validate_import_path(&project_root, def_path) {
                                Ok(validated_path) => {
                                    visited_paths.insert(validated_path.clone());

                                    // Create FileInfo for the definition file
                                    let mut file_info =
                                        create_file_info_for_path(&validated_path, &source_path)?;

                                    // Queue for next depth level if it has type references
                                    if depth + 1 < config.semantic_depth
                                        && !file_info.type_references.is_empty()
                                    {
                                        // We'll need to perform semantic analysis on this file first
                                        // For now, mark it for addition
                                        file_info.imported_by.push(source_path.clone());
                                    }

                                    files_to_add.push((validated_path.clone(), file_info));
                                }
                                Err(_) => {
                                    // Path validation failed, skip this file
                                }
                            }
                        }
                    } else {
                        // Try to find the type definition file
                        // Extract just the module name from the full module path (e.g., "traits" from "traits::Repository")
                        let module_name = type_ref
                            .module
                            .as_deref()
                            .map(|m| m.split("::").next().unwrap_or(m));

                        if let Some(def_path) = find_type_definition_file(
                            &type_ref.name,
                            module_name,
                            &source_path,
                            cache,
                        ) {
                            if !visited_paths.contains(&def_path) {
                                // Validate the path for security using the project root
                                match validate_import_path(&project_root, &def_path) {
                                    Ok(validated_path) => {
                                        visited_paths.insert(validated_path.clone());

                                        // Create FileInfo for the definition file
                                        let mut file_info = create_file_info_for_path(
                                            &validated_path,
                                            &source_path,
                                        )?;

                                        // Queue for next depth level if it has type references
                                        if depth + 1 < config.semantic_depth
                                            && !file_info.type_references.is_empty()
                                        {
                                            // We'll need to perform semantic analysis on this file first
                                            // For now, mark it for addition
                                            file_info.imported_by.push(source_path.clone());
                                        }

                                        files_to_add.push((validated_path, file_info));
                                    }
                                    Err(_) => {
                                        // Path validation failed, skip this file
                                    }
                                }
                            }
                        }
                    }
                }
            }
            ExpansionReason::Imports => {
                // Process imports - already handled by semantic analysis
                // The imported files should already be in the imported_by relationships
            }
            ExpansionReason::Callers => {
                // Process function calls - this would require finding files that define the called functions
                // For now, this is a placeholder for future enhancement
            }
        }
    }

    // Add new files to the map
    for (path, file_info) in files_to_add {
        files_map.insert(path, file_info);
    }

    // Update imported_by relationships for proper prioritization
    update_import_relationships(&mut files_map);

    Ok(files_map)
}

/// Reason for expanding to include a file
#[derive(Debug, Clone, Copy)]
enum ExpansionReason {
    Types,
    Imports,
    Callers,
}

/// Create a basic FileInfo for a newly discovered file
fn create_file_info_for_path(
    path: &PathBuf,
    source_path: &Path,
) -> Result<FileInfo, ContextCreatorError> {
    use crate::utils::file_ext::FileType;
    use std::fs;

    let metadata = fs::metadata(path)?;
    let file_type = FileType::from_path(path);

    // Calculate relative path from common ancestor
    let relative_path = path
        .strip_prefix(common_ancestor(path, source_path))
        .unwrap_or(path)
        .to_path_buf();

    Ok(FileInfo {
        path: path.clone(),
        relative_path,
        size: metadata.len(),
        file_type,
        priority: 1.0, // Default priority, will be adjusted by prioritizer
        imports: Vec::new(),
        imported_by: vec![source_path.to_path_buf()], // Track who caused this file to be included
        function_calls: Vec::new(),
        type_references: Vec::new(),
    })
}

/// Find the lowest common ancestor (LCA) of two paths using a proper set-based approach
fn common_ancestor(path1: &Path, path2: &Path) -> PathBuf {
    use std::collections::HashSet;

    // Collect all ancestors of path1 into a set for efficient lookup
    let ancestors1: HashSet<&Path> = path1.ancestors().collect();

    // Find the first ancestor of path2 that is also in ancestors1
    // This will be the lowest common ancestor since ancestors() returns in order from leaf to root
    for ancestor in path2.ancestors() {
        if ancestors1.contains(ancestor) {
            return ancestor.to_path_buf();
        }
    }

    // If no common ancestor found (shouldn't happen in normal filesystem),
    // fallback to appropriate root for the platform
    #[cfg(windows)]
    {
        // On Windows, try to get the root from one of the paths
        if let Some(root) = path1.ancestors().last() {
            root.to_path_buf()
        } else {
            PathBuf::from("C:\\")
        }
    }
    #[cfg(not(windows))]
    {
        PathBuf::from("/")
    }
}

/// Check if a file contains a definition for a given type name using AST parsing.
fn file_contains_definition(path: &Path, content: &str, type_name: &str) -> bool {
    // Determine the language from the file extension.
    let language = match path.extension().and_then(|s| s.to_str()) {
        Some("rs") => Some(tree_sitter_rust::language()),
        Some("py") => Some(tree_sitter_python::language()),
        Some("ts") | Some("tsx") => Some(tree_sitter_typescript::language_typescript()),
        Some("js") | Some("jsx") => Some(tree_sitter_javascript::language()),
        _ => None,
    };

    if let Some(language) = language {
        let mut parser = tree_sitter::Parser::new();
        if parser.set_language(language).is_err() {
            return false;
        }

        if let Some(tree) = parser.parse(content, None) {
            // Language-specific queries for type definitions (without predicates)
            let query_text = match path.extension().and_then(|s| s.to_str()) {
                Some("rs") => {
                    r#"
                    [
                      (struct_item name: (type_identifier) @name)
                      (enum_item name: (type_identifier) @name)
                      (trait_item name: (type_identifier) @name)
                      (type_item name: (type_identifier) @name)
                      (union_item name: (type_identifier) @name)
                    ]
                "#
                }
                Some("py") => {
                    r#"
                    [
                      (class_definition name: (identifier) @name)
                      (function_definition name: (identifier) @name)
                    ]
                "#
                }
                Some("ts") | Some("tsx") => {
                    r#"
                    [
                      (interface_declaration name: (type_identifier) @name)
                      (type_alias_declaration name: (type_identifier) @name)
                      (class_declaration name: (type_identifier) @name)
                      (enum_declaration name: (identifier) @name)
                    ]
                "#
                }
                Some("js") | Some("jsx") => {
                    r#"
                    [
                      (class_declaration name: (identifier) @name)
                      (function_declaration name: (identifier) @name)
                    ]
                "#
                }
                _ => return false,
            };

            if let Ok(query) = tree_sitter::Query::new(language, query_text) {
                let mut cursor = tree_sitter::QueryCursor::new();
                let matches = cursor.matches(&query, tree.root_node(), content.as_bytes());

                // Check each match to see if the captured name matches our target type
                for m in matches {
                    for capture in m.captures {
                        if let Ok(captured_text) = capture.node.utf8_text(content.as_bytes()) {
                            if captured_text == type_name {
                                return true;
                            }
                        }
                    }
                }
                return false;
            }
        }
    }
    false
}

/// Find a type definition file by searching nearby paths
fn find_type_definition_file(
    type_name: &str,
    module_name: Option<&str>,
    source_file: &Path,
    cache: &FileCache,
) -> Option<PathBuf> {
    // Get the directory of the source file
    let source_dir = source_file.parent()?;

    // Also get the project root (go up to find src directory)
    let mut project_root = source_dir;
    while let Some(parent) = project_root.parent() {
        // If we find a Cargo.toml or src directory, the parent is likely the project root
        if parent.join("Cargo.toml").exists() || parent.join("src").exists() {
            project_root = parent;
            break;
        }
        // If current dir is named "src", its parent is likely the project root
        if project_root.file_name() == Some(std::ffi::OsStr::new("src")) {
            project_root = parent;
            break;
        }
        project_root = parent;
    }

    // Convert type name to lowercase for file matching
    let type_name_lower = type_name.to_lowercase();

    // Common patterns for type definition files
    let mut patterns = vec![
        // Direct file name matches
        format!("{type_name_lower}.rs"),
        format!("{type_name_lower}.py"),
        format!("{type_name_lower}.ts"),
        format!("{type_name_lower}.js"),
        format!("{type_name_lower}.tsx"),
        format!("{type_name_lower}.jsx"),
        // Types files
        "types.rs".to_string(),
        "types.py".to_string(),
        "types.ts".to_string(),
        "types.js".to_string(),
        // Module files
        "mod.rs".to_string(),
        "index.ts".to_string(),
        "index.js".to_string(),
        "__init__.py".to_string(),
        // Common type definition patterns
        format!("{type_name_lower}_types.rs"),
        format!("{type_name_lower}_type.rs"),
        format!("{type_name_lower}s.rs"), // plural form
    ];

    // If we have a module name, add module-based patterns
    if let Some(module) = module_name {
        let module_lower = module.to_lowercase();
        patterns.insert(0, format!("{module_lower}.rs"));
        patterns.insert(1, format!("{module_lower}.py"));
        patterns.insert(2, format!("{module_lower}.ts"));
        patterns.insert(3, format!("{module_lower}.js"));
        patterns.insert(4, format!("{module_lower}.tsx"));
        patterns.insert(5, format!("{module_lower}.jsx"));
        patterns.insert(6, format!("{module}.rs")); // Also try original case
        patterns.insert(7, format!("{module}.py"));
        patterns.insert(8, format!("{module}.ts"));
        patterns.insert(9, format!("{module}.js"));
    }

    // Search in current directory first
    for pattern in &patterns {
        let candidate = source_dir.join(pattern);
        if candidate.exists() {
            // Read the file to verify it contains the type definition
            if let Ok(content) = cache.get_or_load(&candidate) {
                // Use AST-based validation to check for type definitions
                if file_contains_definition(&candidate, &content, type_name) {
                    return Some(candidate);
                }
            }
        }
    }

    // Search in parent directory
    if let Some(parent_dir) = source_dir.parent() {
        for pattern in &patterns {
            let candidate = parent_dir.join(pattern);
            if candidate.exists() {
                if let Ok(content) = cache.get_or_load(&candidate) {
                    if file_contains_definition(&candidate, &content, type_name) {
                        return Some(candidate);
                    }
                }
            }
        }
    }

    // Search in common module directories relative to project root
    let search_dirs = vec![
        project_root.to_path_buf(),
        project_root.join("src"),
        project_root.join("src/models"),
        project_root.join("src/types"),
        project_root.join("shared"),
        project_root.join("shared/types"),
        project_root.join("lib"),
        project_root.join("domain"),
        source_dir.join("models"),
        source_dir.join("types"),
    ];

    for search_dir in search_dirs {
        if search_dir.exists() {
            for pattern in &patterns {
                let candidate = search_dir.join(pattern);
                if candidate.exists() {
                    if let Ok(content) = cache.get_or_load(&candidate) {
                        if file_contains_definition(&candidate, &content, type_name) {
                            return Some(candidate);
                        }
                    }
                }
            }
        }
    }

    None
}

/// Update import relationships after expansion
fn update_import_relationships(files_map: &mut HashMap<PathBuf, FileInfo>) {
    // Build a map of which files import which
    let mut import_map: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();

    for (path, file_info) in files_map.iter() {
        for import_path in &file_info.imports {
            // Track which files import which
            import_map
                .entry(import_path.clone())
                .or_default()
                .push(path.clone());
        }
    }

    // Update imported_by fields
    for (imported_path, importers) in import_map {
        if let Some(file_info) = files_map.get_mut(&imported_path) {
            file_info.imported_by.extend(importers);
            file_info.imported_by.sort();
            file_info.imported_by.dedup();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::file_ext::FileType;

    #[test]
    fn test_no_expansion_when_disabled() {
        let mut files_map = HashMap::new();
        files_map.insert(
            PathBuf::from("test.rs"),
            FileInfo {
                path: PathBuf::from("test.rs"),
                relative_path: PathBuf::from("test.rs"),
                size: 100,
                file_type: FileType::Rust,
                priority: 1.0,
                imports: Vec::new(),
                imported_by: Vec::new(),
                function_calls: Vec::new(),
                type_references: Vec::new(),
            },
        );

        let config = Config {
            trace_imports: false,
            include_callers: false,
            include_types: false,
            ..Default::default()
        };

        let cache = Arc::new(FileCache::new());
        let result = expand_file_list(files_map.clone(), &config, &cache).unwrap();

        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_common_ancestor() {
        #[cfg(windows)]
        {
            let path1 = PathBuf::from("C:\\Users\\user\\project\\src\\main.rs");
            let path2 = PathBuf::from("C:\\Users\\user\\project\\lib\\util.rs");
            let ancestor = common_ancestor(&path1, &path2);
            assert_eq!(ancestor, PathBuf::from("C:\\Users\\user\\project"));

            // Test with same directory
            let path3 = PathBuf::from("C:\\Users\\user\\project\\main.rs");
            let path4 = PathBuf::from("C:\\Users\\user\\project\\util.rs");
            let ancestor2 = common_ancestor(&path3, &path4);
            assert_eq!(ancestor2, PathBuf::from("C:\\Users\\user\\project"));

            // Test with nested paths
            let path5 = PathBuf::from("C:\\Users\\user\\project\\src\\deep\\nested\\file.rs");
            let path6 = PathBuf::from("C:\\Users\\user\\project\\src\\main.rs");
            let ancestor3 = common_ancestor(&path5, &path6);
            assert_eq!(ancestor3, PathBuf::from("C:\\Users\\user\\project\\src"));

            // Test with completely different drives
            let path7 = PathBuf::from("C:\\Program Files\\tool");
            let path8 = PathBuf::from("D:\\Users\\user\\file");
            let ancestor4 = common_ancestor(&path7, &path8);
            // Should fall back to C:\ (root from first path)
            assert_eq!(ancestor4, PathBuf::from("C:\\"));
        }

        #[cfg(not(windows))]
        {
            let path1 = PathBuf::from("/home/user/project/src/main.rs");
            let path2 = PathBuf::from("/home/user/project/lib/util.rs");
            let ancestor = common_ancestor(&path1, &path2);
            assert_eq!(ancestor, PathBuf::from("/home/user/project"));

            // Test with same directory
            let path3 = PathBuf::from("/home/user/project/main.rs");
            let path4 = PathBuf::from("/home/user/project/util.rs");
            let ancestor2 = common_ancestor(&path3, &path4);
            assert_eq!(ancestor2, PathBuf::from("/home/user/project"));

            // Test with nested paths
            let path5 = PathBuf::from("/home/user/project/src/deep/nested/file.rs");
            let path6 = PathBuf::from("/home/user/project/src/main.rs");
            let ancestor3 = common_ancestor(&path5, &path6);
            assert_eq!(ancestor3, PathBuf::from("/home/user/project/src"));

            // Test with completely different top-level directories
            let path7 = PathBuf::from("/usr/local/bin/tool");
            let path8 = PathBuf::from("/home/user/file");
            let ancestor4 = common_ancestor(&path7, &path8);
            assert_eq!(ancestor4, PathBuf::from("/"));

            // Test with one path being an ancestor of the other
            let path9 = PathBuf::from("/home/user/project");
            let path10 = PathBuf::from("/home/user/project/src/main.rs");
            let ancestor5 = common_ancestor(&path9, &path10);
            assert_eq!(ancestor5, PathBuf::from("/home/user/project"));
        }
    }

    #[test]
    fn test_file_contains_definition() {
        // Debug: Test simple tree-sitter parsing first
        let rust_content = r#"
            pub struct MyStruct {
                field1: String,
                field2: i32,
            }
            
            pub enum MyEnum {
                Variant1,
                Variant2(String),
            }
            
            pub trait MyTrait {
                fn method(&self);
            }
        "#;

        let mut parser = tree_sitter::Parser::new();
        parser.set_language(tree_sitter_rust::language()).unwrap();
        let tree = parser.parse(rust_content, None).unwrap();

        // Test simple query without predicate first
        let simple_query = r#"
            [
              (struct_item name: (type_identifier) @name)
              (enum_item name: (type_identifier) @name)
              (trait_item name: (type_identifier) @name)
            ]
        "#;

        let query = tree_sitter::Query::new(tree_sitter_rust::language(), simple_query).unwrap();
        let mut cursor = tree_sitter::QueryCursor::new();
        let matches: Vec<_> = cursor
            .matches(&query, tree.root_node(), rust_content.as_bytes())
            .collect();

        // Should find 3 definitions: MyStruct, MyEnum, MyTrait
        assert_eq!(matches.len(), 3, "Should find exactly 3 type definitions");

        // Test the actual function with a simpler approach
        let rust_path = PathBuf::from("test.rs");
        assert!(file_contains_definition(
            &rust_path,
            rust_content,
            "MyStruct"
        ));
        assert!(file_contains_definition(&rust_path, rust_content, "MyEnum"));
        assert!(file_contains_definition(
            &rust_path,
            rust_content,
            "MyTrait"
        ));
        assert!(!file_contains_definition(
            &rust_path,
            rust_content,
            "NonExistent"
        ));

        // Test Python class definition
        let python_content = r#"
            class MyClass:
                def __init__(self):
                    pass
                    
            def my_function():
                pass
        "#;

        let python_path = PathBuf::from("test.py");
        assert!(file_contains_definition(
            &python_path,
            python_content,
            "MyClass"
        ));
        assert!(file_contains_definition(
            &python_path,
            python_content,
            "my_function"
        ));
        assert!(!file_contains_definition(
            &python_path,
            python_content,
            "NonExistent"
        ));

        // Test TypeScript interface definition
        let typescript_content = r#"
            export interface MyInterface {
                prop1: string;
                prop2: number;
            }
            
            export type MyType = string | number;
            
            export class MyClass {
                constructor() {}
            }
        "#;

        let typescript_path = PathBuf::from("test.ts");
        assert!(file_contains_definition(
            &typescript_path,
            typescript_content,
            "MyInterface"
        ));
        assert!(file_contains_definition(
            &typescript_path,
            typescript_content,
            "MyType"
        ));
        assert!(file_contains_definition(
            &typescript_path,
            typescript_content,
            "MyClass"
        ));
        assert!(!file_contains_definition(
            &typescript_path,
            typescript_content,
            "NonExistent"
        ));
    }

    #[test]
    fn test_query_engine_rust() {
        use crate::core::semantic::query_engine::QueryEngine;
        use tree_sitter::Parser;

        let rust_content = r#"
            use model::{Account, DatabaseFactory, Rule, RuleLevel, RuleName};
            
            pub fn create(
                database: &mut dyn DatabaseFactory,
                account: &Account,
                rule_name: &RuleName,
            ) -> Result<Rule, Box<dyn std::error::Error>> {
                Ok(Rule::new())
            }
        "#;

        let language = tree_sitter_rust::language();
        let query_engine = QueryEngine::new(language, "rust").unwrap();

        let mut parser = Parser::new();
        parser.set_language(tree_sitter_rust::language()).unwrap();

        let result = query_engine
            .analyze_with_parser(&mut parser, rust_content)
            .unwrap();

        println!("Imports found: {:?}", result.imports);
        println!("Type references found: {:?}", result.type_references);

        // Should find imports
        assert!(!result.imports.is_empty(), "Should find imports");

        // Should find type references from the imports
        assert!(
            !result.type_references.is_empty(),
            "Should find type references"
        );

        // Check specific types
        let type_names: Vec<&str> = result
            .type_references
            .iter()
            .map(|t| t.name.as_str())
            .collect();
        assert!(
            type_names.contains(&"DatabaseFactory"),
            "Should find DatabaseFactory type"
        );
        assert!(type_names.contains(&"Account"), "Should find Account type");
        assert!(
            type_names.contains(&"RuleName"),
            "Should find RuleName type"
        );
    }
}
