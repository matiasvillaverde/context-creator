use code_digest::cli::Config;
use code_digest::core::cache::FileCache;
use code_digest::core::walker::{walk_directory, WalkOptions, perform_semantic_analysis};
use std::fs;
use std::sync::Arc;
use tempfile::TempDir;

fn main() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();
    
    // Create a simple Rust project structure
    fs::write(
        root.join("main.rs"),
        r#"
mod lib;
mod utils;

fn main() {
    lib::hello();
    utils::helper();
}
"#,
    )
    .unwrap();
    
    fs::write(
        root.join("lib.rs"),
        r#"
pub fn hello() {
    println!("Hello from lib!");
}
"#,
    )
    .unwrap();
    
    fs::write(
        root.join("utils.rs"),
        r#"
pub fn helper() {
    println!("Helper function");
}
"#,
    )
    .unwrap();
    
    // Create config with semantic analysis enabled
    let config = Config {
        paths: Some(vec![root.to_path_buf()]),
        trace_imports: true,
        include_callers: false,
        include_types: false,
        semantic_depth: 3,
        ..Config::default()
    };
    
    // Create walk options and cache
    let walk_options = WalkOptions::from_config(&config).unwrap();
    let cache = Arc::new(FileCache::new());
    
    // Walk directory and get files
    let mut files = walk_directory(root, walk_options).unwrap();
    println!("Found {} files", files.len());
    
    for file in &files {
        println!("File: {:?}", file.relative_path);
    }
    
    // Perform semantic analysis
    perform_semantic_analysis(&mut files, &config, &cache).unwrap();
    
    // Check results
    for file in &files {
        println!("\nFile: {:?}", file.relative_path);
        println!("  Imports: {:?}", file.imports);
        println!("  Imported by: {:?}", file.imported_by);
        println!("  Function calls: {:?}", file.function_calls);
        println!("  Type references: {:?}", file.type_references);
    }
    
    // Find main.rs
    let main_file = files
        .iter()
        .find(|f| f.relative_path.to_str().unwrap() == "main.rs")
        .unwrap();
    
    println!("\nMain.rs imports: {:?}", main_file.imports);
    println!("Is imports empty? {}", main_file.imports.is_empty());
}