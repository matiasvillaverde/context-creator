//! Semantic analyzer for Python

use crate::core::semantic::{
    analyzer::{
        AnalysisResult, FunctionCall, Import, LanguageAnalyzer, SemanticContext, SemanticResult,
        TypeReference,
    },
    path_validator::{validate_import_path, validate_module_name},
    resolver::{ModuleResolver, ResolvedPath, ResolverUtils},
};
use crate::utils::error::ContextCreatorError;
use std::path::Path;
use tree_sitter::{Node, Parser};

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

        // Create a context for tracking during analysis
        let mut context = AnalysisContext {
            source: content,
            result: &mut result,
            in_comprehension: false,
            in_lambda: false,
        };

        // Walk the tree and extract semantic information
        analyze_node(&root_node, &mut context);

        Ok(result)
    }

    fn can_handle_extension(&self, extension: &str) -> bool {
        matches!(extension, "py" | "pyw" | "pyi")
    }

    fn supported_extensions(&self) -> Vec<&'static str> {
        vec!["py", "pyw", "pyi"]
    }
}

/// Context for tracking state during analysis
struct AnalysisContext<'a> {
    source: &'a str,
    result: &'a mut AnalysisResult,
    in_comprehension: bool,
    in_lambda: bool,
}

/// Recursively analyze nodes in the AST
fn analyze_node(node: &Node, ctx: &mut AnalysisContext) {
    match node.kind() {
        // Import statements
        "import_statement" => handle_import_statement(node, ctx),
        "import_from_statement" => handle_import_from_statement(node, ctx),

        // Function calls
        "call" => handle_function_call(node, ctx),

        // Comprehensions - set context flag
        "list_comprehension"
        | "dictionary_comprehension"
        | "set_comprehension"
        | "generator_expression" => {
            let was_in_comprehension = ctx.in_comprehension;
            ctx.in_comprehension = true;
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    analyze_node(&child, ctx);
                }
            }
            ctx.in_comprehension = was_in_comprehension;
        }

        // Lambda expressions
        "lambda" => {
            let was_in_lambda = ctx.in_lambda;
            ctx.in_lambda = true;
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    analyze_node(&child, ctx);
                }
            }
            ctx.in_lambda = was_in_lambda;
        }

        // Type annotations
        "type" | "annotation" => handle_type_annotation(node, ctx),

        // Function definitions - extract parameter and return types
        "function_definition" => handle_function_definition(node, ctx),

        // Class definitions - extract base classes
        "class_definition" => handle_class_definition(node, ctx),

        // Annotated assignments
        "annotated_assignment" => handle_annotated_assignment(node, ctx),

        // Default: recurse into children
        _ => {
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    analyze_node(&child, ctx);
                }
            }
        }
    }
}

/// Handle import statements (e.g., import os, sys)
fn handle_import_statement(node: &Node, ctx: &mut AnalysisContext) {
    let line = node.start_position().row + 1;

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            match child.kind() {
                "dotted_name" => {
                    if let Ok(module_name) = child.utf8_text(ctx.source.as_bytes()) {
                        ctx.result.imports.push(Import {
                            module: module_name.to_string(),
                            items: vec![],
                            is_relative: false,
                            line,
                        });
                    }
                }
                "aliased_import" => {
                    // Handle "import foo as bar"
                    if let Some(name_node) = child.child_by_field_name("name") {
                        if let Ok(module_name) = name_node.utf8_text(ctx.source.as_bytes()) {
                            ctx.result.imports.push(Import {
                                module: module_name.to_string(),
                                items: vec![],
                                is_relative: false,
                                line,
                            });
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

/// Handle from import statements (e.g., from os import path)
fn handle_import_from_statement(node: &Node, ctx: &mut AnalysisContext) {
    let line = node.start_position().row + 1;
    let mut module_name = String::new();
    let mut imported_items = Vec::new();
    let mut is_relative = false;

    // Check for module name
    if let Some(module_node) = node.child_by_field_name("module_name") {
        match module_node.kind() {
            "relative_import" => {
                is_relative = true;
                // Count dots for relative level
                let text = module_node.utf8_text(ctx.source.as_bytes()).unwrap_or("");
                let dot_count = text.chars().take_while(|&c| c == '.').count();

                // Get the module name after dots if any
                if let Some(dotted_name) = module_node.child_by_field_name("module_name") {
                    if let Ok(name) = dotted_name.utf8_text(ctx.source.as_bytes()) {
                        module_name = format!("{}{}", ".".repeat(dot_count), name);
                    }
                } else {
                    module_name = ".".repeat(dot_count);
                }
            }
            "dotted_name" => {
                if let Ok(name) = module_node.utf8_text(ctx.source.as_bytes()) {
                    module_name = name.to_string();
                }
            }
            _ => {}
        }
    }

    // Get imported names
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            match child.kind() {
                "import_from_as_names" => {
                    imported_items.extend(extract_import_names(&child, ctx.source));
                }
                "import_wildcard" => {
                    imported_items.push("*".to_string());
                }
                _ => {}
            }
        }
    }

    // Sometimes the imported name is a direct child
    if imported_items.is_empty() {
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                if child.kind() == "identifier" {
                    if let Ok(name) = child.utf8_text(ctx.source.as_bytes()) {
                        if name != "from" && name != "import" && name != "as" {
                            imported_items.push(name.to_string());
                        }
                    }
                }
            }
        }
    }

    if !module_name.is_empty() || is_relative {
        ctx.result.imports.push(Import {
            module: module_name,
            items: imported_items,
            is_relative,
            line,
        });
    }
}

/// Extract import names from import_from_as_names node
fn extract_import_names(node: &Node, source: &str) -> Vec<String> {
    let mut names = Vec::new();

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            match child.kind() {
                "identifier" => {
                    if let Ok(name) = child.utf8_text(source.as_bytes()) {
                        names.push(name.to_string());
                    }
                }
                "aliased_import" => {
                    // Handle "import foo as bar" - we want the original name
                    if let Some(name_node) = child.child_by_field_name("name") {
                        if let Ok(name) = name_node.utf8_text(source.as_bytes()) {
                            names.push(name.to_string());
                        }
                    }
                }
                _ => {}
            }
        }
    }

    names
}

/// Handle function calls including method calls
fn handle_function_call(node: &Node, ctx: &mut AnalysisContext) {
    let line = node.start_position().row + 1;

    // Get the function/method being called
    if let Some(function_node) = node.child_by_field_name("function") {
        match function_node.kind() {
            "identifier" => {
                // Simple function call
                if let Ok(name) = function_node.utf8_text(ctx.source.as_bytes()) {
                    ctx.result.function_calls.push(FunctionCall {
                        name: name.to_string(),
                        module: None,
                        line,
                    });
                }
            }
            "attribute" => {
                // Method call or module function call
                if let Some((name, module)) = parse_attribute_chain(&function_node, ctx.source) {
                    ctx.result
                        .function_calls
                        .push(FunctionCall { name, module, line });
                }
            }
            "await" => {
                // Async function call
                if let Some(child) = function_node.child(0) {
                    // Recurse to handle the actual function call
                    if child.kind() == "call" {
                        handle_function_call(&child, ctx);
                    }
                }
            }
            _ => {}
        }
    }

    // Also check for calls within the arguments (for nested calls)
    if let Some(args_node) = node.child_by_field_name("arguments") {
        for i in 0..args_node.child_count() {
            if let Some(child) = args_node.child(i) {
                analyze_node(&child, ctx);
            }
        }
    }
}

/// Parse an attribute chain (e.g., obj.method or module.submodule.function)
fn parse_attribute_chain(node: &Node, source: &str) -> Option<(String, Option<String>)> {
    let mut parts = Vec::new();
    collect_attribute_parts(node, source, &mut parts);

    if parts.is_empty() {
        return None;
    }

    // The last part is the function/method name
    let function_name = parts.pop()?;

    // The rest forms the module/object path
    let module = if parts.is_empty() {
        None
    } else {
        Some(parts.join("."))
    };

    Some((function_name, module))
}

/// Recursively collect parts of an attribute expression
fn collect_attribute_parts(node: &Node, source: &str, parts: &mut Vec<String>) {
    match node.kind() {
        "attribute" => {
            // First collect the object part
            if let Some(object) = node.child_by_field_name("object") {
                collect_attribute_parts(&object, source, parts);
            }
            // Then add the attribute
            if let Some(attr) = node.child_by_field_name("attribute") {
                if let Ok(name) = attr.utf8_text(source.as_bytes()) {
                    parts.push(name.to_string());
                }
            }
        }
        "identifier" => {
            if let Ok(name) = node.utf8_text(source.as_bytes()) {
                parts.push(name.to_string());
            }
        }
        "call" => {
            // Handle chained calls like obj.method().another_method()
            if let Some(function) = node.child_by_field_name("function") {
                collect_attribute_parts(&function, source, parts);
            }
        }
        _ => {}
    }
}

/// Handle type annotations
fn handle_type_annotation(node: &Node, ctx: &mut AnalysisContext) {
    let line = node.start_position().row + 1;

    // Skip if this is part of a type definition
    if let Some(parent) = node.parent() {
        if matches!(parent.kind(), "class_definition" | "type_alias_statement") {
            return;
        }
    }

    if let Some(type_name) = extract_type_name(node, ctx.source) {
        // Split qualified types
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

        ctx.result.type_references.push(TypeReference {
            name,
            module,
            line,
            definition_path: None,
            is_external: false,
            external_package: None,
        });
    }
}

/// Extract type name from various type nodes
fn extract_type_name(node: &Node, source: &str) -> Option<String> {
    match node.kind() {
        "type" | "annotation" => {
            // Look for the actual type within
            if let Some(child) = node.child(0) {
                extract_type_name(&child, source)
            } else {
                None
            }
        }
        "identifier" => node
            .utf8_text(source.as_bytes())
            .ok()
            .map(|s| s.to_string()),
        "attribute" => {
            // Handle qualified types like typing.List
            let mut parts = Vec::new();
            collect_attribute_parts(node, source, &mut parts);
            if !parts.is_empty() {
                Some(parts.join("."))
            } else {
                None
            }
        }
        "subscript" => {
            // Handle generic types like List[str]
            if let Some(value) = node.child_by_field_name("value") {
                // Extract base type
                if let Some(base) = extract_type_name(&value, source) {
                    // Also extract subscript types
                    if let Some(subscript) = node.child_by_field_name("subscript") {
                        let mut subscript_types = Vec::new();
                        extract_subscript_types(&subscript, source, &mut subscript_types);
                        // For now, just return the base type
                        // Could enhance to track generic parameters
                        return Some(base);
                    }
                    return Some(base);
                }
            }
            None
        }
        "union_type" | "optional_type" => {
            // Handle Union[A, B] or Optional[A]
            let mut types = Vec::new();
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    if child.kind() != "|" && child.kind() != "," {
                        if let Some(t) = extract_type_name(&child, source) {
                            types.push(t);
                        }
                    }
                }
            }
            // Return the first type for now
            types.into_iter().next()
        }
        "none" => Some("None".to_string()),
        "string" => {
            // Handle string literal types
            node.utf8_text(source.as_bytes())
                .ok()
                .map(|s| s.trim_matches(|c| c == '"' || c == '\'').to_string())
        }
        _ => None,
    }
}

/// Extract types from subscript expressions
fn extract_subscript_types(node: &Node, source: &str, types: &mut Vec<String>) {
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.kind() != "," && child.kind() != "[" && child.kind() != "]" {
                if let Some(type_name) = extract_type_name(&child, source) {
                    types.push(type_name);
                }
            }
        }
    }
}

/// Handle function definitions to extract parameter and return types
fn handle_function_definition(node: &Node, ctx: &mut AnalysisContext) {
    // Check if it's an async function
    let _is_async = node.child(0).map(|n| n.kind() == "async").unwrap_or(false);

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            match child.kind() {
                "parameters" => handle_parameters(&child, ctx),
                "type" => {
                    // Handle return type annotation
                    handle_type_annotation(&child, ctx);
                }
                "block" => {
                    // Analyze the function body
                    analyze_node(&child, ctx);
                }
                _ => {
                    // Recurse for any other nodes to catch all annotations
                    analyze_node(&child, ctx);
                }
            }
        }
    }
}

/// Handle function parameters to extract type annotations
fn handle_parameters(node: &Node, ctx: &mut AnalysisContext) {
    for i in 0..node.child_count() {
        if let Some(param) = node.child(i) {
            match param.kind() {
                "typed_parameter" | "typed_default_parameter" => {
                    if let Some(type_node) = param.child_by_field_name("type") {
                        handle_type_annotation(&type_node, ctx);
                    }
                }
                "list_splat_pattern" | "dictionary_splat_pattern" => {
                    // Handle *args and **kwargs with type annotations
                    if let Some(type_node) = param.child_by_field_name("type") {
                        handle_type_annotation(&type_node, ctx);
                    }
                }
                _ => {}
            }
        }
    }
}

/// Handle class definitions to extract base classes and type annotations
fn handle_class_definition(node: &Node, ctx: &mut AnalysisContext) {
    // Extract base classes
    if let Some(superclasses) = node.child_by_field_name("superclasses") {
        for i in 0..superclasses.child_count() {
            if let Some(base) = superclasses.child(i) {
                if let Some(type_name) = extract_type_name(&base, ctx.source) {
                    let line = base.start_position().row + 1;
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
                    ctx.result.type_references.push(TypeReference {
                        name,
                        module,
                        line,
                        definition_path: None,
                        is_external: false,
                        external_package: None,
                    });
                }
            }
        }
    }

    // Analyze class body
    if let Some(body) = node.child_by_field_name("body") {
        analyze_node(&body, ctx);
    }
}

/// Handle annotated assignments (e.g., x: int = 5)
fn handle_annotated_assignment(node: &Node, ctx: &mut AnalysisContext) {
    if let Some(annotation) = node.child_by_field_name("annotation") {
        handle_type_annotation(&annotation, ctx);
    }

    // Also analyze the value for any function calls
    if let Some(value) = node.child_by_field_name("value") {
        analyze_node(&value, ctx);
    }
}

pub struct PythonModuleResolver;

impl ModuleResolver for PythonModuleResolver {
    fn resolve_import(
        &self,
        module_path: &str,
        from_file: &Path,
        base_dir: &Path,
    ) -> Result<ResolvedPath, ContextCreatorError> {
        // Validate module name for security
        validate_module_name(module_path)?;

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
                        let validated_path = validate_import_path(base_dir, &resolved)?;
                        return Ok(ResolvedPath {
                            path: validated_path,
                            is_external: false,
                            confidence: 0.9,
                        });
                    }

                    // Try as a package directory with __init__.py
                    let init_path = full_path.join("__init__.py");
                    if init_path.exists() {
                        let validated_path = validate_import_path(base_dir, &init_path)?;
                        return Ok(ResolvedPath {
                            path: validated_path,
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
                        let validated_path = validate_import_path(base_dir, &py_file)?;
                        return Ok(ResolvedPath {
                            path: validated_path,
                            is_external: false,
                            confidence: 0.8,
                        });
                    }

                    // Try as a package directory
                    let init_path = current_path.join("__init__.py");
                    if init_path.exists() {
                        let validated_path = validate_import_path(base_dir, &init_path)?;
                        return Ok(ResolvedPath {
                            path: validated_path,
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
        vec!["py", "pyw", "pyi"]
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
            "abc",
            "enum",
            "dataclasses",
            "contextlib",
            "io",
            "pickle",
            "copy",
            "hashlib",
            "base64",
            "secrets",
            "uuid",
            "platform",
            "socket",
            "ssl",
            "select",
            "queue",
            "struct",
            "array",
            "bisect",
            "heapq",
            "weakref",
            "types",
            "importlib",
            "pkgutil",
            "inspect",
            "ast",
            "dis",
            "traceback",
            "linecache",
            "tokenize",
            "keyword",
            "builtins",
            "__future__",
            "gc",
            "signal",
            "atexit",
            "concurrent",
            "xml",
            "html",
            "urllib",
            "http",
            "ftplib",
            "poplib",
            "imaplib",
            "smtplib",
            "telnetlib",
            "uuid",
            "socketserver",
            "xmlrpc",
            "ipaddress",
            "shutil",
            "tempfile",
            "glob",
            "fnmatch",
            "stat",
            "filecmp",
            "zipfile",
            "tarfile",
            "gzip",
            "bz2",
            "lzma",
            "zlib",
            "configparser",
            "netrc",
            "plistlib",
            "statistics",
            "decimal",
            "fractions",
            "numbers",
            "cmath",
            "operator",
            "difflib",
            "textwrap",
            "unicodedata",
            "stringprep",
            "codecs",
            "encodings",
            "locale",
            "gettext",
            "warnings",
            "pprint",
            "reprlib",
            "graphlib",
        ];

        // Also check common third-party packages that might be imported
        let third_party = [
            "numpy",
            "pandas",
            "requests",
            "flask",
            "django",
            "pytest",
            "matplotlib",
            "scipy",
            "sklearn",
            "tensorflow",
            "torch",
            "beautifulsoup4",
            "selenium",
            "pygame",
            "pillow",
            "sqlalchemy",
            "celery",
            "redis",
            "pymongo",
            "aiohttp",
            "fastapi",
            "pydantic",
            "click",
            "tqdm",
            "colorama",
            "setuptools",
            "pip",
            "wheel",
        ];

        let first_part = module_path.split('.').next().unwrap_or("");
        stdlib_modules.contains(&first_part) || third_party.contains(&first_part)
    }
}
