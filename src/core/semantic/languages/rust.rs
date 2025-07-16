//! Semantic analyzer for Rust

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
pub struct RustAnalyzer {}

impl RustAnalyzer {
    pub fn new() -> Self {
        Self {}
    }
}

impl LanguageAnalyzer for RustAnalyzer {
    fn language_name(&self) -> &'static str {
        "Rust"
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
        parser.set_language(tree_sitter_rust::language()).unwrap();

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
        extension == "rs"
    }

    fn supported_extensions(&self) -> Vec<&'static str> {
        vec!["rs"]
    }
}

/// Extract semantic information from the AST
fn extract_semantic_info(cursor: &mut TreeCursor, source: &str, result: &mut AnalysisResult) {
    loop {
        let node = cursor.node();

        match node.kind() {
            // Import handling
            "use_declaration" => {
                if let Some(import) = parse_use_declaration(&node, source) {
                    result.imports.push(import);
                }
            }
            "mod_item" => {
                if let Some(import) = parse_mod_declaration(&node, source) {
                    result.imports.push(import);
                }
            }
            "extern_crate_declaration" => {
                if let Some(import) = parse_extern_crate(&node, source) {
                    result.imports.push(import);
                }
            }

            // Function call handling
            "call_expression" => {
                if let Some(call) = parse_function_call(&node, source) {
                    result.function_calls.push(call);
                }
            }
            "method_call_expression" => {
                if let Some(call) = parse_method_call(&node, source) {
                    result.function_calls.push(call);
                }
            }

            // Type reference handling
            "type_identifier" => {
                if let Some(type_ref) = parse_type_reference(&node, source) {
                    result.type_references.push(type_ref);
                }
            }
            "generic_type" => {
                if let Some(type_ref) = parse_generic_type(&node, source) {
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

/// Parse a use declaration
fn parse_use_declaration(node: &Node, source: &str) -> Option<Import> {
    let start = node.start_position();
    let line = start.row + 1;

    // Find the path in the use declaration
    let mut path = String::new();
    let mut cursor = node.walk();

    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            match child.kind() {
                "use_tree" => {
                    path = extract_use_path(&child, source);
                    break;
                }
                "scoped_identifier" => {
                    // Direct scoped_identifier (e.g., std::collections::HashMap)
                    path = extract_scoped_identifier(&child, source);
                    break;
                }
                _ => {}
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }

    if path.is_empty() {
        return None;
    }

    let is_relative =
        path.starts_with("super") || path.starts_with("self") || path.starts_with("crate");

    Some(Import {
        module: path,
        items: vec![], // TODO: Extract specific items from use tree
        is_relative,
        line,
    })
}

/// Extract the path from a use tree
fn extract_use_path(node: &Node, source: &str) -> String {
    let mut path_parts = Vec::new();
    let mut cursor = node.walk();

    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            match child.kind() {
                "identifier" | "super" | "self" | "crate" => {
                    if let Ok(text) = child.utf8_text(source.as_bytes()) {
                        path_parts.push(text);
                    }
                }
                "scoped_identifier" => {
                    let scoped_path = extract_scoped_identifier(&child, source);
                    return scoped_path;
                }
                "use_tree" => {
                    // Nested use tree
                    return extract_use_path(&child, source);
                }
                _ => {}
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }

    path_parts.join("::")
}

/// Extract a scoped identifier (e.g., std::collections::HashMap)
fn extract_scoped_identifier(node: &Node, source: &str) -> String {
    let mut parts = Vec::new();
    let mut cursor = node.walk();

    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            match child.kind() {
                "identifier" | "super" | "self" | "crate" => {
                    if let Ok(text) = child.utf8_text(source.as_bytes()) {
                        parts.push(text.to_string());
                    }
                }
                "scoped_identifier" => {
                    // Recursively handle nested scoped identifiers
                    let nested = extract_scoped_identifier(&child, source);
                    if !nested.is_empty() {
                        parts.push(nested);
                    }
                }
                _ => {}
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }

    parts.join("::")
}

/// Parse a mod declaration
fn parse_mod_declaration(node: &Node, source: &str) -> Option<Import> {
    let start = node.start_position();
    let line = start.row + 1;

    // Look for the module name
    let mut cursor = node.walk();
    let mut module_name = String::new();

    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            if child.kind() == "identifier" {
                if let Ok(name) = child.utf8_text(source.as_bytes()) {
                    module_name = name.to_string();
                    break;
                }
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }

    if module_name.is_empty() {
        return None;
    }

    Some(Import {
        module: module_name,
        items: vec![],
        is_relative: true, // Local modules are relative
        line,
    })
}

/// Parse an extern crate declaration
fn parse_extern_crate(node: &Node, source: &str) -> Option<Import> {
    let start = node.start_position();
    let line = start.row + 1;

    // Look for the crate name
    let mut cursor = node.walk();
    let mut crate_name = String::new();

    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            if child.kind() == "identifier" {
                if let Ok(name) = child.utf8_text(source.as_bytes()) {
                    crate_name = name.to_string();
                    break;
                }
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }

    if crate_name.is_empty() {
        return None;
    }

    Some(Import {
        module: crate_name,
        items: vec![],
        is_relative: false,
        line,
    })
}

/// Parse a function call
fn parse_function_call(node: &Node, source: &str) -> Option<FunctionCall> {
    let start = node.start_position();
    let line = start.row + 1;

    // Get the function being called
    let mut cursor = node.walk();
    let mut function_name = String::new();
    let mut module = None;

    if cursor.goto_first_child() {
        let first_child = cursor.node();
        match first_child.kind() {
            "identifier" => {
                if let Ok(name) = first_child.utf8_text(source.as_bytes()) {
                    function_name = name.to_string();
                }
            }
            "scoped_identifier" => {
                let full_path = extract_scoped_identifier(&first_child, source);
                let parts: Vec<&str> = full_path.rsplitn(2, "::").collect();
                if parts.len() == 2 {
                    function_name = parts[0].to_string();
                    module = Some(parts[1].to_string());
                } else {
                    function_name = full_path;
                }
            }
            "field_expression" => {
                // Handle something like obj.method() - this would be handled by method_call_expression
                return None;
            }
            _ => {}
        }
    }

    if function_name.is_empty() {
        return None;
    }

    Some(FunctionCall {
        name: function_name,
        module,
        line,
    })
}

/// Parse a method call
fn parse_method_call(node: &Node, source: &str) -> Option<FunctionCall> {
    let start = node.start_position();
    let line = start.row + 1;

    // Look for the method name
    let mut cursor = node.walk();
    let mut method_name = String::new();

    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            if child.kind() == "field_identifier" {
                if let Ok(name) = child.utf8_text(source.as_bytes()) {
                    method_name = name.to_string();
                    break;
                }
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }

    if method_name.is_empty() {
        return None;
    }

    Some(FunctionCall {
        name: method_name,
        module: None, // Method calls don't have explicit modules
        line,
    })
}

/// Parse a type reference
fn parse_type_reference(node: &Node, source: &str) -> Option<TypeReference> {
    // Skip if this is part of a declaration
    if let Some(parent) = node.parent() {
        match parent.kind() {
            "struct_item" | "enum_item" | "type_item" | "trait_item" => {
                return None; // Skip type definitions
            }
            _ => {}
        }
    }

    let start = node.start_position();
    let line = start.row + 1;

    if let Ok(type_name) = node.utf8_text(source.as_bytes()) {
        Some(TypeReference {
            name: type_name.to_string(),
            module: None, // TODO: Determine module from context
            line,
        })
    } else {
        None
    }
}

/// Parse a generic type
fn parse_generic_type(node: &Node, source: &str) -> Option<TypeReference> {
    let start = node.start_position();
    let line = start.row + 1;

    // Get the base type
    let mut cursor = node.walk();
    let mut base_type = String::new();

    if cursor.goto_first_child() {
        let first_child = cursor.node();
        match first_child.kind() {
            "type_identifier" => {
                if let Ok(name) = first_child.utf8_text(source.as_bytes()) {
                    base_type = name.to_string();
                }
            }
            "scoped_type_identifier" => {
                base_type = extract_scoped_identifier(&first_child, source);
            }
            _ => {}
        }
    }

    if base_type.is_empty() {
        return None;
    }

    Some(TypeReference {
        name: base_type,
        module: None, // TODO: Extract module from scoped identifiers
        line,
    })
}

pub struct RustModuleResolver;

impl ModuleResolver for RustModuleResolver {
    fn resolve_import(
        &self,
        module_path: &str,
        from_file: &Path,
        base_dir: &Path,
    ) -> Result<ResolvedPath, CodeDigestError> {
        // Handle standard library imports
        if self.is_external_module(module_path) {
            return Ok(ResolvedPath {
                path: base_dir.join("Cargo.toml"), // Point to Cargo.toml as indicator
                is_external: true,
                confidence: 1.0,
            });
        }

        // Handle crate-relative imports
        if module_path.starts_with("crate::") {
            let relative_path = module_path.strip_prefix("crate::").unwrap();
            let path = ResolverUtils::module_to_path(relative_path);
            let full_path = base_dir.join("src").join(path);

            if let Some(resolved) = ResolverUtils::find_with_extensions(&full_path, &["rs"]) {
                return Ok(ResolvedPath {
                    path: resolved,
                    is_external: false,
                    confidence: 0.9,
                });
            }

            // Try as a directory module (mod.rs)
            let mod_path = full_path.join("mod.rs");
            if mod_path.exists() {
                return Ok(ResolvedPath {
                    path: mod_path,
                    is_external: false,
                    confidence: 0.9,
                });
            }
        }

        // Handle relative imports (self, super)
        if module_path.starts_with("self::") || module_path.starts_with("super::") {
            if let Some(resolved) = ResolverUtils::resolve_relative(module_path, from_file, &["rs"])
            {
                return Ok(ResolvedPath {
                    path: resolved,
                    is_external: false,
                    confidence: 0.9,
                });
            }
        }

        // Handle simple module names (e.g., "mod lib;" in same directory)
        if !module_path.contains("::") {
            if let Some(parent) = from_file.parent() {
                // Try as a file
                let file_path = parent.join(format!("{}.rs", module_path));
                if file_path.exists() {
                    return Ok(ResolvedPath {
                        path: file_path,
                        is_external: false,
                        confidence: 0.9,
                    });
                }

                // Try as a directory module
                let mod_path = parent.join(module_path).join("mod.rs");
                if mod_path.exists() {
                    return Ok(ResolvedPath {
                        path: mod_path,
                        is_external: false,
                        confidence: 0.9,
                    });
                }
            }
        }

        // Otherwise, assume it's an external crate
        Ok(ResolvedPath {
            path: base_dir.join("Cargo.toml"), // Point to Cargo.toml as indicator
            is_external: true,
            confidence: 0.5,
        })
    }

    fn get_file_extensions(&self) -> Vec<&'static str> {
        vec!["rs"]
    }

    fn is_external_module(&self, module_path: &str) -> bool {
        // Common standard library crates
        let stdlib_crates = ["std", "core", "alloc", "proc_macro", "test"];

        // Get the first part of the path (before ::)
        let first_part = module_path.split("::").next().unwrap_or(module_path);

        // Check if it's a standard library crate
        if stdlib_crates.contains(&first_part) {
            return true;
        }

        // Simple module names (no ::) are NOT external - they're local modules
        if !module_path.contains("::") {
            return false;
        }

        // crate::, self::, super:: are always local
        if module_path.starts_with("crate::")
            || module_path.starts_with("self::")
            || module_path.starts_with("super::")
        {
            return false;
        }

        // Other paths with :: might be external crates
        // For now, we'll consider them external unless we have more context
        true
    }
}
