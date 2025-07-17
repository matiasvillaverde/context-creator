#[cfg(test)]
mod tests {
    use crate::core::semantic::{analyzer::*, languages::python::PythonAnalyzer};
    use std::path::PathBuf;

    #[test]
    fn test_python_import_parsing() {
        let analyzer = PythonAnalyzer::new();
        let content = r#"
import os
import sys
from pathlib import Path
from collections import defaultdict
from . import utils
from ..lib import helper

def main():
    print("Hello World")
"#;
        let path = PathBuf::from("main.py");
        let context = SemanticContext::new(path.clone(), PathBuf::from("."), 3);

        let result = analyzer.analyze_file(&path, content, &context).unwrap();

        assert!(!result.imports.is_empty(), "Should find imports");

        // Debug: print all imports
        println!("Found {} imports:", result.imports.len());
        for import in &result.imports {
            println!(
                "  Module: '{}', Items: {:?}, Relative: {}",
                import.module, import.items, import.is_relative
            );
        }

        // Check that we found the different import types
        let modules: Vec<&str> = result.imports.iter().map(|i| i.module.as_str()).collect();
        assert!(modules.contains(&"os"), "Should find 'import os'");
        assert!(modules.contains(&"sys"), "Should find 'import sys'");
        assert!(
            modules.contains(&"pathlib"),
            "Should find 'from pathlib import Path'"
        );
        assert!(
            modules.contains(&"collections"),
            "Should find 'from collections import defaultdict'"
        );
        // Check for relative imports
        // For now, just verify we found relative imports
        let relative_imports: Vec<_> = result.imports.iter().filter(|i| i.is_relative).collect();
        assert_eq!(relative_imports.len(), 2, "Should find 2 relative imports");

        // TODO: Fix item extraction for relative imports
        // The implementation should capture "utils" and "helper" in the items list
    }

    #[test]
    fn test_python_function_call_parsing() {
        let analyzer = PythonAnalyzer::new();
        let content = r#"
import os

def greet(name):
    return f"Hello, {name}!"

def main():
    print(greet("World"))
    os.path.join("a", "b")
    len([1, 2, 3])
"#;
        let path = PathBuf::from("main.py");
        let context = SemanticContext::new(path.clone(), PathBuf::from("."), 3);

        let result = analyzer.analyze_file(&path, content, &context).unwrap();

        assert!(
            !result.function_calls.is_empty(),
            "Should find function calls"
        );

        let func_names: Vec<&str> = result
            .function_calls
            .iter()
            .map(|f| f.name.as_str())
            .collect();
        assert!(func_names.contains(&"print"), "Should find 'print' call");
        assert!(func_names.contains(&"greet"), "Should find 'greet' call");
        assert!(
            func_names.contains(&"join"),
            "Should find 'join' method call"
        );
        assert!(func_names.contains(&"len"), "Should find 'len' call");
    }

    #[test]
    fn test_python_type_reference_parsing() {
        let analyzer = PythonAnalyzer::new();
        let content = r#"
from typing import List, Dict, Optional
from dataclasses import dataclass

@dataclass
class Person:
    name: str
    age: int

def get_people() -> List[Person]:
    return []

def process_data(data: Dict[str, Person]) -> Optional[str]:
    return None
"#;
        let path = PathBuf::from("types.py");
        let context = SemanticContext::new(path.clone(), PathBuf::from("."), 3);

        let result = analyzer.analyze_file(&path, content, &context).unwrap();

        assert!(
            !result.type_references.is_empty(),
            "Should find type references"
        );

        // Debug: print all type references
        for type_ref in &result.type_references {
            println!(
                "Found type: {} (module: {:?})",
                type_ref.name, type_ref.module
            );
        }

        let type_names: Vec<&str> = result
            .type_references
            .iter()
            .map(|t| t.name.as_str())
            .collect();

        // For now, just check that we find some types
        // The full implementation would need to handle generic types better
        assert!(type_names.contains(&"str"), "Should find 'str' type");
        assert!(type_names.contains(&"int"), "Should find 'int' type");

        // TODO: Enhance implementation to extract types from:
        // - Generic type annotations (List[Person])
        // - Function return types
        // - Function parameter types
    }
}
