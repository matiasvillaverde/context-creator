//! Semantic analyzer for R

use crate::core::semantic::analyzer::{
    AnalysisResult, LanguageAnalyzer, SemanticContext, SemanticResult,
};
use std::path::Path;
use tree_sitter::Parser;

#[allow(clippy::new_without_default)]
pub struct RAnalyzer {
    #[allow(dead_code)]
    parser: Parser,
}

impl RAnalyzer {
    pub fn new() -> Self {
        let parser = Parser::new();
        // Note: tree_sitter_r::language() would be used here when the crate is added
        // parser.set_language(tree_sitter_r::language()).unwrap();
        Self { parser }
    }
}

impl LanguageAnalyzer for RAnalyzer {
    fn language_name(&self) -> &'static str {
        "R"
    }

    fn analyze_file(
        &self,
        _path: &Path,
        _content: &str,
        _context: &SemanticContext,
    ) -> SemanticResult<AnalysisResult> {
        // TODO: Implement R analysis
        Ok(AnalysisResult::default())
    }

    fn can_handle_extension(&self, extension: &str) -> bool {
        extension == "r" || extension == "R"
    }

    fn supported_extensions(&self) -> Vec<&'static str> {
        vec!["r", "R"]
    }
}
