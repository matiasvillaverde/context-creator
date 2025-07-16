//! Semantic analyzer for Python

use crate::core::semantic::{
    analyzer::{
        AnalysisResult, FunctionCall, Import, LanguageAnalyzer, SemanticContext, SemanticResult,
        TypeReference,
    },
    resolver::{ModuleResolver, ResolvedPath, ResolverUtils},
};
use crate::utils::error::CodeDigestError;
use std::path::Path;
use tree_sitter::{Node, Parser, TreeCursor};

#[allow(clippy::new_without_default)]
pub struct PythonAnalyzer {}

impl PythonAnalyzer {
    pub fn new() -> Self {
        Self {}
    }
}

impl LanguageAnalyzer for PythonAnalyzer {
    fn language_name(&self) -> &'static str {
        "Python"
    }

    fn analyze_file(
        &self,
        _path: &Path,
        content: &str,
        _context: &SemanticContext,
    ) -> SemanticResult<AnalysisResult> {
        let mut result = AnalysisResult::default();

        // Create a new parser for this analysis
        let mut parser = Parser::new();
        parser.set_language(tree_sitter_python::language()).unwrap();

        // Parse the content with tree-sitter
        let tree = match parser.parse(content, None) {
            Some(tree) => tree,
            None => return Ok(result), // Return empty result if parsing fails
        };

        let root_node = tree.root_node();
        let mut cursor = root_node.walk();

        // Walk the tree and extract semantic information
        extract_semantic_info(&mut cursor, content, &mut result);

        Ok(result)
    }

    fn can_handle_extension(&self, extension: &str) -> bool {
        extension == "py"
    }

    fn supported_extensions(&self) -> Vec<&'static str> {
        vec!["py"]
    }
}

/// Extract semantic information from the AST
fn extract_semantic_info(cursor: &mut TreeCursor, source: &str, result: &mut AnalysisResult) {
    loop {
        let node = cursor.node();

        match node.kind() {
            // Import handling
            "import_statement" => {
                if let Some(imports) = parse_import_statement(&node, source) {
                    result.imports.extend(imports);
                }
            }
            "import_from_statement" => {
                if let Some(import) = parse_import_from_statement(&node, source) {
                    result.imports.push(import);
                }
            }

            // Function call handling
            "call" => {
                if let Some(call) = parse_function_call(&node, source) {
                    result.function_calls.push(call);
                }
            }

            // Type reference handling - Python type hints
            "type" => {
                if let Some(type_ref) = parse_type_annotation(&node, source) {
                    result.type_references.push(type_ref);
                }
            }
            "annotation" => {
                if let Some(type_ref) = parse_type_annotation(&node, source) {
                    result.type_references.push(type_ref);
                }
            }

            _ => {}
        }

        // Traverse children
        if cursor.goto_first_child() {
            extract_semantic_info(cursor, source, result);
            cursor.goto_parent();
        }

        if !cursor.goto_next_sibling() {
            break;
        }
    }
}

/// Parse an import statement (e.g., import os, sys)
fn parse_import_statement(node: &Node, source: &str) -> Option<Vec<Import>> {
    let start = node.start_position();
    let line = start.row + 1;
    let mut imports = Vec::new();

    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            match child.kind() {
                "dotted_name" => {
                    if let Ok(module_name) = child.utf8_text(source.as_bytes()) {
                        imports.push(Import {
                            module: module_name.to_string(),
                            items: vec![],
                            is_relative: false,
                            line,
                        });
                    }
                }
                "aliased_import" => {
                    // Handle "import foo as bar"
                    let mut alias_cursor = child.walk();
                    if alias_cursor.goto_first_child() {
                        if let Some(name_node) = alias_cursor.node().child(0) {
                            if name_node.kind() == "dotted_name" {
                                if let Ok(module_name) = name_node.utf8_text(source.as_bytes()) {
                                    imports.push(Import {
                                        module: module_name.to_string(),
                                        items: vec![],
                                        is_relative: false,
                                        line,
                                    });
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }

    if imports.is_empty() {
        None
    } else {
        Some(imports)
    }
}

/// Parse a from import statement (e.g., from os import path)
fn parse_import_from_statement(node: &Node, source: &str) -> Option<Import> {
    let start = node.start_position();
    let line = start.row + 1;

    let mut module_name = String::new();
    let mut imported_items = Vec::new();
    let mut is_relative = false;

    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            match child.kind() {
                "relative_import" => {
                    is_relative = true;
                    // Check for module name after dots
                    let mut rel_cursor = child.walk();
                    if rel_cursor.goto_first_child() {
                        loop {
                            if rel_cursor.node().kind() == "dotted_name" {
                                if let Ok(name) = rel_cursor.node().utf8_text(source.as_bytes()) {
                                    module_name = name.to_string();
                                }
                            }
                            if !rel_cursor.goto_next_sibling() {
                                break;
                            }
                        }
                    }
                }
                "dotted_name" => {
                    if module_name.is_empty() {
                        if let Ok(name) = child.utf8_text(source.as_bytes()) {
                            module_name = name.to_string();
                        }
                    }
                }
                "import_from_as_names" => {
                    // Extract imported items
                    imported_items.extend(extract_import_names(&child, source));
                }
                "identifier" => {
                    // Single imported item
                    if let Ok(name) = child.utf8_text(source.as_bytes()) {
                        if name != "from" && name != "import" {
                            imported_items.push(name.to_string());
                        }
                    }
                }
                "*" => {
                    // from module import *
                    imported_items.push("*".to_string());
                }
                _ => {}
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }

    if module_name.is_empty() && !is_relative {
        return None;
    }

    Some(Import {
        module: module_name,
        items: imported_items,
        is_relative,
        line,
    })
}

/// Extract import names from import_from_as_names node
fn extract_import_names(node: &Node, source: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut cursor = node.walk();

    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            match child.kind() {
                "identifier" => {
                    if let Ok(name) = child.utf8_text(source.as_bytes()) {
                        names.push(name.to_string());
                    }
                }
                "aliased_import" => {
                    // Handle "import foo as bar" - we want the original name
                    if let Some(name_node) = child.child(0) {
                        if let Ok(name) = name_node.utf8_text(source.as_bytes()) {
                            names.push(name.to_string());
                        }
                    }
                }
                _ => {}
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }

    names
}

/// Parse a function call
fn parse_function_call(node: &Node, source: &str) -> Option<FunctionCall> {
    let start = node.start_position();
    let line = start.row + 1;

    // Get the function being called
    let mut cursor = node.walk();
    if !cursor.goto_first_child() {
        return None;
    }

    let function_node = cursor.node();
    let (function_name, module) = match function_node.kind() {
        "identifier" => {
            if let Ok(name) = function_node.utf8_text(source.as_bytes()) {
                (name.to_string(), None)
            } else {
                return None;
            }
        }
        "attribute" => {
            // Handle method calls like obj.method()
            parse_attribute_call(&function_node, source)?
        }
        _ => return None,
    };

    Some(FunctionCall {
        name: function_name,
        module,
        line,
    })
}

/// Parse an attribute call (e.g., os.path.join())
fn parse_attribute_call(node: &Node, source: &str) -> Option<(String, Option<String>)> {
    let mut parts = Vec::new();
    collect_attribute_parts(node, source, &mut parts);

    if parts.is_empty() {
        return None;
    }

    // The last part is the function name
    let function_name = parts.pop()?;

    // The rest is the module path
    let module = if parts.is_empty() {
        None
    } else {
        Some(parts.join("."))
    };

    Some((function_name, module))
}

/// Recursively collect parts of an attribute expression
fn collect_attribute_parts(node: &Node, source: &str, parts: &mut Vec<String>) {
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            match child.kind() {
                "identifier" => {
                    if let Ok(name) = child.utf8_text(source.as_bytes()) {
                        parts.push(name.to_string());
                    }
                }
                "attribute" => {
                    collect_attribute_parts(&child, source, parts);
                }
                _ => {}
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
}

/// Parse a type annotation
fn parse_type_annotation(node: &Node, source: &str) -> Option<TypeReference> {
    let start = node.start_position();
    let line = start.row + 1;

    // Skip if this is a type definition
    if let Some(parent) = node.parent() {
        match parent.kind() {
            "class_definition" | "type_alias_statement" => {
                return None;
            }
            _ => {}
        }
    }

    let type_name = extract_type_name(node, source)?;

    // Try to extract module from qualified types like typing.List
    let (name, module) = if type_name.contains('.') {
        let parts: Vec<&str> = type_name.rsplitn(2, '.').collect();
        if parts.len() == 2 {
            (parts[0].to_string(), Some(parts[1].to_string()))
        } else {
            (type_name, None)
        }
    } else {
        (type_name, None)
    };

    Some(TypeReference { name, module, line })
}

/// Extract type name from a type node
fn extract_type_name(node: &Node, source: &str) -> Option<String> {
    match node.kind() {
        "type" | "annotation" => {
            // Look for the actual type identifier within
            let mut cursor = node.walk();
            if cursor.goto_first_child() {
                return extract_type_name(&cursor.node(), source);
            }
        }
        "identifier" => {
            return node
                .utf8_text(source.as_bytes())
                .ok()
                .map(|s| s.to_string());
        }
        "attribute" => {
            // Handle qualified types like typing.List
            let mut parts = Vec::new();
            collect_attribute_parts(node, source, &mut parts);
            if !parts.is_empty() {
                return Some(parts.join("."));
            }
        }
        "subscript" => {
            // Handle generic types like List[str]
            if let Some(value_node) = node.child(0) {
                return extract_type_name(&value_node, source);
            }
        }
        _ => {}
    }
    None
}

pub struct PythonModuleResolver;

impl ModuleResolver for PythonModuleResolver {
    fn resolve_import(
        &self,
        module_path: &str,
        from_file: &Path,
        base_dir: &Path,
    ) -> Result<ResolvedPath, CodeDigestError> {
        // Handle standard library imports
        if self.is_external_module(module_path) {
            return Ok(ResolvedPath {
                path: base_dir.join("requirements.txt"), // Point to requirements.txt as indicator
                is_external: true,
                confidence: 1.0,
            });
        }

        // Handle relative imports (., ..)
        if module_path.starts_with('.') {
            let mut level = 0;
            let mut chars = module_path.chars();
            while chars.next() == Some('.') {
                level += 1;
            }

            // Get the rest of the module path after dots
            let rest = &module_path[level..];

            if let Some(parent) = from_file.parent() {
                let mut current = parent;

                // Go up directories based on dot count
                for _ in 1..level {
                    if let Some(p) = current.parent() {
                        current = p;
                    }
                }

                // Resolve the rest of the path
                if !rest.is_empty() {
                    let path = ResolverUtils::module_to_path(rest);
                    let full_path = current.join(path);

                    // Try as a Python file
                    if let Some(resolved) = ResolverUtils::find_with_extensions(&full_path, &["py"])
                    {
                        return Ok(ResolvedPath {
                            path: resolved,
                            is_external: false,
                            confidence: 0.9,
                        });
                    }

                    // Try as a package directory with __init__.py
                    let init_path = full_path.join("__init__.py");
                    if init_path.exists() {
                        return Ok(ResolvedPath {
                            path: init_path,
                            is_external: false,
                            confidence: 0.9,
                        });
                    }
                }
            }
        }

        // Handle absolute imports
        let parts: Vec<&str> = module_path.split('.').collect();

        // Start from base directory or parent of current file
        let search_paths = vec![
            base_dir.to_path_buf(),
            from_file.parent().unwrap_or(base_dir).to_path_buf(),
        ];

        for search_path in &search_paths {
            let mut current_path = search_path.clone();

            // Build path from module parts
            for (i, part) in parts.iter().enumerate() {
                current_path = current_path.join(part);

                // Check if this is the final part
                if i == parts.len() - 1 {
                    // Try as a Python file
                    let py_file = current_path.with_extension("py");
                    if py_file.exists() {
                        return Ok(ResolvedPath {
                            path: py_file,
                            is_external: false,
                            confidence: 0.8,
                        });
                    }

                    // Try as a package directory
                    let init_path = current_path.join("__init__.py");
                    if init_path.exists() {
                        return Ok(ResolvedPath {
                            path: init_path,
                            is_external: false,
                            confidence: 0.8,
                        });
                    }
                }
            }
        }

        // Otherwise, assume it's an external package
        Ok(ResolvedPath {
            path: base_dir.join("requirements.txt"),
            is_external: true,
            confidence: 0.5,
        })
    }

    fn get_file_extensions(&self) -> Vec<&'static str> {
        vec!["py"]
    }

    fn is_external_module(&self, module_path: &str) -> bool {
        // Common standard library modules
        let stdlib_modules = [
            "os",
            "sys",
            "json",
            "math",
            "random",
            "datetime",
            "collections",
            "itertools",
            "functools",
            "re",
            "time",
            "subprocess",
            "pathlib",
            "typing",
            "asyncio",
            "unittest",
            "logging",
            "argparse",
            "urllib",
            "http",
            "email",
            "csv",
            "sqlite3",
            "threading",
            "multiprocessing",
        ];

        let first_part = module_path.split('.').next().unwrap_or("");
        stdlib_modules.contains(&first_part)
    }
}
