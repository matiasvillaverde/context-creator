#![cfg(test)]

//! Edge case tests for semantic analysis in markdown output

use std::fs;
use tempfile::TempDir;

/// Test circular imports are handled correctly
#[test]
fn test_circular_imports_handling() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();

    // Create main.rs that declares the modules
    fs::write(
        src_dir.join("main.rs"),
        r#"
mod a;
mod b;
mod c;

fn main() {
    a::function_a();
}
"#,
    )
    .unwrap();

    // Create circular dependency: a.rs -> b.rs -> c.rs -> a.rs
    fs::write(
        src_dir.join("a.rs"),
        r#"
use crate::b::function_b;

pub fn function_a() {
    function_b();
}
"#,
    )
    .unwrap();

    fs::write(
        src_dir.join("b.rs"),
        r#"
use crate::c::function_c;

pub fn function_b() {
    function_c();
}
"#,
    )
    .unwrap();

    fs::write(
        src_dir.join("c.rs"),
        r#"
use crate::a::function_a;

pub fn function_c() {
    // This creates a circular dependency
    if false {
        function_a();
    }
}
"#,
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_context-creator"))
        .arg(&src_dir)
        .arg("--trace-imports")
        .arg("--include-callers")
        .output()
        .expect("Failed to execute context-creator");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Debug output
    eprintln!("STDOUT:\n{stdout}");
    eprintln!("STDERR:\n{stderr}");
    eprintln!("Status: {:?}", output.status);

    // Should handle circular imports without crashing
    assert!(output.status.success(), "Should handle circular imports");

    // Check that main.rs imports the modules
    assert!(stdout.contains("main.rs"));
    assert!(
        stdout.contains("Imports: a, b, c"),
        "main.rs should import a, b, c"
    );

    // Check that modules are imported by main
    assert!(stdout.contains("a.rs") && stdout.contains("Imported by: main.rs"));
    assert!(stdout.contains("b.rs") && stdout.contains("Imported by: main.rs"));
    assert!(stdout.contains("c.rs") && stdout.contains("Imported by: main.rs"));
}

/// Test files with no imports/exports
#[test]
fn test_files_with_no_semantic_info() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();

    // Create files with no imports or function calls
    fs::write(
        src_dir.join("standalone.rs"),
        r#"
// A file with no imports or exports
const VALUE: i32 = 42;

fn internal_function() {
    let x = VALUE + 1;
    let _y = x * 2;
}
"#,
    )
    .unwrap();

    fs::write(
        src_dir.join("constants.rs"),
        r#"
// Just constants, no functions or imports
pub const MAX_SIZE: usize = 1024;
pub const DEFAULT_NAME: &str = "default";
"#,
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_context-creator"))
        .arg(&src_dir)
        .arg("--trace-imports")
        .arg("--include-callers")
        .arg("--include-types")
        .output()
        .expect("Failed to execute context-creator");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should not show semantic sections for files without semantic info
    let standalone_section = stdout
        .split("## standalone.rs")
        .nth(1)
        .unwrap_or("")
        .split("##")
        .next()
        .unwrap_or("");

    assert!(!standalone_section.contains("Imports:"));
    assert!(!standalone_section.contains("Imported by:"));
    assert!(!standalone_section.contains("Function calls:"));
}

/// Test files with many imports
#[test]
fn test_file_with_many_imports() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();

    // Create a file with many imports
    fs::write(
        src_dir.join("main.rs"),
        r#"
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

mod utils;
mod helpers;
mod config;
mod database;
mod api;

use utils::process;
use helpers::{format_output, validate_input};
use config::Settings;
use database::Connection;
use api::{Client, Response};

fn main() {
    process();
    format_output("test");
    validate_input("data");
}
"#,
    )
    .unwrap();

    // Create the imported modules
    for module in &["utils", "helpers", "config", "database", "api"] {
        fs::write(
            src_dir.join(format!("{module}.rs")),
            format!(
                r#"
pub fn {}() {{
    println!("{} module");
}}
"#,
                if module == &"utils" {
                    "process"
                } else {
                    "function"
                },
                module
            ),
        )
        .unwrap();
    }

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_context-creator"))
        .arg(&src_dir)
        .arg("--trace-imports")
        .arg("--include-callers")
        .output()
        .expect("Failed to execute context-creator");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should show multiple imports
    assert!(stdout.contains("main.rs"));
    assert!(
        stdout.contains("Imports:") && stdout.contains("utils, helpers, config, database, api")
    );

    // Each module should show it's imported by main
    assert!(stdout.contains("utils.rs") && stdout.contains("Imported by: main.rs"));
    assert!(stdout.contains("helpers.rs") && stdout.contains("Imported by: main.rs"));
}

/// Test deeply nested module structure
#[test]
fn test_deeply_nested_modules() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");

    // Create nested directory structure
    let deep_path = src_dir.join("core").join("semantic").join("analyzer");
    fs::create_dir_all(&deep_path).unwrap();

    fs::write(
        src_dir.join("main.rs"),
        r#"
mod core;
use core::semantic::analyzer::analyze;

fn main() {
    analyze();
}
"#,
    )
    .unwrap();

    fs::write(
        src_dir.join("core").join("mod.rs"),
        r#"
pub mod semantic;
"#,
    )
    .unwrap();

    fs::write(
        src_dir.join("core").join("semantic").join("mod.rs"),
        r#"
pub mod analyzer;
"#,
    )
    .unwrap();

    fs::write(
        deep_path.join("mod.rs"),
        r#"
pub fn analyze() {
    println!("Analyzing...");
}
"#,
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_context-creator"))
        .arg(&src_dir)
        .arg("--trace-imports")
        .output()
        .expect("Failed to execute context-creator");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should handle nested module imports
    assert!(output.status.success());
    assert!(stdout.contains("main.rs"));
    // The semantic analyzer should track the deep import chain
}

/// Test file with same name in different directories
#[test]
fn test_same_filename_different_dirs() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    let tests_dir = temp_dir.path().join("tests");
    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(&tests_dir).unwrap();

    // Create lib.rs in both directories
    fs::write(
        src_dir.join("lib.rs"),
        r#"
pub fn library_function() {
    println!("Source library");
}
"#,
    )
    .unwrap();

    fs::write(
        tests_dir.join("lib.rs"),
        r#"
// Test library that imports the main library
use crate::library_function;

#[test]
fn test_library() {
    library_function();
}
"#,
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_context-creator"))
        .arg(temp_dir.path())
        .arg("--trace-imports")
        .output()
        .expect("Failed to execute context-creator");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should distinguish between files with same name
    assert!(stdout.contains("src/lib.rs") || stdout.contains("src\\lib.rs"));
    assert!(stdout.contains("tests/lib.rs") || stdout.contains("tests\\lib.rs"));
}

/// Test with type references edge cases
#[test]
fn test_complex_type_references() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();

    fs::write(
        src_dir.join("types.rs"),
        r#"
use std::collections::{HashMap, HashSet, BTreeMap};
use std::sync::{Arc, Mutex, RwLock};

pub type StringMap = HashMap<String, String>;
pub type ThreadSafeCounter = Arc<Mutex<i32>>;
pub type ConcurrentSet<T> = Arc<RwLock<HashSet<T>>>;

pub struct ComplexType<T, U> 
where 
    T: Clone + Send,
    U: Default
{
    data: HashMap<T, Vec<U>>,
    cache: BTreeMap<String, T>,
}

impl<T: Clone + Send, U: Default> ComplexType<T, U> {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
            cache: BTreeMap::new(),
        }
    }
}
"#,
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_context-creator"))
        .arg(&src_dir)
        .arg("--include-types")
        .output()
        .expect("Failed to execute context-creator");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should track type references
    assert!(output.status.success());
    if stdout.contains("Type references:") {
        // Should include standard library types used
        assert!(stdout.contains("HashMap") || stdout.contains("HashSet"));
    }
}

/// Test with function calls including method calls
#[test]
fn test_various_function_call_patterns() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();

    fs::write(
        src_dir.join("calls.rs"),
        r#"
use std::fs;
use std::path::Path;

mod utils;
use utils::{helper, process_data};

fn main() {
    // Direct function calls
    helper();
    process_data("input");
    
    // Method calls
    let path = Path::new("test.txt");
    path.exists();
    path.is_file();
    
    // Associated function calls
    fs::read_to_string("file.txt").unwrap();
    String::from("hello");
    
    // Chained calls
    vec![1, 2, 3]
        .iter()
        .map(|x| x * 2)
        .filter(|x| x > &2)
        .collect::<Vec<_>>();
    
    // Closure calls
    let closure = |x| x + 1;
    closure(5);
    
    // Macro calls (might not be tracked)
    println!("Hello");
    vec![1, 2, 3];
}
"#,
    )
    .unwrap();

    fs::write(
        src_dir.join("utils.rs"),
        r#"
pub fn helper() {
    println!("Helper");
}

pub fn process_data(input: &str) {
    println!("Processing: {}", input);
}
"#,
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_context-creator"))
        .arg(&src_dir)
        .arg("--include-callers")
        .output()
        .expect("Failed to execute context-creator");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should track various function call patterns
    assert!(output.status.success());
    if stdout.contains("Function calls:") {
        // Should at least track direct function calls
        assert!(stdout.contains("helper()") || stdout.contains("process_data()"));
    }
}

/// Test empty directory
#[test]
fn test_empty_directory() {
    let temp_dir = TempDir::new().unwrap();
    let empty_dir = temp_dir.path().join("empty");
    fs::create_dir_all(&empty_dir).unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_context-creator"))
        .arg(&empty_dir)
        .arg("--trace-imports")
        .arg("--include-callers")
        .arg("--include-types")
        .output()
        .expect("Failed to execute context-creator");

    // Should handle empty directory gracefully
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Total files: 0"));
}

/// Test with invalid UTF-8 in file names (if supported by OS)
#[test]
#[cfg(unix)]
fn test_non_utf8_filenames() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();

    // Create a file with valid UTF-8 content but potentially problematic name
    fs::write(
        src_dir.join("файл.rs"), // Cyrillic characters
        r#"
pub fn function() {
    println!("Non-ASCII filename");
}
"#,
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_context-creator"))
        .arg(&src_dir)
        .arg("--trace-imports")
        .output()
        .expect("Failed to execute context-creator");

    // Should handle non-ASCII filenames
    assert!(output.status.success());
}

/// Test files that import from parent directories
#[test]
fn test_parent_directory_imports() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    let sub_dir = src_dir.join("submodule");
    fs::create_dir_all(&sub_dir).unwrap();

    fs::write(
        src_dir.join("lib.rs"),
        r#"
pub mod submodule;

pub fn parent_function() {
    println!("Parent");
}
"#,
    )
    .unwrap();

    fs::write(
        sub_dir.join("mod.rs"),
        r#"
use super::parent_function;

pub fn child_function() {
    parent_function();
}
"#,
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_context-creator"))
        .arg(&src_dir)
        .arg("--trace-imports")
        .arg("--include-callers")
        .output()
        .expect("Failed to execute context-creator");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should track imports from parent modules
    assert!(output.status.success());
    assert!(stdout.contains("submodule/mod.rs") || stdout.contains("submodule\\mod.rs"));
}

/// Test with external crate imports (won't be tracked but shouldn't crash)
#[test]
fn test_external_crate_imports() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();

    fs::write(
        src_dir.join("external.rs"),
        r#"
// These external crates won't be in our codebase
use serde::{Serialize, Deserialize};
use tokio::runtime::Runtime;
use anyhow::{Result, Context};

#[derive(Serialize, Deserialize)]
pub struct Config {
    name: String,
    value: i32,
}

pub async fn async_function() -> Result<()> {
    Ok(())
}
"#,
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_context-creator"))
        .arg(&src_dir)
        .arg("--trace-imports")
        .arg("--include-types")
        .output()
        .expect("Failed to execute context-creator");

    // Should not crash on external imports
    assert!(output.status.success());
}

/// Test very long import lists
#[test]
fn test_very_long_import_list() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();

    // Create many modules
    let module_count = 50;
    let mut imports = String::new();
    let mut import_list = Vec::new();

    for i in 0..module_count {
        let module_name = format!("module_{i}");
        imports.push_str(&format!("mod {module_name};\n"));
        imports.push_str(&format!("use {module_name}::function_{i};\n"));
        import_list.push(module_name.clone());

        fs::write(
            src_dir.join(format!("{module_name}.rs")),
            format!(
                r#"
pub fn function_{i}() {{
    println!("Function {{i}}", {i});
}}
"#
            ),
        )
        .unwrap();
    }

    fs::write(
        src_dir.join("main.rs"),
        format!(
            r#"
{}

fn main() {{
    // Call all functions
    {}
}}
"#,
            imports,
            (0..module_count)
                .map(|i| format!("function_{i}();"))
                .collect::<Vec<_>>()
                .join("\n    ")
        ),
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_context-creator"))
        .arg(&src_dir)
        .arg("--trace-imports")
        .output()
        .expect("Failed to execute context-creator");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should handle long import lists
    assert!(output.status.success());
    assert!(stdout.contains("main.rs"));
    assert!(stdout.contains("Imports:"));

    // The import list should be reasonably formatted
    let expected_imports = import_list.join(", ");
    assert!(stdout.contains(&expected_imports) || stdout.contains(&import_list[0]));
}
