//! Tree-sitter query engine for efficient semantic analysis
//!
//! This module provides a declarative query-based approach to semantic analysis
//! using Tree-sitter's query engine, replacing manual AST traversal.

use crate::core::semantic::analyzer::{
    AnalysisResult, FunctionCall, FunctionDefinition, Import, TypeReference,
};
use crate::utils::error::ContextCreatorError;
use std::collections::HashMap;
use tree_sitter::{Language, Parser, Query, QueryCursor, Tree};

/// Query engine for semantic analysis using Tree-sitter queries
pub struct QueryEngine {
    #[allow(dead_code)]
    language: Language,
    #[allow(dead_code)]
    language_name: String,
    import_query: Query,
    function_call_query: Query,
    type_reference_query: Query,
    function_definition_query: Query,
}

impl QueryEngine {
    /// Create a new query engine for the specified language
    pub fn new(language: Language, language_name: &str) -> Result<Self, ContextCreatorError> {
        let import_query = Self::create_import_query(language, language_name)?;
        let function_call_query = Self::create_function_call_query(language, language_name)?;
        let type_reference_query = Self::create_type_reference_query(language, language_name)?;
        let function_definition_query =
            Self::create_function_definition_query(language, language_name)?;

        Ok(Self {
            language,
            language_name: language_name.to_string(),
            import_query,
            function_call_query,
            type_reference_query,
            function_definition_query,
        })
    }

    /// Analyze content using Tree-sitter queries
    pub fn analyze_with_parser(
        &self,
        parser: &mut Parser,
        content: &str,
    ) -> Result<AnalysisResult, ContextCreatorError> {
        // Parse the content
        let tree = parser.parse(content, None).ok_or_else(|| {
            ContextCreatorError::ParseError("Failed to parse content".to_string())
        })?;

        self.analyze_tree(&tree, content)
    }

    /// Analyze a parsed tree using queries
    pub fn analyze_tree(
        &self,
        tree: &Tree,
        content: &str,
    ) -> Result<AnalysisResult, ContextCreatorError> {
        let mut result = AnalysisResult::default();
        let mut query_cursor = QueryCursor::new();
        let root_node = tree.root_node();

        // Execute import query
        let import_matches =
            query_cursor.matches(&self.import_query, root_node, content.as_bytes());
        result.imports = self.extract_imports(import_matches, content)?;

        // Execute function call query
        let call_matches =
            query_cursor.matches(&self.function_call_query, root_node, content.as_bytes());
        result.function_calls = self.extract_function_calls(call_matches, content)?;

        // Execute type reference query
        let type_matches =
            query_cursor.matches(&self.type_reference_query, root_node, content.as_bytes());
        result.type_references = self.extract_type_references(type_matches, content)?;

        // Execute function definition query
        let definition_matches = query_cursor.matches(
            &self.function_definition_query,
            root_node,
            content.as_bytes(),
        );
        result.exported_functions =
            self.extract_function_definitions(definition_matches, content)?;

        Ok(result)
    }

    /// Create import query for the specified language
    fn create_import_query(
        language: Language,
        language_name: &str,
    ) -> Result<Query, ContextCreatorError> {
        let query_text = match language_name {
            "rust" => {
                r#"
                ; Use declarations
                (use_declaration) @rust_import

                ; Module declarations  
                (mod_item
                  name: (identifier) @mod_name
                ) @rust_module

                ; Extern crate declarations
                (extern_crate_declaration
                  name: (identifier) @crate_name
                ) @extern_crate
            "#
            }
            "python" => {
                r#"
                ; Simple import statements (import os, import sys)
                (import_statement
                  (dotted_name) @module_name
                ) @simple_import

                ; From import statements with absolute modules (from pathlib import Path)
                (import_from_statement
                  module_name: (dotted_name) @from_module
                  (dotted_name) @import_item
                ) @from_import
                
                ; From import with aliased imports  
                (import_from_statement
                  module_name: (dotted_name) @from_module
                  (aliased_import
                    name: (dotted_name) @import_item
                  )
                ) @from_import_aliased

                ; Wildcard imports (from module import *)
                (import_from_statement
                  module_name: (dotted_name) @from_module
                  (wildcard_import) @wildcard
                ) @wildcard_import

                ; Relative wildcard imports (from . import *, from ..utils import *)
                (import_from_statement
                  module_name: (relative_import) @relative_module
                  (wildcard_import) @wildcard
                ) @relative_wildcard_import

                ; Relative from imports (from . import utils, from ..lib import helper)
                (import_from_statement
                  module_name: (relative_import) @relative_module
                  (dotted_name) @import_item
                ) @relative_from_import

                ; Relative from imports with aliased imports
                (import_from_statement
                  module_name: (relative_import) @relative_module
                  (aliased_import
                    name: (dotted_name) @import_item
                  )
                ) @relative_from_import_aliased
            "#
            }
            "javascript" => {
                r#"
                ; Import declarations
                (import_statement
                  (import_clause
                    [
                      (identifier) @import_name
                      (namespace_import (identifier) @import_name)
                      (named_imports
                        (import_specifier
                          [
                            (identifier) @import_name
                            name: (identifier) @import_name
                          ]
                        )
                      )
                    ]
                  )?
                  source: (string) @module_path
                ) @js_import

                ; Require calls (CommonJS)
                (call_expression
                  function: (identifier) @require_fn (#eq? @require_fn "require")
                  arguments: (arguments (string) @module_path)
                ) @require
            "#
            }
            "typescript" => {
                r#"
                ; Import declarations
                (import_statement
                  (import_clause
                    [
                      (identifier) @import_name
                      (namespace_import (identifier) @import_name)
                      (named_imports
                        (import_specifier
                          [
                            (identifier) @import_name
                            name: (identifier) @import_name
                          ]
                        )
                      )
                    ]
                  )?
                  source: (string) @module_path
                ) @ts_import

                ; Require calls (CommonJS)
                (call_expression
                  function: (identifier) @require_fn (#eq? @require_fn "require")
                  arguments: (arguments (string) @module_path)
                ) @require
            "#
            }
            _ => {
                return Err(ContextCreatorError::ParseError(format!(
                    "Unsupported language for import queries: {language_name}"
                )))
            }
        };

        Query::new(language, query_text).map_err(|e| {
            ContextCreatorError::ParseError(format!("Failed to create import query: {e}"))
        })
    }

    /// Create function call query for the specified language
    fn create_function_call_query(
        language: Language,
        language_name: &str,
    ) -> Result<Query, ContextCreatorError> {
        let query_text = match language_name {
            "rust" => {
                r#"
                ; Simple function calls (helper)
                (call_expression
                  function: (identifier) @fn_name
                ) @call

                ; Scoped function calls (lib::greet)
                (call_expression
                  function: (scoped_identifier
                    path: (identifier) @module_name
                    name: (identifier) @fn_name
                  )
                ) @scoped_call

                ; Nested scoped function calls (lib::User::new)
                (call_expression
                  function: (scoped_identifier
                    path: (scoped_identifier
                      path: (identifier) @module_name
                      name: (identifier) @type_name
                    )
                    name: (identifier) @fn_name
                  )
                ) @nested_scoped_call

                ; Method calls (obj.method())
                (call_expression
                  function: (field_expression
                    field: (field_identifier) @method_name
                  )
                ) @method_call

                ; Macro calls (println!)
                (macro_invocation
                  macro: (identifier) @macro_name
                ) @macro_call
            "#
            }
            "python" => {
                r#"
                ; Simple function calls (print, len)
                (call
                  function: (identifier) @fn_name
                ) @call

                ; Module attribute calls (os.path, module.func)
                (call
                  function: (attribute
                    object: (identifier) @module_name
                    attribute: (identifier) @fn_name
                  )
                ) @module_call

                ; Nested attribute calls (os.path.join)
                (call
                  function: (attribute
                    attribute: (identifier) @fn_name
                  )
                ) @nested_call
            "#
            }
            "javascript" => {
                r#"
                ; Function calls
                (call_expression
                  function: [
                    (identifier) @fn_name
                    (member_expression
                      object: (identifier) @module_name
                      property: (property_identifier) @fn_name
                    )
                  ]
                ) @call
            "#
            }
            "typescript" => {
                r#"
                ; Function calls
                (call_expression
                  function: [
                    (identifier) @fn_name
                    (member_expression
                      object: (identifier) @module_name
                      property: (property_identifier) @fn_name
                    )
                  ]
                ) @call
            "#
            }
            _ => {
                return Err(ContextCreatorError::ParseError(format!(
                    "Unsupported language for function call queries: {language_name}"
                )))
            }
        };

        Query::new(language, query_text).map_err(|e| {
            ContextCreatorError::ParseError(format!("Failed to create function call query: {e}"))
        })
    }

    /// Create function definition query for the specified language
    fn create_function_definition_query(
        language: Language,
        language_name: &str,
    ) -> Result<Query, ContextCreatorError> {
        let query_text = match language_name {
            "rust" => {
                r#"
                ; Function declarations with visibility
                (function_item
                  (visibility_modifier)? @visibility
                  name: (identifier) @fn_name
                ) @function
                
                ; Method declarations in impl blocks
                (impl_item
                  body: (declaration_list
                    (function_item
                      (visibility_modifier)? @method_visibility
                      name: (identifier) @method_name
                    ) @method
                  )
                )
                
                ; Trait method declarations
                (trait_item
                  body: (declaration_list
                    (function_signature_item
                      name: (identifier) @trait_fn_name
                    ) @trait_function
                  )
                )
            "#
            }
            "python" => {
                r#"
                ; Function definitions
                (function_definition
                  name: (identifier) @fn_name
                ) @function
                
                ; Method definitions in classes
                (class_definition
                  body: (block
                    (function_definition
                      name: (identifier) @method_name
                    ) @method
                  )
                )
                
                ; Async function definitions
                (function_definition
                  "async" @async_marker
                  name: (identifier) @async_fn_name
                ) @async_function
            "#
            }
            "javascript" => {
                r#"
                ; Function declarations
                (function_declaration
                  name: (identifier) @fn_name
                ) @function
                
                ; Arrow function assigned to const/let/var
                (variable_declarator
                  name: (identifier) @arrow_fn_name
                  value: (arrow_function)
                ) @arrow_function
                
                ; Function expressions assigned to const/let/var
                (variable_declarator
                  name: (identifier) @fn_expr_name
                  value: (function_expression)
                ) @function_expression
                
                ; Method definitions in objects
                (method_definition
                  name: (property_identifier) @method_name
                ) @method
                
                ; Export function declarations
                (export_statement
                  declaration: (function_declaration
                    name: (identifier) @export_fn_name
                  )
                ) @export_function
                
                ; CommonJS exports pattern: exports.functionName = function()
                (assignment_expression
                  left: (member_expression
                    object: (identifier) @exports_obj (#eq? @exports_obj "exports")
                    property: (property_identifier) @commonjs_export_name
                  )
                  right: [
                    (function_expression)
                    (arrow_function)
                  ]
                ) @commonjs_export
            "#
            }
            "typescript" => {
                r#"
                ; Function declarations
                (function_declaration
                  name: (identifier) @fn_name
                ) @function
                
                ; Arrow function assigned to const/let/var
                (variable_declarator
                  name: (identifier) @arrow_fn_name
                  value: (arrow_function)
                ) @arrow_function
                
                ; Function expressions assigned to const/let/var
                (variable_declarator
                  name: (identifier) @fn_expr_name
                  value: (function_expression)
                ) @function_expression
                
                ; Method definitions in classes
                (method_definition
                  name: (property_identifier) @method_name
                ) @method
                
                ; Export function declarations
                (export_statement
                  declaration: (function_declaration
                    name: (identifier) @export_fn_name
                  )
                ) @export_function
            "#
            }
            _ => {
                return Err(ContextCreatorError::ParseError(format!(
                    "Unsupported language for function definition queries: {language_name}"
                )))
            }
        };

        Query::new(language, query_text).map_err(|e| {
            ContextCreatorError::ParseError(format!(
                "Failed to create function definition query: {e}"
            ))
        })
    }

    /// Create type reference query for the specified language
    fn create_type_reference_query(
        language: Language,
        language_name: &str,
    ) -> Result<Query, ContextCreatorError> {
        let query_text = match language_name {
            "rust" => {
                r#"
                ; Type identifiers (excluding definitions)
                (type_identifier) @type_name
                (#not-match? @type_name "^(i8|i16|i32|i64|i128|u8|u16|u32|u64|u128|f32|f64|bool|char|str|String|Vec|Option|Result)$")

                ; Generic types
                (generic_type
                  type: (type_identifier) @type_name
                )

                ; Scoped type identifiers
                (scoped_type_identifier
                  path: (identifier) @module_name
                  name: (type_identifier) @type_name
                )

                ; Types in function parameters
                (parameter
                  type: [
                    (type_identifier) @param_type
                    (generic_type type: (type_identifier) @param_type)
                    (reference_type type: (type_identifier) @param_type)
                  ]
                )

                ; Return types
                (function_item
                  return_type: [
                    (type_identifier) @return_type
                    (generic_type type: (type_identifier) @return_type)
                    (reference_type type: (type_identifier) @return_type)
                  ]
                )

                ; Field types in structs
                (field_declaration
                  type: [
                    (type_identifier) @field_type
                    (generic_type type: (type_identifier) @field_type)
                    (reference_type type: (type_identifier) @field_type)
                  ]
                )

                ; Trait bounds
                (trait_bounds
                  (type_identifier) @trait_name
                )

                ; Types in use statements (traits and types)
                (use_declaration
                  (scoped_identifier
                    name: (identifier) @imported_type
                  )
                )
                (#match? @imported_type "^[A-Z]")
            "#
            }
            "python" => {
                r#"
                ; Type identifiers in type positions
                (type (identifier) @type_name)

                ; Function parameter type annotations 
                (typed_parameter (identifier) @param_type)

                ; Class inheritance 
                (class_definition
                  superclasses: (argument_list (identifier) @parent_class)
                )

                ; Generic/subscript type references
                (subscript (identifier) @subscript_type)
            "#
            }
            "javascript" => {
                r#"
                ; JSX element types (React components)
                (jsx_element
                  open_tag: (jsx_opening_element
                    name: (identifier) @jsx_type
                  )
                )
                (#match? @jsx_type "^[A-Z]")

                ; JSX self-closing elements
                (jsx_self_closing_element
                  name: (identifier) @jsx_type
                )
                (#match? @jsx_type "^[A-Z]")
            "#
            }
            "typescript" => {
                r#"
                ; Type annotations
                (type_annotation
                  (type_identifier) @type_name
                )

                ; Predefined type annotations (void, any, etc.)
                (type_annotation
                  (predefined_type) @type_name
                )

                ; Generic type arguments
                (type_arguments
                  (type_identifier) @type_arg
                )

                ; Interface declarations
                (interface_declaration
                  name: (type_identifier) @interface_name
                )

                ; Type aliases
                (type_alias_declaration
                  name: (type_identifier) @type_alias
                )
            "#
            }
            _ => {
                return Err(ContextCreatorError::ParseError(format!(
                    "Unsupported language for type queries: {language_name}"
                )))
            }
        };

        Query::new(language, query_text).map_err(|e| {
            ContextCreatorError::ParseError(format!("Failed to create type reference query: {e}"))
        })
    }

    /// Extract imports from query matches
    fn extract_imports<'a>(
        &self,
        matches: tree_sitter::QueryMatches<'a, 'a, &'a [u8]>,
        content: &str,
    ) -> Result<Vec<Import>, ContextCreatorError> {
        let mut imports = Vec::new();
        let import_query_captures = self.import_query.capture_names();

        for match_ in matches {
            let mut module = String::new();
            let mut items = Vec::new();
            let mut is_relative = false;
            let mut line = 0;

            for capture in match_.captures {
                let capture_name = &import_query_captures[capture.index as usize];
                let node = capture.node;
                line = node.start_position().row + 1;

                match capture_name.as_str() {
                    "rust_import" => {
                        // Parse Rust use declaration
                        let (parsed_module, parsed_items, is_rel) =
                            self.parse_rust_use_declaration(node, content);
                        module = parsed_module;
                        items = parsed_items;
                        is_relative = is_rel;
                    }
                    "js_import" | "ts_import" => {
                        // For JavaScript/TypeScript, we rely on module_path and import_name captures
                        // The module and items will be set by those specific captures
                    }
                    "simple_import" => {
                        // Python simple import statement
                    }
                    "from_import" | "from_import_aliased" => {
                        // Python from import statement
                    }
                    "wildcard_import" => {
                        // Python wildcard import statement (from module import *)
                        items.push("*".to_string());
                    }
                    "relative_wildcard_import" => {
                        // Python relative wildcard import statement
                        is_relative = true;
                        items.push("*".to_string());
                    }
                    "relative_from_import" | "relative_from_import_aliased" => {
                        // Python relative from import statement
                        is_relative = true;
                    }
                    "rust_module" => {
                        // Parse module declaration (mod item)
                        let (parsed_module, parsed_items, is_rel) =
                            self.parse_rust_module_declaration(node, content);
                        module = parsed_module;
                        items = parsed_items;
                        is_relative = is_rel;
                    }
                    "mod_name" | "crate_name" => {
                        if let Ok(name) = node.utf8_text(content.as_bytes()) {
                            // Only set module if it's not already set by the full module parsing
                            if module.is_empty() {
                                module = name.to_string();
                                is_relative = capture_name == "mod_name";
                            }
                        }
                    }
                    "module_name" => {
                        // For Python simple imports and Rust/JS module paths
                        if let Ok(name) = node.utf8_text(content.as_bytes()) {
                            module = name.trim_matches('"').to_string();
                        }
                    }
                    "from_module" => {
                        // For Python from imports
                        if let Ok(name) = node.utf8_text(content.as_bytes()) {
                            module = name.to_string();
                        }
                    }
                    "relative_module" => {
                        // For Python relative imports (. or ..lib)
                        if let Ok(name) = node.utf8_text(content.as_bytes()) {
                            module = name.to_string();
                            is_relative = true;
                        }
                    }
                    "import_name" | "import_item" => {
                        if let Ok(name) = node.utf8_text(content.as_bytes()) {
                            items.push(name.to_string());
                        }
                    }
                    "wildcard" => {
                        // Wildcard import (*)
                        items.push("*".to_string());
                    }
                    "module_path" => {
                        if let Ok(name) = node.utf8_text(content.as_bytes()) {
                            module = name.trim_matches('"').trim_matches('\'').to_string();
                            // Check if it's a relative import for JavaScript/TypeScript
                            if module.starts_with('.') {
                                is_relative = true;
                            }
                        }
                    }
                    _ => {}
                }
            }

            if !module.is_empty() || !items.is_empty() {
                // Security check: validate the module path before adding
                if self.is_secure_import(&module) {
                    imports.push(Import {
                        module,
                        items,
                        is_relative,
                        line,
                    });
                } else {
                    // Log dangerous imports but don't include them
                    eprintln!("Warning: Blocked potentially dangerous import: {module}");
                }
            }
        }

        Ok(imports)
    }

    /// Extract function calls from query matches
    fn extract_function_calls<'a>(
        &self,
        matches: tree_sitter::QueryMatches<'a, 'a, &'a [u8]>,
        content: &str,
    ) -> Result<Vec<FunctionCall>, ContextCreatorError> {
        let mut calls = Vec::new();
        let call_query_captures = self.function_call_query.capture_names();

        for match_ in matches {
            let mut name = String::new();
            let mut module = None;
            let mut line = 0;
            let mut module_name = String::new();
            let mut type_name = String::new();

            for capture in match_.captures {
                let capture_name = &call_query_captures[capture.index as usize];
                let node = capture.node;
                line = node.start_position().row + 1;

                match capture_name.as_str() {
                    "fn_name" | "method_name" => {
                        if let Ok(fn_name) = node.utf8_text(content.as_bytes()) {
                            name = fn_name.to_string();
                        }
                    }
                    "module_name" => {
                        if let Ok(mod_name) = node.utf8_text(content.as_bytes()) {
                            module_name = mod_name.to_string();
                            module = Some(mod_name.to_string());
                        }
                    }
                    "type_name" => {
                        if let Ok(type_name_str) = node.utf8_text(content.as_bytes()) {
                            type_name = type_name_str.to_string();
                        }
                    }
                    "macro_name" => {
                        if let Ok(macro_name) = node.utf8_text(content.as_bytes()) {
                            name = macro_name.to_string();
                        }
                    }
                    _ => {}
                }
            }

            // Handle nested scoped calls (lib::User::new)
            if !module_name.is_empty() && !type_name.is_empty() {
                module = Some(format!("{module_name}::{type_name}"));
            }

            if !name.is_empty() {
                calls.push(FunctionCall { name, module, line });
            }
        }

        Ok(calls)
    }

    /// Extract type references from query matches
    fn extract_type_references<'a>(
        &self,
        matches: tree_sitter::QueryMatches<'a, 'a, &'a [u8]>,
        content: &str,
    ) -> Result<Vec<TypeReference>, ContextCreatorError> {
        let mut type_refs = Vec::new();
        let type_query_captures = self.type_reference_query.capture_names();

        for match_ in matches {
            let mut names = HashMap::new();
            let mut module = None;
            let mut line = 0;

            for capture in match_.captures {
                let capture_name = &type_query_captures[capture.index as usize];
                let node = capture.node;
                line = node.start_position().row + 1;

                if let Ok(text) = node.utf8_text(content.as_bytes()) {
                    match capture_name.as_str() {
                        "type_name" | "param_type" | "return_type" | "field_type"
                        | "trait_name" | "imported_type" | "interface_name" | "type_alias"
                        | "jsx_type" | "parent_class" | "type_arg" | "base_type"
                        | "subscript_type" => {
                            names.insert(capture_name.to_string(), text.to_string());
                        }
                        "module_name" => {
                            module = Some(text.to_string());
                        }
                        _ => {}
                    }
                }
            }

            // Create type references for each captured type name
            for (_, type_name) in names {
                // Skip built-in types and primitives
                if self.is_builtin_type(&type_name) {
                    continue;
                }

                type_refs.push(TypeReference {
                    name: type_name.clone(),
                    module: module.clone(),
                    line,
                    definition_path: None,
                    is_external: false,
                    external_package: None,
                });
            }
        }

        Ok(type_refs)
    }

    /// Resolve type definitions for type references
    /// This method attempts to find the file that defines each type
    pub fn resolve_type_definitions(
        &self,
        type_refs: &mut [TypeReference],
        current_file: &std::path::Path,
        project_root: &std::path::Path,
    ) -> Result<(), ContextCreatorError> {
        use crate::core::semantic::path_validator::validate_import_path;

        for type_ref in type_refs.iter_mut() {
            // Skip if already resolved or is external
            if type_ref.definition_path.is_some() || type_ref.is_external {
                continue;
            }

            // Try to resolve the type definition
            if let Some(def_path) = self.find_type_definition(
                &type_ref.name,
                type_ref.module.as_deref(),
                current_file,
                project_root,
            )? {
                // Validate the path for security
                match validate_import_path(project_root, &def_path) {
                    Ok(validated_path) => {
                        type_ref.definition_path = Some(validated_path);
                    }
                    Err(_) => {
                        // Path validation failed, mark as external for safety
                        type_ref.is_external = true;
                    }
                }
            }
        }

        Ok(())
    }

    /// Find the definition file for a given type
    fn find_type_definition(
        &self,
        type_name: &str,
        module_name: Option<&str>,
        current_file: &std::path::Path,
        project_root: &std::path::Path,
    ) -> Result<Option<std::path::PathBuf>, ContextCreatorError> {
        use std::fs;

        // Get the directory of the current file
        let current_dir = current_file.parent().unwrap_or(project_root);

        // Convert type name to lowercase for file matching
        let type_name_lower = type_name.to_lowercase();

        // Get file extensions based on current file
        let extensions = self.get_search_extensions(current_file);

        // Build search patterns
        let mut patterns = vec![
            // Direct file name matches
            format!("{type_name_lower}.{}", extensions[0]),
            // Types files
            format!("types.{}", extensions[0]),
            // Module files
            format!("mod.{}", extensions[0]),
            format!("index.{}", extensions[0]),
            // Common type definition patterns
            format!("{type_name_lower}_types.{}", extensions[0]),
            format!("{type_name_lower}_type.{}", extensions[0]),
            format!("{type_name_lower}s.{}", extensions[0]), // plural form
        ];

        // Add patterns for all supported extensions
        for ext in &extensions[1..] {
            patterns.push(format!("{type_name_lower}.{ext}"));
            patterns.push(format!("types.{ext}"));
            patterns.push(format!("index.{ext}"));
        }

        // If we have a module name, add module-based patterns
        if let Some(module) = module_name {
            // Handle Rust module paths like "crate::models::User"
            if module.starts_with("crate::") {
                let relative_path = module.strip_prefix("crate::").unwrap();
                let path_parts: Vec<&str> = relative_path.split("::").collect();

                if path_parts.len() > 1 {
                    // Convert module path to file path
                    // crate::models::User -> models/user.rs
                    let module_path = path_parts[..path_parts.len() - 1].join("/");
                    let type_name_lower = path_parts.last().unwrap().to_lowercase();

                    for ext in &extensions {
                        patterns.insert(0, format!("{module_path}/{type_name_lower}.{ext}"));
                        patterns.insert(1, format!("{module_path}/mod.{ext}"));
                    }
                }
            } else if module.contains("::") {
                // Handle other module paths like "shared::types::ApiResponse"
                let path_parts: Vec<&str> = module.split("::").collect();

                if path_parts.len() > 1 {
                    // Convert module path to file path
                    // shared::types::ApiResponse -> shared/types/mod.rs
                    let module_path = path_parts[..path_parts.len() - 1].join("/");
                    let type_name_lower = path_parts.last().unwrap().to_lowercase();

                    for ext in &extensions {
                        patterns.insert(0, format!("{module_path}/{type_name_lower}.{ext}"));
                        patterns.insert(1, format!("{module_path}/mod.{ext}"));
                    }
                }
            } else {
                // Handle simple module names
                let module_lower = module.to_lowercase();
                for ext in &extensions {
                    patterns.insert(0, format!("{module_lower}.{ext}"));
                    patterns.insert(1, format!("{module}.{ext}")); // Also try original case
                }
            }
        }

        // Search directories in priority order
        let mut search_dirs = vec![
            project_root.join("src"), // Start with project root src for crate:: paths
            project_root.to_path_buf(),
            current_dir.to_path_buf(),
        ];

        // Add parent directory if it exists
        if let Some(parent_dir) = current_dir.parent() {
            search_dirs.push(parent_dir.to_path_buf());
        }

        // Add common project directories
        search_dirs.extend(vec![
            project_root.join("src/models"),
            project_root.join("src/types"),
            project_root.join("shared"),
            project_root.join("shared/types"),
            project_root.join("lib"),
            project_root.join("domain"),
            current_dir.join("models"),
            current_dir.join("types"),
        ]);

        for search_dir in search_dirs {
            if !search_dir.exists() {
                continue;
            }

            for pattern in &patterns {
                let candidate = search_dir.join(pattern);
                if candidate.exists() {
                    // Read the file to verify it contains the type definition
                    if let Ok(content) = fs::read_to_string(&candidate) {
                        if self.file_contains_definition(&candidate, &content, type_name)? {
                            return Ok(Some(candidate));
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    /// Check if a file contains a definition for a given type name using AST parsing
    fn file_contains_definition(
        &self,
        path: &std::path::Path,
        content: &str,
        type_name: &str,
    ) -> Result<bool, ContextCreatorError> {
        // Determine the language from the file extension
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
                return Ok(false);
            }

            if let Some(tree) = parser.parse(content, None) {
                // Language-specific queries for type definitions
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
                    _ => return Ok(false),
                };

                if let Ok(query) = tree_sitter::Query::new(language, query_text) {
                    let mut cursor = tree_sitter::QueryCursor::new();
                    let matches = cursor.matches(&query, tree.root_node(), content.as_bytes());

                    // Check each match to see if the captured name matches our target type
                    for m in matches {
                        for capture in m.captures {
                            if let Ok(captured_text) = capture.node.utf8_text(content.as_bytes()) {
                                if captured_text == type_name {
                                    return Ok(true);
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(false)
    }

    /// Get appropriate file extensions for searching based on current file
    fn get_search_extensions(&self, current_file: &std::path::Path) -> Vec<&'static str> {
        match current_file.extension().and_then(|s| s.to_str()) {
            Some("rs") => vec!["rs"],
            Some("py") => vec!["py"],
            Some("ts") | Some("tsx") => vec!["ts", "tsx", "js", "jsx"],
            Some("js") | Some("jsx") => vec!["js", "jsx", "ts", "tsx"],
            _ => vec!["rs", "py", "ts", "js"], // Default fallback
        }
    }

    /// Parse Rust use tree structure
    #[allow(dead_code)]
    fn parse_rust_use_tree(
        &self,
        node: tree_sitter::Node,
        content: &str,
    ) -> (String, Vec<String>, bool) {
        // Implementation would recursively parse the use tree structure
        // For now, simplified implementation
        if let Ok(text) = node.utf8_text(content.as_bytes()) {
            let is_relative =
                text.contains("self::") || text.contains("super::") || text.contains("crate::");
            (text.to_string(), Vec::new(), is_relative)
        } else {
            (String::new(), Vec::new(), false)
        }
    }

    /// Parse Rust module declaration structure
    fn parse_rust_module_declaration(
        &self,
        node: tree_sitter::Node,
        content: &str,
    ) -> (String, Vec<String>, bool) {
        // Parse module declaration like "mod config;"
        if let Ok(text) = node.utf8_text(content.as_bytes()) {
            // Look for the module name after "mod"
            if let Some(mod_start) = text.find("mod ") {
                let after_mod = &text[mod_start + 4..];
                if let Some(end_pos) = after_mod.find(';') {
                    let module_name = after_mod[..end_pos].trim();
                    return (module_name.to_string(), Vec::new(), true);
                } else if let Some(end_pos) = after_mod.find(' ') {
                    let module_name = after_mod[..end_pos].trim();
                    return (module_name.to_string(), Vec::new(), true);
                }
            }
        }
        (String::new(), Vec::new(), false)
    }

    /// Parse Rust use declaration structure
    fn parse_rust_use_declaration(
        &self,
        node: tree_sitter::Node,
        content: &str,
    ) -> (String, Vec<String>, bool) {
        // Parse the entire use declaration
        if let Ok(text) = node.utf8_text(content.as_bytes()) {
            // Extract module path and imported items from use declaration
            // Example: "use model::{Account, DatabaseFactory, Rule};"
            let clean_text = text
                .trim()
                .trim_start_matches("use ")
                .trim_end_matches(';')
                .trim();

            let is_relative = clean_text.contains("self::")
                || clean_text.contains("super::")
                || clean_text.contains("crate::");

            if clean_text.contains('{') && clean_text.contains('}') {
                // Handle scoped imports like "model::{Account, DatabaseFactory}"
                if let Some(colon_pos) = clean_text.find("::") {
                    let module = clean_text[..colon_pos].to_string();

                    // Extract items from braces
                    if let Some(start) = clean_text.find('{') {
                        if let Some(end) = clean_text.find('}') {
                            let items_str = &clean_text[start + 1..end];
                            let items: Vec<String> = items_str
                                .split(',')
                                .map(|s| s.trim().to_string())
                                .filter(|s| !s.is_empty())
                                .collect();
                            return (module, items, is_relative);
                        }
                    }
                }
            } else {
                // Handle simple imports like "use std::collections::HashMap;"
                return (clean_text.to_string(), Vec::new(), is_relative);
            }

            (clean_text.to_string(), Vec::new(), is_relative)
        } else {
            (String::new(), Vec::new(), false)
        }
    }

    /// Check if an import is secure (doesn't attempt path traversal or system access)
    fn is_secure_import(&self, module: &str) -> bool {
        // Reject empty modules
        if module.is_empty() {
            return false;
        }

        // Check for absolute paths that could be system paths
        if module.starts_with('/') {
            // Unix absolute paths like /etc/passwd
            if module.contains("/etc/") || module.contains("/sys/") || module.contains("/proc/") {
                return false;
            }
        }

        // Check for Windows absolute paths
        if module.len() >= 2 && module.chars().nth(1) == Some(':') {
            // Windows paths like C:\Windows\System32
            if module.to_lowercase().contains("windows")
                || module.to_lowercase().contains("system32")
            {
                return false;
            }
        }

        // Check for excessive path traversal
        let dot_dot_count = module.matches("..").count();
        if dot_dot_count > 3 {
            // More than 3 levels of .. is suspicious
            return false;
        }

        // Check for known dangerous patterns
        let dangerous_patterns = [
            "/etc/passwd",
            "/etc/shadow",
            "/root/",
            "C:\\Windows\\",
            "C:\\System32\\",
            "../../../../etc/",
            "..\\..\\..\\..\\windows\\",
            "file:///",
            "~/../../../",
            "%USERPROFILE%",
            "$HOME/../../../",
        ];

        for pattern in &dangerous_patterns {
            if module.contains(pattern) {
                return false;
            }
        }

        // Check for suspicious characters that might indicate injection
        if module.contains('\0') || module.contains('\x00') {
            return false;
        }

        // Allow the import if it passes all checks
        true
    }

    /// Extract function definitions from query matches
    fn extract_function_definitions<'a>(
        &self,
        matches: tree_sitter::QueryMatches<'a, 'a, &'a [u8]>,
        content: &str,
    ) -> Result<Vec<FunctionDefinition>, ContextCreatorError> {
        let mut definitions = Vec::new();
        let def_query_captures = self.function_definition_query.capture_names();

        for match_ in matches {
            let mut name = String::new();
            let mut is_exported = false;
            let mut line = 0;

            for capture in match_.captures {
                let capture_name = &def_query_captures[capture.index as usize];
                let node = capture.node;
                line = node.start_position().row + 1;

                match capture_name.as_str() {
                    "fn_name"
                    | "method_name"
                    | "assoc_fn_name"
                    | "arrow_fn_name"
                    | "fn_expr_name"
                    | "async_fn_name"
                    | "export_fn_name"
                    | "trait_fn_name"
                    | "commonjs_export_name" => {
                        if let Ok(fn_name) = node.utf8_text(content.as_bytes()) {
                            name = fn_name.to_string();
                        }
                    }
                    "visibility" | "method_visibility" => {
                        if let Ok(vis) = node.utf8_text(content.as_bytes()) {
                            // In Rust, pub means exported
                            is_exported = vis.contains("pub");
                        }
                    }
                    "export_function" | "commonjs_export" => {
                        // JavaScript/TypeScript export
                        is_exported = true;
                    }
                    "function"
                    | "method"
                    | "assoc_function"
                    | "arrow_function"
                    | "function_expression"
                    | "async_function" => {
                        // For languages without explicit visibility, check context
                        if self.language_name == "python" {
                            // In Python, functions not starting with _ are considered public
                            is_exported = !name.starts_with('_');
                        } else if self.language_name == "javascript"
                            || self.language_name == "typescript"
                        {
                            // In JS/TS, all module-level functions are potentially callable
                            // unless explicitly marked private or are nested
                            is_exported = true;
                        }
                    }
                    _ => {}
                }
            }

            if !name.is_empty() {
                // Special handling for Python methods
                if self.language_name == "python" && !name.starts_with('_') {
                    is_exported = true;
                }

                // Special handling for JavaScript/TypeScript without explicit export
                if (self.language_name == "javascript" || self.language_name == "typescript")
                    && !is_exported
                {
                    // Default to exported for top-level functions
                    is_exported = true;
                }

                definitions.push(FunctionDefinition {
                    name,
                    is_exported,
                    line,
                });
            }
        }

        Ok(definitions)
    }

    /// Check if a type name is a built-in type
    fn is_builtin_type(&self, type_name: &str) -> bool {
        matches!(
            type_name,
            "i8" | "i16"
                | "i32"
                | "i64"
                | "i128"
                | "u8"
                | "u16"
                | "u32"
                | "u64"
                | "u128"
                | "f32"
                | "f64"
                | "bool"
                | "char"
                | "str"
                | "String"
                | "Vec"
                | "Option"
                | "Result"
                | "Box"
                | "Rc"
                | "Arc"
                | "HashMap"
                | "HashSet"
                | "number"
                | "string"
                | "boolean"
                | "object"
                | "int"
                | "float"
                | "list"
                | "dict"
                | "tuple"
                | "set"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_query_creation() {
        let engine = QueryEngine::new(tree_sitter_rust::language(), "rust");
        assert!(engine.is_ok());
    }

    #[test]
    fn test_python_query_creation() {
        let engine = QueryEngine::new(tree_sitter_python::language(), "python");
        if let Err(e) = &engine {
            println!("Python QueryEngine error: {e}");
        }
        assert!(engine.is_ok());
    }

    #[test]
    fn test_javascript_query_creation() {
        let engine = QueryEngine::new(tree_sitter_javascript::language(), "javascript");
        if let Err(e) = &engine {
            println!("JavaScript QueryEngine error: {e}");
        }
        assert!(engine.is_ok());
    }

    #[test]
    fn test_typescript_query_creation() {
        let engine = QueryEngine::new(tree_sitter_typescript::language_typescript(), "typescript");
        if let Err(e) = &engine {
            println!("TypeScript QueryEngine error: {e}");
        }
        assert!(engine.is_ok());
    }

    #[test]
    fn test_builtin_type_detection() {
        let engine = QueryEngine::new(tree_sitter_rust::language(), "rust").unwrap();

        assert!(engine.is_builtin_type("String"));
        assert!(engine.is_builtin_type("Vec"));
        assert!(engine.is_builtin_type("i32"));
        assert!(!engine.is_builtin_type("MyCustomType"));
    }
}
