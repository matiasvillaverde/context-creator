#[cfg(test)]
mod tests {
    use crate::core::semantic::{analyzer::*, languages::rust::RustAnalyzer};
    use std::path::PathBuf;

    #[test]
    fn test_rust_module_function_calls() {
        let analyzer = RustAnalyzer::new();
        let content = r#"
mod lib;
mod utils;

use crate::utils::helper;

fn main() {
    lib::greet("World");
    helper();
    let user = lib::User::new("Alice");
    println!("{}", user.name);
}
"#;
        let path = PathBuf::from("main.rs");
        let context = SemanticContext::new(path.clone(), PathBuf::from("."), 3);

        let result = analyzer.analyze_file(&path, content, &context).unwrap();

        assert!(
            !result.function_calls.is_empty(),
            "Should find function calls"
        );

        // Debug: print all function calls
        println!("Found {} function calls:", result.function_calls.len());
        for call in &result.function_calls {
            println!(
                "  Function: '{}', Module: {:?}, Line: {}",
                call.name, call.module, call.line
            );
        }

        let func_names: Vec<&str> = result
            .function_calls
            .iter()
            .map(|f| f.name.as_str())
            .collect();

        // Check for lib::greet call
        assert!(func_names.contains(&"greet"), "Should find 'greet' call");
        let greet_call = result
            .function_calls
            .iter()
            .find(|call| call.name == "greet")
            .expect("Should find greet function call");
        assert_eq!(
            greet_call.module,
            Some("lib".to_string()),
            "greet should be called from lib module"
        );

        // Check for helper call
        assert!(func_names.contains(&"helper"), "Should find 'helper' call");

        // Check for User::new call
        assert!(func_names.contains(&"new"), "Should find 'new' call");
        let new_call = result
            .function_calls
            .iter()
            .find(|call| call.name == "new")
            .expect("Should find new function call");
        assert_eq!(
            new_call.module,
            Some("lib::User".to_string()),
            "new should be called on lib::User"
        );

        // Check for println! macro call
        assert!(
            func_names.contains(&"println"),
            "Should find 'println' call"
        );
    }
}
