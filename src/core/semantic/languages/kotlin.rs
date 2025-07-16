//! Semantic analyzer for Kotlin

use crate::core::semantic::analyzer::{
    AnalysisResult, LanguageAnalyzer, SemanticContext, SemanticResult,
};
use std::path::Path;
use tree_sitter::Parser;

#[allow(clippy::new_without_default)]
pub struct KotlinAnalyzer {
    #[allow(dead_code)]
    parser: Parser,
}

impl KotlinAnalyzer {
    pub fn new() -> Self {
        let parser = Parser::new();
        // Note: tree_sitter_kotlin::language() would be used here when the crate is added
        // parser.set_language(tree_sitter_kotlin::language()).unwrap();
        Self { parser }
    }
}

impl LanguageAnalyzer for KotlinAnalyzer {
    fn language_name(&self) -> &'static str {
        "Kotlin"
    }

    fn analyze_file(
        &self,
        _path: &Path,
        _content: &str,
        _context: &SemanticContext,
    ) -> SemanticResult<AnalysisResult> {
        // TODO: Implement Kotlin analysis
        Ok(AnalysisResult::default())
    }

    fn can_handle_extension(&self, extension: &str) -> bool {
        extension == "kt" || extension == "kts"
    }

    fn supported_extensions(&self) -> Vec<&'static str> {
        vec!["kt", "kts"]
    }
}
