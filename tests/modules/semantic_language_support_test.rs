#![cfg(test)]

use context_creator::core::semantic::{get_analyzer_for_file, get_resolver_for_file};
use std::path::Path;

#[test]
fn test_semantic_analyzers_are_registered_only_for_implemented_languages() {
    for extension in [
        "rs", "py", "pyi", "js", "jsx", "mjs", "cjs", "ts", "tsx", "mts", "cts", "go", "swift",
        "java", "c", "h", "cpp", "cc", "cxx", "hpp", "hh", "cs", "kt", "kts", "rb", "php",
    ] {
        let path = format!("sample.{extension}");
        assert!(
            get_analyzer_for_file(Path::new(&path)).unwrap().is_some(),
            "{extension} should have an implemented semantic analyzer"
        );
    }

    for extension in [
        "scala", "sc", "dart", "lua", "r", "R", "jl", "ex", "exs", "elm",
    ] {
        let path = format!("sample.{extension}");
        assert!(
            get_analyzer_for_file(Path::new(&path)).unwrap().is_none(),
            "{extension} should not be advertised as semantically implemented"
        );
    }
}

#[test]
fn test_semantic_resolvers_are_registered_for_implemented_dependency_expansion() {
    for extension in [
        "rs", "py", "pyi", "js", "jsx", "mjs", "cjs", "ts", "tsx", "mts", "cts", "go", "swift",
        "java", "c", "h", "cpp", "cc", "cxx", "hpp", "hh", "cs", "kt", "kts", "rb", "php",
    ] {
        let path = format!("sample.{extension}");
        assert!(
            get_resolver_for_file(Path::new(&path)).unwrap().is_some(),
            "{extension} should have a resolver for dependency expansion"
        );
    }
}
