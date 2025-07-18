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
pub mod graph_builder;
pub mod graph_traverser;
pub mod languages;
pub mod parallel_analyzer;
pub mod parser_pool;
pub mod path_validator;
pub mod query_engine;
pub mod resolver;
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
        "py" => Some(Box::new(languages::python::PythonAnalyzer::new())),
        "js" | "jsx" => Some(Box::new(languages::javascript::JavaScriptAnalyzer::new())),
        "ts" | "tsx" => Some(Box::new(languages::typescript::TypeScriptAnalyzer::new())),
        "go" => Some(Box::new(languages::go::GoAnalyzer::new())),
        "java" => Some(Box::new(languages::java::JavaAnalyzer::new())),
        "cpp" | "cc" | "cxx" | "hpp" | "h" => Some(Box::new(languages::cpp::CppAnalyzer::new())),
        "c" => Some(Box::new(languages::c::CAnalyzer::new())),
        "cs" => Some(Box::new(languages::csharp::CSharpAnalyzer::new())),
        "rb" => Some(Box::new(languages::ruby::RubyAnalyzer::new())),
        "php" => Some(Box::new(languages::php::PhpAnalyzer::new())),
        "swift" => Some(Box::new(languages::swift::SwiftAnalyzer::new())),
        "kt" | "kts" => Some(Box::new(languages::kotlin::KotlinAnalyzer::new())),
        "scala" | "sc" => Some(Box::new(languages::scala::ScalaAnalyzer::new())),
        "dart" => Some(Box::new(languages::dart::DartAnalyzer::new())),
        "lua" => Some(Box::new(languages::lua::LuaAnalyzer::new())),
        "r" | "R" => Some(Box::new(languages::r::RAnalyzer::new())),
        "jl" => Some(Box::new(languages::julia::JuliaAnalyzer::new())),
        "ex" | "exs" => Some(Box::new(languages::elixir::ElixirAnalyzer::new())),
        "elm" => Some(Box::new(languages::elm::ElmAnalyzer::new())),
        _ => None,
    };

    Ok(analyzer)
}

/// Get the appropriate module resolver for a file
pub fn get_resolver_for_file(
    path: &Path,
) -> Result<Option<Box<dyn ModuleResolver>>, ContextCreatorError> {
    let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");

    let resolver: Option<Box<dyn ModuleResolver>> = match extension {
        "rs" => Some(Box::new(languages::rust::RustModuleResolver)),
        "py" => Some(Box::new(languages::python::PythonModuleResolver)),
        "js" | "jsx" => Some(Box::new(languages::javascript::JavaScriptModuleResolver)),
        "ts" | "tsx" => Some(Box::new(languages::javascript::JavaScriptModuleResolver)), // TypeScript uses same resolution as JS
        _ => None,
    };

    Ok(resolver)
}
