//! Semantic analyzer for Elm

use crate::core::semantic::analyzer::{
    AnalysisResult, LanguageAnalyzer, SemanticContext, SemanticResult,
};
use std::path::Path;
use tree_sitter::Parser;

#[allow(clippy::new_without_default)]
pub struct ElmAnalyzer {
    #[allow(dead_code)]
    #[allow(dead_code)]
    parser: Parser,
}

impl ElmAnalyzer {
    pub fn new() -> Self {
        let parser = Parser::new();
        // Note: tree_sitter_elm::language() would be used here when the crate is added
        // parser.set_language(tree_sitter_elm::language()).unwrap();
        Self { parser }
    }
}

impl LanguageAnalyzer for ElmAnalyzer {
    fn language_name(&self) -> &'static str {
        "Elm"
    }

    fn analyze_file(
        &self,
        _path: &Path,
        _content: &str,
        _context: &SemanticContext,
    ) -> SemanticResult<AnalysisResult> {
        // TODO: Implement Elm analysis
        Ok(AnalysisResult::default())
    }

    fn can_handle_extension(&self, extension: &str) -> bool {
        matches!(extension, "elm")
    }

    fn supported_extensions(&self) -> Vec<&'static str> {
        vec!["elm"]
    }
}
