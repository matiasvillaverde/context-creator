#[cfg(test)]
mod tests {
    use crate::core::semantic::{analyzer::*, languages::javascript::JavaScriptAnalyzer};
    use std::path::PathBuf;

    #[test]
    fn test_javascript_import_parsing() {
        let analyzer = JavaScriptAnalyzer::new();
        let content = r#"
import React from 'react';
import { useState, useEffect } from 'react';
import * as utils from './utils';
import './styles.css';
const fs = require('fs');
const { readFile } = require('fs/promises');

export default function App() {
    return <div>Hello</div>;
}
"#;
        let path = PathBuf::from("App.js");
        let context = SemanticContext::new(path.clone(), PathBuf::from("."), 3);

        let result = analyzer.analyze_file(&path, content, &context).unwrap();

        assert!(!result.imports.is_empty(), "Should find imports");

        let modules: Vec<&str> = result.imports.iter().map(|i| i.module.as_str()).collect();
        assert!(modules.contains(&"react"), "Should find 'react' import");
        assert!(modules.contains(&"./utils"), "Should find './utils' import");
        assert!(
            modules.contains(&"./styles.css"),
            "Should find './styles.css' import"
        );
        assert!(modules.contains(&"fs"), "Should find 'fs' require");
        assert!(
            modules.contains(&"fs/promises"),
            "Should find 'fs/promises' require"
        );
    }

    #[test]
    fn test_javascript_function_call_parsing() {
        let analyzer = JavaScriptAnalyzer::new();
        let content = r#"
function greet(name) {
    return `Hello, ${name}!`;
}

const main = () => {
    console.log(greet("World"));
    setTimeout(() => {
        alert("Done");
    }, 1000);
    
    const arr = [1, 2, 3];
    arr.map(x => x * 2);
    Math.max(...arr);
};
"#;
        let path = PathBuf::from("main.js");
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
        assert!(
            func_names.contains(&"log"),
            "Should find 'console.log' call"
        );
        assert!(func_names.contains(&"greet"), "Should find 'greet' call");
        assert!(
            func_names.contains(&"setTimeout"),
            "Should find 'setTimeout' call"
        );
        assert!(func_names.contains(&"alert"), "Should find 'alert' call");
        assert!(func_names.contains(&"map"), "Should find 'map' method call");
        assert!(func_names.contains(&"max"), "Should find 'Math.max' call");
    }

    #[test]
    fn test_javascript_jsx_parsing() {
        let analyzer = JavaScriptAnalyzer::new();
        let content = r#"
import React from 'react';
import Button from './Button';

function MyComponent({ title, items }) {
    return (
        <div className="container">
            <h1>{title}</h1>
            <ul>
                {items.map(item => (
                    <li key={item.id}>{item.name}</li>
                ))}
            </ul>
            <Button onClick={() => console.log('clicked')}>
                Click me
            </Button>
        </div>
    );
}
"#;
        let path = PathBuf::from("MyComponent.jsx");
        let context = SemanticContext::new(path.clone(), PathBuf::from("."), 3);

        let result = analyzer.analyze_file(&path, content, &context).unwrap();

        // Check for imports
        let modules: Vec<&str> = result.imports.iter().map(|i| i.module.as_str()).collect();
        assert!(modules.contains(&"react"), "Should find 'react' import");
        assert!(
            modules.contains(&"./Button"),
            "Should find './Button' import"
        );

        // Check for JSX type references (components used)
        let type_names: Vec<&str> = result
            .type_references
            .iter()
            .map(|t| t.name.as_str())
            .collect();
        assert!(
            type_names.contains(&"Button"),
            "Should find 'Button' component reference"
        );
    }
}
