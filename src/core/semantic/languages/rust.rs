//! Semantic analyzer for Rust

use crate::core::semantic::{
    analyzer::{AnalysisResult, LanguageAnalyzer, SemanticContext, SemanticResult},
    path_validator::{validate_import_path, validate_module_name},
    query_engine::QueryEngine,
    resolver::{ModuleResolver, ResolvedPath, ResolverUtils},
};
use crate::utils::error::ContextCreatorError;
use std::path::Path;
use tree_sitter::Parser;

#[allow(clippy::new_without_default)]
pub struct RustAnalyzer {
    query_engine: QueryEngine,
}

impl RustAnalyzer {
    pub fn new() -> Self {
        let language = tree_sitter_rust::language();
        let query_engine =
            QueryEngine::new(language, "rust").expect("Failed to create Rust query engine");
        Self { query_engine }
    }
}

impl LanguageAnalyzer for RustAnalyzer {
    fn language_name(&self) -> &'static str {
        "Rust"
    }

    fn analyze_file(
        &self,
        path: &Path,
        content: &str,
        context: &SemanticContext,
    ) -> SemanticResult<AnalysisResult> {
        let mut parser = Parser::new();
        parser
            .set_language(tree_sitter_rust::language())
            .map_err(|e| ContextCreatorError::ParseError(format!("Failed to set language: {e}")))?;

        let mut result = self
            .query_engine
            .analyze_with_parser(&mut parser, content)?;

        // Correlate type references with imports to populate module information
        self.correlate_types_with_imports(&mut result);

        // Resolve type definitions for the type references found
        self.query_engine.resolve_type_definitions(
            &mut result.type_references,
            path,
            &context.base_dir,
        )?;

        Ok(result)
    }

    fn can_handle_extension(&self, extension: &str) -> bool {
        extension == "rs"
    }

    fn supported_extensions(&self) -> Vec<&'static str> {
        vec!["rs"]
    }
}

impl RustAnalyzer {
    /// Correlate type references with imports to populate module information
    fn correlate_types_with_imports(&self, result: &mut AnalysisResult) {
        use std::collections::HashMap;

        // Create a mapping from imported type names to their module paths
        let mut type_to_module: HashMap<String, String> = HashMap::new();

        for import in &result.imports {
            if import.items.is_empty() {
                // Handle simple imports like "use std::collections::HashMap;"
                if let Some(type_name) = import.module.split("::").last() {
                    // Check if this looks like a type (starts with uppercase)
                    if type_name.chars().next().is_some_and(|c| c.is_uppercase()) {
                        type_to_module.insert(type_name.to_string(), import.module.clone());
                    }
                }
            } else {
                // Handle scoped imports like "use model::{Account, DatabaseFactory};"
                for item in &import.items {
                    // Check if this looks like a type (starts with uppercase)
                    if item.chars().next().is_some_and(|c| c.is_uppercase()) {
                        type_to_module.insert(item.clone(), import.module.clone());
                    }
                }
            }
        }

        // Update type references with module information
        for type_ref in &mut result.type_references {
            if type_ref.module.is_none() {
                if let Some(module_path) = type_to_module.get(&type_ref.name) {
                    type_ref.module = Some(module_path.clone());
                }
            }
        }
    }
}

pub struct RustModuleResolver;

impl ModuleResolver for RustModuleResolver {
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
                path: base_dir.join("Cargo.toml"), // Point to Cargo.toml as indicator
                is_external: true,
                confidence: 1.0,
            });
        }

        // Handle current crate imports (e.g., my_lib::module)
        // Check if this might be the current crate by looking for Cargo.toml
        let cargo_path = base_dir.join("Cargo.toml");
        if cargo_path.exists() {
            // Try to parse crate name from Cargo.toml
            if let Ok(contents) = std::fs::read_to_string(&cargo_path) {
                // Simple parsing to find package name
                for line in contents.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("name") && trimmed.contains('=') {
                        // Extract the crate name from: name = "my_lib"
                        if let Some(name_part) = trimmed.split('=').nth(1) {
                            let crate_name = name_part.trim().trim_matches('"').trim_matches('\'');
                            if module_path.starts_with(&format!("{crate_name}::")) {
                                // This is a reference to the current crate - treat it like crate::
                                let relative_path = module_path
                                    .strip_prefix(&format!("{crate_name}::"))
                                    .unwrap();
                                let path = ResolverUtils::module_to_path(relative_path);
                                let full_path = base_dir.join("src").join(path);

                                if let Some(resolved) =
                                    ResolverUtils::find_with_extensions(&full_path, &["rs"])
                                {
                                    let validated_path = validate_import_path(base_dir, &resolved)?;
                                    return Ok(ResolvedPath {
                                        path: validated_path,
                                        is_external: false,
                                        confidence: 0.9,
                                    });
                                }

                                // Try as a directory module (mod.rs)
                                let mod_path = full_path.join("mod.rs");
                                if mod_path.exists() {
                                    let validated_path = validate_import_path(base_dir, &mod_path)?;
                                    return Ok(ResolvedPath {
                                        path: validated_path,
                                        is_external: false,
                                        confidence: 0.9,
                                    });
                                }
                                break;
                            }
                        }
                    }
                }
            }
        }

        // Handle crate-relative imports
        if module_path.starts_with("crate::") {
            let relative_path = module_path.strip_prefix("crate::").unwrap();
            let path = ResolverUtils::module_to_path(relative_path);
            let full_path = base_dir.join("src").join(path);

            if let Some(resolved) = ResolverUtils::find_with_extensions(&full_path, &["rs"]) {
                let validated_path = validate_import_path(base_dir, &resolved)?;
                return Ok(ResolvedPath {
                    path: validated_path,
                    is_external: false,
                    confidence: 0.9,
                });
            }

            // Try as a directory module (mod.rs)
            let mod_path = full_path.join("mod.rs");
            if mod_path.exists() {
                let validated_path = validate_import_path(base_dir, &mod_path)?;
                return Ok(ResolvedPath {
                    path: validated_path,
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
                let file_path = parent.join(format!("{module_path}.rs"));
                if file_path.exists() {
                    let validated_path = validate_import_path(base_dir, &file_path)?;
                    return Ok(ResolvedPath {
                        path: validated_path,
                        is_external: false,
                        confidence: 0.9,
                    });
                }

                // Try as a directory module
                let mod_path = parent.join(module_path).join("mod.rs");
                if mod_path.exists() {
                    let validated_path = validate_import_path(base_dir, &mod_path)?;
                    return Ok(ResolvedPath {
                        path: validated_path,
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
