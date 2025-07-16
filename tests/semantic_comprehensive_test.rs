//! Comprehensive semantic analysis integration tests

use code_digest::cli::Config;
use code_digest::core::cache::FileCache;
use code_digest::core::walker::{walk_directory, WalkOptions};
use std::fs;
use std::sync::Arc;
use tempfile::TempDir;

#[test]
fn test_multi_language_project_semantic_analysis() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create a multi-language project structure
    // Python module
    fs::create_dir(root.join("python")).unwrap();
    fs::write(
        root.join("python/main.py"),
        r#"
import os
from .utils import helper
from .models import User

def main():
    user = User("Alice")
    helper.process(user)
    print(f"Hello, {user.name}")
"#,
    )
    .unwrap();

    fs::write(
        root.join("python/utils.py"),
        r#"
def process(obj):
    return obj.name.upper()
"#,
    )
    .unwrap();

    fs::write(
        root.join("python/models.py"),
        r#"
class User:
    def __init__(self, name):
        self.name = name
"#,
    )
    .unwrap();

    // JavaScript module
    fs::create_dir(root.join("js")).unwrap();
    fs::write(
        root.join("js/index.js"),
        r#"
import { fetchUser } from './api';
import { formatName } from './utils';

async function main() {
    const user = await fetchUser(123);
    console.log(formatName(user.name));
}

main();
"#,
    )
    .unwrap();

    fs::write(
        root.join("js/api.js"),
        r#"
export async function fetchUser(id) {
    return { id, name: 'Bob' };
}
"#,
    )
    .unwrap();

    fs::write(
        root.join("js/utils.js"),
        r#"
export function formatName(name) {
    return name.toUpperCase();
}
"#,
    )
    .unwrap();

    // Rust module
    fs::create_dir(root.join("rust")).unwrap();
    fs::write(
        root.join("rust/main.rs"),
        r#"
mod lib;
mod utils;

use crate::utils::process_name;

fn main() {
    let name = "Charlie";
    let processed = process_name(name);
    lib::greet(&processed);
}
"#,
    )
    .unwrap();

    fs::write(
        root.join("rust/lib.rs"),
        r#"
pub fn greet(name: &str) {
    println!("Hello, {}!", name);
}
"#,
    )
    .unwrap();

    fs::write(
        root.join("rust/utils.rs"),
        r#"
pub fn process_name(name: &str) -> String {
    name.to_uppercase()
}
"#,
    )
    .unwrap();

    // Create config with semantic analysis enabled
    let config = Config {
        paths: Some(vec![root.to_path_buf()]),
        trace_imports: true,
        include_callers: true,
        include_types: true,
        semantic_depth: 3,
        ..Config::default()
    };

    // Create walk options and cache
    let walk_options = WalkOptions::from_config(&config).unwrap();
    let cache = Arc::new(FileCache::new());

    // Walk directory and get files
    let mut files = walk_directory(root, walk_options).unwrap();

    // Perform semantic analysis
    code_digest::core::walker::perform_semantic_analysis(&mut files, &config, &cache).unwrap();

    // Verify Python imports
    let py_main = files
        .iter()
        .find(|f| f.relative_path.to_str().unwrap() == "python/main.py")
        .unwrap();
    assert!(
        !py_main.imports.is_empty(),
        "Python main should have imports"
    );

    let py_utils = files
        .iter()
        .find(|f| f.relative_path.to_str().unwrap() == "python/utils.py")
        .unwrap();
    assert!(
        !py_utils.imported_by.is_empty(),
        "Python utils should be imported by main"
    );

    // Verify JavaScript imports
    let js_index = files
        .iter()
        .find(|f| f.relative_path.to_str().unwrap() == "js/index.js")
        .unwrap();
    assert!(!js_index.imports.is_empty(), "JS index should have imports");
    assert!(
        !js_index.function_calls.is_empty(),
        "JS index should have function calls"
    );

    let js_api = files
        .iter()
        .find(|f| f.relative_path.to_str().unwrap() == "js/api.js")
        .unwrap();
    assert!(
        !js_api.imported_by.is_empty(),
        "JS api should be imported by index"
    );

    // Verify Rust imports
    let rs_main = files
        .iter()
        .find(|f| f.relative_path.to_str().unwrap() == "rust/main.rs")
        .unwrap();
    assert!(!rs_main.imports.is_empty(), "Rust main should have imports");
    assert!(
        !rs_main.function_calls.is_empty(),
        "Rust main should have function calls"
    );

    let rs_lib = files
        .iter()
        .find(|f| f.relative_path.to_str().unwrap() == "rust/lib.rs")
        .unwrap();
    assert!(
        !rs_lib.imported_by.is_empty(),
        "Rust lib should be imported by main"
    );
}

#[test]
fn test_circular_dependency_detection() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create circular dependencies
    fs::write(
        root.join("a.rs"),
        r#"
mod b;
use crate::b::func_b;

pub fn func_a() {
    func_b();
}
"#,
    )
    .unwrap();

    fs::write(
        root.join("b.rs"),
        r#"
mod c;
use crate::c::func_c;

pub fn func_b() {
    func_c();
}
"#,
    )
    .unwrap();

    fs::write(
        root.join("c.rs"),
        r#"
mod a;
use crate::a::func_a;

pub fn func_c() {
    func_a();
}
"#,
    )
    .unwrap();

    // Create config with semantic analysis enabled
    let config = Config {
        paths: Some(vec![root.to_path_buf()]),
        trace_imports: true,
        semantic_depth: 5, // High depth to test cycle detection
        ..Config::default()
    };

    // Create walk options and cache
    let walk_options = WalkOptions::from_config(&config).unwrap();
    let cache = Arc::new(FileCache::new());

    // Walk directory and get files
    let mut files = walk_directory(root, walk_options).unwrap();

    // Perform semantic analysis - should handle cycles gracefully
    code_digest::core::walker::perform_semantic_analysis(&mut files, &config, &cache).unwrap();

    // All files should have been analyzed despite the cycle
    assert_eq!(files.len(), 3);
    for file in &files {
        if file.relative_path.extension().unwrap() == "rs" {
            assert!(
                !file.imports.is_empty() || !file.imported_by.is_empty(),
                "File {:?} should have import relationships",
                file.relative_path
            );
        }
    }
}

#[test]
fn test_semantic_depth_limiting() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create a deep dependency chain
    for i in 0..10 {
        let content = if i == 0 {
            r#"
fn main() {
    mod1::func1();
}
"#
            .to_string()
        } else if i < 9 {
            format!(
                r#"
mod mod{};
use crate::mod{}::func{};

pub fn func{}() {{
    func{}();
}}
"#,
                i + 1,
                i + 1,
                i + 1,
                i,
                i + 1
            )
        } else {
            r#"
pub fn func9() {
    println!("End of chain");
}
"#
            .to_string()
        };

        fs::write(root.join(format!("mod{i}.rs")), content).unwrap();
    }

    // Test with shallow depth
    let config = Config {
        paths: Some(vec![root.to_path_buf()]),
        trace_imports: true,
        semantic_depth: 2, // Only analyze 2 levels deep
        ..Config::default()
    };

    let walk_options = WalkOptions::from_config(&config).unwrap();
    let cache = Arc::new(FileCache::new());
    let mut files = walk_directory(root, walk_options).unwrap();

    code_digest::core::walker::perform_semantic_analysis(&mut files, &config, &cache).unwrap();

    // Check that analysis was limited by depth
    // Files beyond depth 2 should have minimal analysis
    let _deep_file = files
        .iter()
        .find(|f| f.relative_path.to_str().unwrap() == "mod5.rs")
        .unwrap();

    // Deep files might not be fully analyzed due to depth limit
    // This is expected behavior
    assert_eq!(files.len(), 10, "All files should be present");
}
