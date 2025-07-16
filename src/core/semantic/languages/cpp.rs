//! Semantic analyzer for Cpp

use crate::core::semantic::analyzer::{
    AnalysisResult, LanguageAnalyzer, SemanticContext, SemanticResult,
};
use std::path::Path;
use tree_sitter::Parser;

#[allow(clippy::new_without_default)]
pub struct CppAnalyzer {
    #[allow(dead_code)]
    parser: Parser,
}

impl CppAnalyzer {
    pub fn new() -> Self {
        let parser = Parser::new();
        // Note: tree_sitter_cpp::language() would be used here when the crate is added
        // parser.set_language(tree_sitter_cpp::language()).unwrap();
        Self { parser }
    }
}

impl LanguageAnalyzer for CppAnalyzer {
    fn language_name(&self) -> &'static str {
        "Cpp"
    }

    fn analyze_file(
        &self,
        _path: &Path,
        _content: &str,
        _context: &SemanticContext,
    ) -> SemanticResult<AnalysisResult> {
        // TODO: Implement Cpp analysis
        Ok(AnalysisResult::default())
    }

    fn can_handle_extension(&self, extension: &str) -> bool {
        matches!(extension, "cpp" | "cc" | "cxx" | "hpp" | "h")
    }

    fn supported_extensions(&self) -> Vec<&'static str> {
        vec!["cpp", "cc", "cxx", "hpp", "h"]
    }
}
