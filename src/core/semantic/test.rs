#[cfg(test)]
mod tests {
    use crate::core::semantic::{analyzer::*, languages::rust::RustAnalyzer};
    use std::path::PathBuf;

    #[test]
    fn test_rust_mod_declaration_parsing() {
        let analyzer = RustAnalyzer::new();
        let content = r#"
mod lib;
mod utils;

fn main() {
    lib::hello();
    utils::helper();
}
"#;
        let path = PathBuf::from("main.rs");
        let context = SemanticContext::new(path.clone(), PathBuf::from("."), 3);

        let result = analyzer.analyze_file(&path, content, &context).unwrap();

        assert!(
            !result.imports.is_empty(),
            "Should find mod declarations as imports"
        );
        assert_eq!(result.imports.len(), 2, "Should find 2 imports");

        // Check that we found the mod declarations
        let mod_names: Vec<&str> = result.imports.iter().map(|i| i.module.as_str()).collect();
        assert!(mod_names.contains(&"lib"), "Should find 'mod lib'");
        assert!(mod_names.contains(&"utils"), "Should find 'mod utils'");
    }

    #[test]
    fn test_rust_use_declaration_parsing() {
        let analyzer = RustAnalyzer::new();
        let content = r#"
use std::collections::HashMap;
use crate::utils::helper;

fn main() {
    let map = HashMap::new();
}
"#;
        let path = PathBuf::from("main.rs");
        let context = SemanticContext::new(path.clone(), PathBuf::from("."), 3);

        let result = analyzer.analyze_file(&path, content, &context).unwrap();

        assert!(
            !result.imports.is_empty(),
            "Should find use declarations as imports"
        );
        assert_eq!(result.imports.len(), 2, "Should find 2 imports");
    }

    #[test]
    fn test_rust_function_call_parsing() {
        let analyzer = RustAnalyzer::new();
        let content = r#"
mod utils;

fn main() {
    utils::helper();
    println!("Hello");
}
"#;
        let path = PathBuf::from("main.rs");
        let context = SemanticContext::new(path.clone(), PathBuf::from("."), 3);

        let result = analyzer.analyze_file(&path, content, &context).unwrap();

        assert!(
            !result.function_calls.is_empty(),
            "Should find function calls"
        );
    }
}
