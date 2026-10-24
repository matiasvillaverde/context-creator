//! Semantic analysis module for context-creator
//!
//! This module provides language-agnostic semantic analysis capabilities including:
//! - Import/dependency tracing
//! - Function call analysis
//! - Type dependency tracking

#![allow(clippy::new_without_default)]

pub mod analyzer;
pub mod cache;
pub mod cycle_detector;
pub mod dependency_types;
pub mod function_call_index;
pub mod graph_builder;
pub mod graph_traverser;
pub mod languages;
pub mod parallel_analyzer;
pub mod parser_pool;
pub mod parsing;
pub mod path_validator;
pub mod query_engine;
pub mod resolver;
pub mod symbols;
pub mod type_resolver;

#[cfg(test)]
mod rust_function_call_test;

// Re-export commonly used types
pub use cache::AstCacheV2;

#[cfg(test)]
mod javascript_test;
#[cfg(test)]
mod python_test;
#[cfg(test)]
mod test;

pub use analyzer::{LanguageAnalyzer, SemanticContext, SemanticResult};
pub use resolver::{ModuleResolver, ResolvedPath};

// Create an alias for get_resolver_for_file
pub use self::get_resolver_for_file as get_module_resolver_for_file;

use crate::utils::error::ContextCreatorError;
use std::path::Path;

/// Semantic analysis options
#[derive(Debug, Clone)]
pub struct SemanticOptions {
    /// Enable import tracing
    pub trace_imports: bool,
    /// Include function callers
    pub include_callers: bool,
    /// Include type dependencies
    pub include_types: bool,
    /// Maximum depth for dependency traversal
    pub semantic_depth: usize,
}

impl SemanticOptions {
    /// Create SemanticOptions from CLI config
    pub fn from_config(config: &crate::cli::Config) -> Self {
        Self {
            trace_imports: config.trace_imports,
            include_callers: config.include_callers,
            include_types: config.include_types,
            semantic_depth: config.semantic_depth,
        }
    }

    /// Check if any semantic analysis is enabled
    pub fn is_enabled(&self) -> bool {
        self.trace_imports || self.include_callers || self.include_types
    }
}

/// Get the appropriate language analyzer for a file
pub fn get_analyzer_for_file(
    path: &Path,
) -> Result<Option<Box<dyn LanguageAnalyzer>>, ContextCreatorError> {
    let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");

    let analyzer: Option<Box<dyn LanguageAnalyzer>> = match extension {
        "rs" => Some(Box::new(languages::rust::RustAnalyzer::new())),
        "py" | "pyi" | "pyw" => Some(Box::new(languages::python::PythonAnalyzer::new())),
        "js" | "jsx" | "mjs" | "cjs" => {
            Some(Box::new(languages::javascript::JavaScriptAnalyzer::new()))
        }
        "ts" | "tsx" | "mts" | "cts" => Some(Box::new(
            languages::typescript::TypeScriptAnalyzer::for_file(path),
        )),
        "go" => Some(Box::new(languages::go::GoAnalyzer::new())),
        "swift" => Some(Box::new(languages::swift::SwiftAnalyzer::new())),
        "java" => Some(Box::new(languages::java::JavaAnalyzer::new())),
        "c" | "h" => Some(Box::new(languages::c::CAnalyzer::new())),
        "cpp" | "cc" | "cxx" | "hpp" | "hh" | "hxx" | "h++" | "c++" => {
            Some(Box::new(languages::cpp::CppAnalyzer::new()))
        }
        "cs" => Some(Box::new(languages::csharp::CSharpAnalyzer::new())),
        "kt" | "kts" => Some(Box::new(languages::kotlin::KotlinAnalyzer::new())),
        "rb" => Some(Box::new(languages::ruby::RubyAnalyzer::new())),
        "php" => Some(Box::new(languages::php::PhpAnalyzer::new())),
        _ => None,
    };

    Ok(analyzer)
}

/// Grammar selection is shared by map extraction and language-specific analysis.
pub fn grammar_for_file(path: &Path) -> Option<tree_sitter::Language> {
    match path.extension()?.to_str()? {
        "rs" => Some(tree_sitter_rust::LANGUAGE.into()),
        "py" | "pyi" | "pyw" => Some(tree_sitter_python::LANGUAGE.into()),
        "js" | "jsx" | "mjs" | "cjs" => Some(tree_sitter_javascript::LANGUAGE.into()),
        "ts" | "mts" | "cts" => Some(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()),
        "tsx" => Some(tree_sitter_typescript::LANGUAGE_TSX.into()),
        "go" => Some(tree_sitter_go::LANGUAGE.into()),
        "swift" => Some(tree_sitter_swift::LANGUAGE.into()),
        "java" => Some(tree_sitter_java::LANGUAGE.into()),
        "c" | "h" => Some(tree_sitter_c::LANGUAGE.into()),
        "cpp" | "cc" | "cxx" | "hpp" | "hh" | "hxx" | "h++" | "c++" => {
            Some(tree_sitter_cpp::LANGUAGE.into())
        }
        "cs" => Some(tree_sitter_c_sharp::LANGUAGE.into()),
        "kt" | "kts" => Some(tree_sitter_kotlin_ng::LANGUAGE.into()),
        "rb" => Some(tree_sitter_ruby::LANGUAGE.into()),
        "php" => Some(tree_sitter_php::LANGUAGE_PHP.into()),
        _ => None,
    }
}

pub(crate) fn is_cpp_header(path: &Path, source: &str) -> bool {
    path.extension().and_then(|extension| extension.to_str()) == Some("h")
        && [
            "class ",
            "namespace ",
            "std::",
            "template<",
            "template <",
            "constexpr ",
        ]
        .iter()
        .any(|marker| source.contains(marker))
}

pub(crate) fn grammar_for_source(path: &Path, source: &str) -> Option<tree_sitter::Language> {
    if is_cpp_header(path, source) {
        Some(tree_sitter_cpp::LANGUAGE.into())
    } else {
        grammar_for_file(path)
    }
}

/// Get the appropriate module resolver for a file
pub fn get_resolver_for_file(
    path: &Path,
) -> Result<Option<Box<dyn ModuleResolver>>, ContextCreatorError> {
    let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");

    let resolver: Option<Box<dyn ModuleResolver>> = match extension {
        "rs" => Some(Box::new(languages::rust::RustModuleResolver)),
        "py" | "pyi" | "pyw" => Some(Box::new(languages::python::PythonModuleResolver)),
        "js" | "jsx" | "mjs" | "cjs" => {
            Some(Box::new(languages::javascript::JavaScriptModuleResolver))
        }
        "ts" | "tsx" | "mts" | "cts" => {
            Some(Box::new(languages::typescript::TypeScriptModuleResolver))
        }
        "go" => Some(Box::new(languages::go::GoModuleResolver)),
        "swift" => Some(Box::new(languages::swift::SwiftModuleResolver)),
        "java" => Some(Box::new(languages::java::JavaModuleResolver)),
        "c" | "h" => Some(Box::new(languages::structural::StructuralResolver(
            languages::structural::Dialect::C,
        ))),
        "cpp" | "cc" | "cxx" | "hpp" | "hh" | "hxx" | "h++" | "c++" => Some(Box::new(
            languages::structural::StructuralResolver(languages::structural::Dialect::Cpp),
        )),
        "cs" => Some(Box::new(languages::structural::StructuralResolver(
            languages::structural::Dialect::CSharp,
        ))),
        "kt" | "kts" => Some(Box::new(languages::structural::StructuralResolver(
            languages::structural::Dialect::Kotlin,
        ))),
        "rb" => Some(Box::new(languages::structural::StructuralResolver(
            languages::structural::Dialect::Ruby,
        ))),
        "php" => Some(Box::new(languages::structural::StructuralResolver(
            languages::structural::Dialect::Php,
        ))),
        _ => None,
    };

    Ok(resolver)
}
