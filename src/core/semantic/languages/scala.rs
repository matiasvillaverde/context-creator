//! Semantic analyzer for Scala

use crate::core::semantic::analyzer::{
    AnalysisResult, LanguageAnalyzer, SemanticContext, SemanticResult,
};
use std::path::Path;
use tree_sitter::Parser;

#[allow(clippy::new_without_default)]
pub struct ScalaAnalyzer {
    #[allow(dead_code)]
    parser: Parser,
}

impl ScalaAnalyzer {
    pub fn new() -> Self {
        let parser = Parser::new();
        // Note: tree_sitter_scala::language() would be used here when the crate is added
        // parser.set_language(tree_sitter_scala::language()).unwrap();
        Self { parser }
    }
}

impl LanguageAnalyzer for ScalaAnalyzer {
    fn language_name(&self) -> &'static str {
        "Scala"
    }

    fn analyze_file(
        &self,
        _path: &Path,
        _content: &str,
        _context: &SemanticContext,
    ) -> SemanticResult<AnalysisResult> {
        // TODO: Implement Scala analysis
        Ok(AnalysisResult::default())
    }

    fn can_handle_extension(&self, extension: &str) -> bool {
        extension == "scala" || extension == "sc"
    }

    fn supported_extensions(&self) -> Vec<&'static str> {
        vec!["scala", "sc"]
    }
}
