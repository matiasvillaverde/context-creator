//! Semantic analyzer for TypeScript

use crate::core::semantic::analyzer::{
    AnalysisResult, FunctionCall, Import, LanguageAnalyzer, SemanticContext, SemanticResult,
    TypeReference,
};
use std::path::Path;
use tree_sitter::{Node, Parser, TreeCursor};

#[allow(clippy::new_without_default)]
pub struct TypeScriptAnalyzer {}

impl TypeScriptAnalyzer {
    pub fn new() -> Self {
        Self {}
    }
}

impl LanguageAnalyzer for TypeScriptAnalyzer {
    fn language_name(&self) -> &'static str {
        "TypeScript"
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
        parser
            .set_language(tree_sitter_typescript::language_typescript())
            .unwrap();

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
        extension == "ts" || extension == "tsx"
    }

    fn supported_extensions(&self) -> Vec<&'static str> {
        vec!["ts", "tsx"]
    }
}

/// Extract semantic information from the AST
fn extract_semantic_info(cursor: &mut TreeCursor, source: &str, result: &mut AnalysisResult) {
    loop {
        let node = cursor.node();

        match node.kind() {
            // Import handling
            "import_statement" => {
                if let Some(import) = parse_import_statement(&node, source) {
                    result.imports.push(import);
                }
            }

            // Function call handling
            "call_expression" => {
                if let Some(call) = parse_function_call(&node, source) {
                    result.function_calls.push(call);
                }
            }

            // Type reference handling - TypeScript has explicit type annotations
            "type_identifier" => {
                if let Some(type_ref) = parse_type_identifier(&node, source) {
                    result.type_references.push(type_ref);
                }
            }
            "generic_type" => {
                if let Some(type_ref) = parse_generic_type(&node, source) {
                    result.type_references.push(type_ref);
                }
            }
            "type_annotation" => {
                // Handle type annotations in variable declarations, parameters, etc.
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

/// Parse an import statement (same as JavaScript but can import types)
fn parse_import_statement(node: &Node, source: &str) -> Option<Import> {
    let start = node.start_position();
    let line = start.row + 1;

    let mut module_path = String::new();
    let mut imported_items = Vec::new();

    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            match child.kind() {
                "import_clause" => {
                    // Extract imported items
                    imported_items.extend(extract_import_items(&child, source));
                }
                "string" => {
                    // The module path in quotes
                    if let Ok(path) = child.utf8_text(source.as_bytes()) {
                        // Remove quotes
                        module_path = path
                            .trim_matches(|c| c == '"' || c == '\'' || c == '`')
                            .to_string();
                    }
                }
                _ => {}
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }

    if module_path.is_empty() {
        return None;
    }

    let is_relative = module_path.starts_with('.') || module_path.starts_with('/');

    Some(Import {
        module: module_path,
        items: imported_items,
        is_relative,
        line,
    })
}

/// Extract imported items from import clause
fn extract_import_items(node: &Node, source: &str) -> Vec<String> {
    let mut items = Vec::new();
    let mut cursor = node.walk();

    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            match child.kind() {
                "identifier" => {
                    // Default import
                    if let Ok(name) = child.utf8_text(source.as_bytes()) {
                        items.push(name.to_string());
                    }
                }
                "namespace_import" => {
                    // import * as name
                    let mut ns_cursor = child.walk();
                    if ns_cursor.goto_first_child() {
                        loop {
                            if ns_cursor.node().kind() == "identifier" {
                                if let Ok(name) = ns_cursor.node().utf8_text(source.as_bytes()) {
                                    items.push(format!("* as {name}"));
                                }
                            }
                            if !ns_cursor.goto_next_sibling() {
                                break;
                            }
                        }
                    }
                }
                "named_imports" => {
                    // import { a, b, c } or import type { Type }
                    items.extend(extract_named_imports(&child, source));
                }
                _ => {}
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }

    items
}

/// Extract named imports { a, b as c }
fn extract_named_imports(node: &Node, source: &str) -> Vec<String> {
    let mut items = Vec::new();
    let mut cursor = node.walk();

    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            if child.kind() == "import_specifier" {
                // Could be "name" or "name as alias"
                let mut spec_cursor = child.walk();
                if spec_cursor.goto_first_child() {
                    if let Ok(name) = spec_cursor.node().utf8_text(source.as_bytes()) {
                        items.push(name.to_string());
                    }
                }
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }

    items
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
        "member_expression" => {
            // Handle method calls like obj.method()
            parse_member_expression(&function_node, source)?
        }
        _ => return None,
    };

    Some(FunctionCall {
        name: function_name,
        module,
        line,
    })
}

/// Parse a member expression (e.g., console.log, fs.readFile)
fn parse_member_expression(node: &Node, source: &str) -> Option<(String, Option<String>)> {
    let mut parts = Vec::new();
    collect_member_parts(node, source, &mut parts);

    if parts.is_empty() {
        return None;
    }

    // The last part is the function/method name
    let function_name = parts.pop()?;

    // The rest is the object/module path
    let module = if parts.is_empty() {
        None
    } else {
        Some(parts.join("."))
    };

    Some((function_name, module))
}

/// Recursively collect parts of a member expression
fn collect_member_parts(node: &Node, source: &str, parts: &mut Vec<String>) {
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
                "property_identifier" => {
                    if let Ok(name) = child.utf8_text(source.as_bytes()) {
                        parts.push(name.to_string());
                    }
                }
                "member_expression" => {
                    collect_member_parts(&child, source, parts);
                }
                _ => {}
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
}

/// Parse a type identifier
fn parse_type_identifier(node: &Node, source: &str) -> Option<TypeReference> {
    // Skip if this is part of a type definition
    if let Some(parent) = node.parent() {
        match parent.kind() {
            "interface_declaration" | "type_alias_declaration" | "class_declaration" => {
                return None;
            }
            _ => {}
        }
    }

    let start = node.start_position();
    let line = start.row + 1;

    if let Ok(type_name) = node.utf8_text(source.as_bytes()) {
        Some(TypeReference {
            name: type_name.to_string(),
            module: None,
            line,
            definition_path: None,
            is_external: false,
            external_package: None,
        })
    } else {
        None
    }
}

/// Parse a generic type (e.g., Array<string>, Map<string, number>)
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
            "nested_type_identifier" => {
                // Handle qualified types like React.Component
                base_type = extract_nested_type_identifier(&first_child, source);
            }
            _ => {}
        }
    }

    if base_type.is_empty() {
        return None;
    }

    // Try to extract module from qualified types
    let (name, module) = if base_type.contains('.') {
        let parts: Vec<&str> = base_type.rsplitn(2, '.').collect();
        if parts.len() == 2 {
            (parts[0].to_string(), Some(parts[1].to_string()))
        } else {
            (base_type, None)
        }
    } else {
        (base_type, None)
    };

    Some(TypeReference {
        name,
        module,
        line,
        definition_path: None,
        is_external: false,
        external_package: None,
    })
}

/// Extract nested type identifier (e.g., React.Component)
fn extract_nested_type_identifier(node: &Node, source: &str) -> String {
    let mut parts = Vec::new();
    let mut cursor = node.walk();

    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            if child.kind() == "identifier" || child.kind() == "type_identifier" {
                if let Ok(text) = child.utf8_text(source.as_bytes()) {
                    parts.push(text.to_string());
                }
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }

    parts.join(".")
}

/// Parse a type annotation
fn parse_type_annotation(node: &Node, source: &str) -> Option<TypeReference> {
    // Look for the actual type within the annotation
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            match child.kind() {
                "type_identifier" => {
                    return parse_type_identifier(&child, source);
                }
                "generic_type" => {
                    return parse_generic_type(&child, source);
                }
                "predefined_type" => {
                    // Built-in types like string, number, boolean
                    let start = child.start_position();
                    let line = start.row + 1;

                    if let Ok(type_name) = child.utf8_text(source.as_bytes()) {
                        return Some(TypeReference {
                            name: type_name.to_string(),
                            module: None,
                            line,
                            definition_path: None,
                            is_external: false,
                            external_package: None,
                        });
                    }
                }
                _ => {}
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }

    None
}
