//! Semantic analyzer for Python

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
pub struct PythonAnalyzer {
    query_engine: QueryEngine,
}

impl PythonAnalyzer {
    pub fn new() -> Self {
        let language = tree_sitter_python::language();
        let query_engine =
            QueryEngine::new(language, "python").expect("Failed to create Python query engine");
        Self { query_engine }
    }
}

impl LanguageAnalyzer for PythonAnalyzer {
    fn language_name(&self) -> &'static str {
        "Python"
    }

    fn analyze_file(
        &self,
        path: &Path,
        content: &str,
        context: &SemanticContext,
    ) -> SemanticResult<AnalysisResult> {
        let mut parser = Parser::new();
        parser
            .set_language(tree_sitter_python::language())
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
        matches!(extension, "py" | "pyw" | "pyi")
    }

    fn supported_extensions(&self) -> Vec<&'static str> {
        vec!["py", "pyw", "pyi"]
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
        // Validate module name for security - allow Python relative imports
        if !module_path.starts_with('.') {
            validate_module_name(module_path)?;
        } else {
            // For relative imports, do a minimal validation
            if module_path.is_empty() || module_path.len() > 255 || module_path.contains('\0') {
                return Err(ContextCreatorError::SecurityError(format!(
                    "Invalid relative module name: {module_path}"
                )));
            }
        }

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
                // For level=1 (.), stay in current directory
                // For level=2 (..), go up 1 directory  
                // For level=3 (...), go up 2 directories
                for _ in 0..(level.saturating_sub(1)) {
                    if let Some(p) = current.parent() {
                        current = p;
                    }
                }

                // Resolve the rest of the path
                if !rest.is_empty() {
                    let path = ResolverUtils::module_to_path(rest);
                    let full_path = current.join(&path);

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
