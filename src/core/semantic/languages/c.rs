//! Semantic analyzer for C

use crate::core::semantic::analyzer::{
    AnalysisResult, LanguageAnalyzer, SemanticContext, SemanticResult,
};
use std::path::Path;
use tree_sitter::Parser;

#[allow(clippy::new_without_default)]
pub struct CAnalyzer {
    #[allow(dead_code)]
    parser: Parser,
}

impl CAnalyzer {
    pub fn new() -> Self {
        let parser = Parser::new();
        // Note: tree_sitter_c::language() would be used here when the crate is added
        // parser.set_language(tree_sitter_c::language()).unwrap();
        Self { parser }
    }
}

impl LanguageAnalyzer for CAnalyzer {
    fn language_name(&self) -> &'static str {
        "C"
    }

    fn analyze_file(
        &self,
        _path: &Path,
        _content: &str,
        _context: &SemanticContext,
    ) -> SemanticResult<AnalysisResult> {
        // TODO: Implement C analysis
        Ok(AnalysisResult::default())
    }

    fn can_handle_extension(&self, extension: &str) -> bool {
        extension == "c" || extension == "h"
    }

    fn supported_extensions(&self) -> Vec<&'static str> {
        vec!["c", "h"]
    }
}
