//! Semantic analyzer for Elixir

use crate::core::semantic::analyzer::{
    AnalysisResult, LanguageAnalyzer, SemanticContext, SemanticResult,
};
use std::path::Path;
use tree_sitter::Parser;

#[allow(clippy::new_without_default)]
pub struct ElixirAnalyzer {
    #[allow(dead_code)]
    parser: Parser,
}

impl ElixirAnalyzer {
    pub fn new() -> Self {
        let parser = Parser::new();
        // Note: tree_sitter_elixir::language() would be used here when the crate is added
        // parser.set_language(tree_sitter_elixir::language()).unwrap();
        Self { parser }
    }
}

impl LanguageAnalyzer for ElixirAnalyzer {
    fn language_name(&self) -> &'static str {
        "Elixir"
    }

    fn analyze_file(
        &self,
        _path: &Path,
        _content: &str,
        _context: &SemanticContext,
    ) -> SemanticResult<AnalysisResult> {
        // TODO: Implement Elixir analysis
        Ok(AnalysisResult::default())
    }

    fn can_handle_extension(&self, extension: &str) -> bool {
        extension == "ex" || extension == "exs"
    }

    fn supported_extensions(&self) -> Vec<&'static str> {
        vec!["ex", "exs"]
    }
}
