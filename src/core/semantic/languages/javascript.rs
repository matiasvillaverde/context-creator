//! Semantic analyzer for JavaScript

use crate::core::semantic::{
    analyzer::{
        AnalysisResult, FunctionCall, Import, LanguageAnalyzer, SemanticContext, SemanticResult,
        TypeReference,
    },
    resolver::{ModuleResolver, ResolvedPath},
};
use crate::utils::error::CodeDigestError;
use std::path::Path;
use tree_sitter::{Node, Parser, TreeCursor};

#[allow(clippy::new_without_default)]
pub struct JavaScriptAnalyzer {}

impl JavaScriptAnalyzer {
    pub fn new() -> Self {
        Self {}
    }
}

impl LanguageAnalyzer for JavaScriptAnalyzer {
    fn language_name(&self) -> &'static str {
        "JavaScript"
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
            .set_language(tree_sitter_javascript::language())
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
        extension == "js" || extension == "jsx"
    }

    fn supported_extensions(&self) -> Vec<&'static str> {
        vec!["js", "jsx"]
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

            // Type reference handling - JSDoc type annotations
            "type_annotation" => {
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

/// Parse an import statement
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
                                    items.push(format!("* as {}", name));
                                }
                            }
                            if !ns_cursor.goto_next_sibling() {
                                break;
                            }
                        }
                    }
                }
                "named_imports" => {
                    // import { a, b, c }
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

/// Parse a type annotation (JSDoc style)
fn parse_type_annotation(node: &Node, source: &str) -> Option<TypeReference> {
    let start = node.start_position();
    let line = start.row + 1;

    // Extract type name from JSDoc annotation
    if let Ok(type_text) = node.utf8_text(source.as_bytes()) {
        // Simple extraction - could be improved
        let type_name = type_text.trim_start_matches('@').trim();

        Some(TypeReference {
            name: type_name.to_string(),
            module: None,
            line,
        })
    } else {
        None
    }
}

pub struct JavaScriptModuleResolver;

impl ModuleResolver for JavaScriptModuleResolver {
    fn resolve_import(
        &self,
        module_path: &str,
        from_file: &Path,
        base_dir: &Path,
    ) -> Result<ResolvedPath, CodeDigestError> {
        // Handle node_modules imports
        if !module_path.starts_with('.') && !module_path.starts_with('/') {
            // Check if it's a built-in Node.js module
            if self.is_external_module(module_path) {
                return Ok(ResolvedPath {
                    path: base_dir.join("package.json"),
                    is_external: true,
                    confidence: 1.0,
                });
            }

            // Try to find in node_modules
            let mut current = from_file.parent();
            while let Some(dir) = current {
                let node_modules = dir.join("node_modules").join(module_path);

                // Check package.json for main entry
                let package_json = node_modules.join("package.json");
                if package_json.exists() {
                    return Ok(ResolvedPath {
                        path: package_json,
                        is_external: true,
                        confidence: 0.9,
                    });
                }

                // Try index.js
                let index_js = node_modules.join("index.js");
                if index_js.exists() {
                    return Ok(ResolvedPath {
                        path: index_js,
                        is_external: true,
                        confidence: 0.8,
                    });
                }

                current = dir.parent();
            }
        }

        // Handle relative imports
        if module_path.starts_with('.') {
            if let Some(parent) = from_file.parent() {
                let path = parent.join(module_path);

                // Try exact path first
                if path.exists() && path.is_file() {
                    return Ok(ResolvedPath {
                        path,
                        is_external: false,
                        confidence: 1.0,
                    });
                }

                // Try with extensions
                for ext in &["js", "jsx", "mjs", "cjs", "json"] {
                    let with_ext = path.with_extension(ext);
                    if with_ext.exists() {
                        return Ok(ResolvedPath {
                            path: with_ext,
                            is_external: false,
                            confidence: 0.9,
                        });
                    }
                }

                // Try as directory with index.js
                if path.is_dir() {
                    for index_name in &["index.js", "index.jsx", "index.mjs"] {
                        let index_path = path.join(index_name);
                        if index_path.exists() {
                            return Ok(ResolvedPath {
                                path: index_path,
                                is_external: false,
                                confidence: 0.8,
                            });
                        }
                    }
                }
            }
        }

        // Otherwise, assume it's an external module
        Ok(ResolvedPath {
            path: base_dir.join("package.json"),
            is_external: true,
            confidence: 0.5,
        })
    }

    fn get_file_extensions(&self) -> Vec<&'static str> {
        vec!["js", "jsx", "mjs", "cjs"]
    }

    fn is_external_module(&self, module_path: &str) -> bool {
        // Common Node.js built-in modules
        let builtin_modules = [
            "fs",
            "path",
            "http",
            "https",
            "crypto",
            "os",
            "util",
            "stream",
            "events",
            "child_process",
            "cluster",
            "net",
            "dgram",
            "dns",
            "readline",
            "repl",
            "vm",
            "assert",
            "buffer",
            "console",
            "process",
            "querystring",
            "string_decoder",
            "timers",
            "tls",
            "tty",
            "url",
            "zlib",
            "async_hooks",
            "http2",
            "perf_hooks",
            "worker_threads",
        ];

        let module_name = module_path.split('/').next().unwrap_or(module_path);
        builtin_modules.contains(&module_name)
    }
}
