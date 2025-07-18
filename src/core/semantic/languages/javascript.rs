//! Semantic analyzer for JavaScript

use crate::core::semantic::{
    analyzer::{AnalysisResult, LanguageAnalyzer, SemanticContext, SemanticResult},
    path_validator::{validate_import_path, validate_module_name},
    query_engine::QueryEngine,
    resolver::{ModuleResolver, ResolvedPath},
};
use crate::utils::error::ContextCreatorError;
use std::path::Path;
use tree_sitter::Parser;

#[allow(clippy::new_without_default)]
pub struct JavaScriptAnalyzer {
    query_engine: QueryEngine,
}

impl JavaScriptAnalyzer {
    pub fn new() -> Self {
        let language = tree_sitter_javascript::language();
        let query_engine = QueryEngine::new(language, "javascript")
            .expect("Failed to create JavaScript query engine");
        Self { query_engine }
    }
}

impl LanguageAnalyzer for JavaScriptAnalyzer {
    fn language_name(&self) -> &'static str {
        "JavaScript"
    }

    fn analyze_file(
        &self,
        path: &Path,
        content: &str,
        context: &SemanticContext,
    ) -> SemanticResult<AnalysisResult> {
        let mut parser = Parser::new();
        parser
            .set_language(tree_sitter_javascript::language())
            .map_err(|e| ContextCreatorError::ParseError(format!("Failed to set language: {e}")))?;

        let mut result = self
            .query_engine
            .analyze_with_parser(&mut parser, content)?;

        // Resolve type definitions for the type references found
        self.query_engine.resolve_type_definitions(
            &mut result.type_references,
            path,
            &context.base_dir,
        )?;

        Ok(result)
    }

    fn can_handle_extension(&self, extension: &str) -> bool {
        extension == "js" || extension == "jsx"
    }

    fn supported_extensions(&self) -> Vec<&'static str> {
        vec!["js", "jsx"]
    }
}

pub struct JavaScriptModuleResolver;

impl ModuleResolver for JavaScriptModuleResolver {
    fn resolve_import(
        &self,
        module_path: &str,
        from_file: &Path,
        base_dir: &Path,
    ) -> Result<ResolvedPath, ContextCreatorError> {
        // Validate module name for security
        validate_module_name(module_path)?;

        // Handle Node.js built-in modules
        if self.is_external_module(module_path) {
            return Ok(ResolvedPath {
                path: base_dir.join("package.json"), // Point to package.json as indicator
                is_external: true,
                confidence: 1.0,
            });
        }

        // Handle relative imports (./, ../)
        if module_path.starts_with('.') {
            if let Some(parent) = from_file.parent() {
                // Properly resolve relative paths by removing leading "./"
                let clean_path = if let Some(stripped) = module_path.strip_prefix("./") {
                    stripped // Remove "./" prefix
                } else {
                    module_path // Keep as-is for "../" or other relative paths
                };
                let resolved_path = parent.join(clean_path);

                // Try different extensions
                for ext in &["js", "jsx", "ts", "tsx"] {
                    let with_ext = resolved_path.with_extension(ext);
                    if with_ext.exists() {
                        let validated_path = validate_import_path(base_dir, &with_ext)?;
                        return Ok(ResolvedPath {
                            path: validated_path,
                            is_external: false,
                            confidence: 0.9,
                        });
                    }
                }

                // Try as directory with index file
                for ext in &["js", "jsx", "ts", "tsx"] {
                    let index_path = resolved_path.join(format!("index.{ext}"));
                    if index_path.exists() {
                        let validated_path = validate_import_path(base_dir, &index_path)?;
                        return Ok(ResolvedPath {
                            path: validated_path,
                            is_external: false,
                            confidence: 0.9,
                        });
                    }
                }
            }
        }

        // Handle absolute imports from node_modules or project root
        let search_paths = vec![
            base_dir.to_path_buf(),
            from_file.parent().unwrap_or(base_dir).to_path_buf(),
        ];

        for search_path in &search_paths {
            // Try as a file
            for ext in &["js", "jsx", "ts", "tsx"] {
                let file_path = search_path.join(format!("{module_path}.{ext}"));
                if file_path.exists() {
                    let validated_path = validate_import_path(base_dir, &file_path)?;
                    return Ok(ResolvedPath {
                        path: validated_path,
                        is_external: false,
                        confidence: 0.8,
                    });
                }
            }

            // Try as a directory with index file
            for ext in &["js", "jsx", "ts", "tsx"] {
                let index_path = search_path.join(module_path).join(format!("index.{ext}"));
                if index_path.exists() {
                    let validated_path = validate_import_path(base_dir, &index_path)?;
                    return Ok(ResolvedPath {
                        path: validated_path,
                        is_external: false,
                        confidence: 0.8,
                    });
                }
            }
        }

        // Otherwise, assume it's an external package
        Ok(ResolvedPath {
            path: base_dir.join("package.json"),
            is_external: true,
            confidence: 0.5,
        })
    }

    fn get_file_extensions(&self) -> Vec<&'static str> {
        vec!["js", "jsx"]
    }

    fn is_external_module(&self, module_path: &str) -> bool {
        // Node.js built-in modules
        let builtin_modules = [
            "assert",
            "buffer",
            "child_process",
            "cluster",
            "crypto",
            "dgram",
            "dns",
            "domain",
            "events",
            "fs",
            "http",
            "https",
            "net",
            "os",
            "path",
            "punycode",
            "querystring",
            "readline",
            "repl",
            "stream",
            "string_decoder",
            "tls",
            "tty",
            "url",
            "util",
            "v8",
            "vm",
            "zlib",
            "process",
            "console",
            "timers",
            "module",
        ];

        // Common npm packages
        let common_packages = [
            "react",
            "react-dom",
            "vue",
            "angular",
            "lodash",
            "express",
            "next",
            "webpack",
            "babel",
            "eslint",
            "typescript",
            "jest",
            "mocha",
            "chai",
            "sinon",
            "axios",
            "moment",
            "dayjs",
            "socket.io",
            "cors",
            "helmet",
            "bcrypt",
            "jsonwebtoken",
            "passport",
            "multer",
            "nodemailer",
            "mongoose",
            "sequelize",
            "prisma",
            "graphql",
            "apollo",
            "redux",
            "mobx",
            "zustand",
            "styled-components",
            "emotion",
            "tailwindcss",
        ];

        let first_part = module_path.split('/').next().unwrap_or("");
        builtin_modules.contains(&first_part) || common_packages.contains(&first_part)
    }
}
