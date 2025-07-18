#![cfg(test)]

//! Comprehensive test suite for --trace-imports file expansion functionality
//!
//! This module tests that the --trace-imports flag correctly expands the file list
//! to include all imported files, similar to how --include-types works.

use context_creator::cli::Config;
use context_creator::core::cache::FileCache;
use context_creator::core::walker::{walk_directory, WalkOptions};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;

/// Helper function to perform the full pipeline: walk, analyze, expand
fn process_files_with_trace_imports(
    root: &std::path::Path,
    config: &Config,
) -> HashMap<PathBuf, context_creator::core::walker::FileInfo> {
    let walk_options = WalkOptions::from_config(config).unwrap();
    let cache = Arc::new(FileCache::new());
    let mut files = walk_directory(root, walk_options).unwrap();

    // Debug: Print files before semantic analysis
    eprintln!("Files before semantic analysis: {}", files.len());
    for file in &files {
        eprintln!("  - {}", file.path.display());
    }

    // Perform semantic analysis to populate imports
    context_creator::core::walker::perform_semantic_analysis(&mut files, config, &cache).unwrap();

    // Debug: Print files after semantic analysis
    eprintln!("Files after semantic analysis:");
    for file in &files {
        eprintln!(
            "  - {} with {} imports",
            file.path.display(),
            file.imports.len()
        );
        for imp in &file.imports {
            eprintln!("    -> {}", imp.display());
        }
    }

    // Convert to HashMap for expansion
    let mut files_map = HashMap::new();
    for file in files {
        files_map.insert(file.path.clone(), file);
    }

    // Expand file list based on imports
    let walk_options = context_creator::core::walker::WalkOptions::from_config(config).unwrap();
    context_creator::core::file_expander::expand_file_list(files_map, config, &cache, &walk_options)
        .unwrap()
}

#[test]
fn test_direct_imports_expansion() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create a git repository
    fs::create_dir_all(root.join(".git")).unwrap();

    // Create main.rs that imports from lib.rs
    fs::write(
        root.join("main.rs"),
        r#"
mod lib;
use crate::lib::helper;

fn main() {
    helper::do_something();
}
"#,
    )
    .unwrap();

    // Create lib.rs that won't be included in initial walk
    fs::write(
        root.join("lib.rs"),
        r#"
pub mod helper {
    pub fn do_something() {
        println!("Doing something");
    }
}
"#,
    )
    .unwrap();

    let config = Config {
        paths: Some(vec![root.to_path_buf()]),
        include: Some(vec!["main.rs".to_string()]), // Only include main.rs initially
        trace_imports: true,
        semantic_depth: 2,
        ..Default::default()
    };

    let expanded_files = process_files_with_trace_imports(root, &config);

    // Debug: Print what files we have
    eprintln!("Expanded files count: {}", expanded_files.len());
    for (path, info) in &expanded_files {
        eprintln!("  - File: {}", path.display());
        eprintln!("    Imports: {:?}", info.imports);
        eprintln!("    Imported by: {:?}", info.imported_by);
    }

    // Should expand from 1 file to 2 files
    assert_eq!(
        expanded_files.len(),
        2,
        "Should include both main.rs and imported lib.rs"
    );

    // Verify both files are present
    let file_names: Vec<String> = expanded_files
        .values()
        .map(|f| f.relative_path.to_string_lossy().to_string())
        .collect();

    assert!(file_names.contains(&"main.rs".to_string()));
    assert!(file_names.contains(&"lib.rs".to_string()));

    // Verify import relationship
    let lib_file = expanded_files
        .values()
        .find(|f| f.relative_path.to_str().unwrap() == "lib.rs")
        .unwrap();
    assert!(
        lib_file.imported_by.iter().any(|p| p.ends_with("main.rs")),
        "lib.rs should be marked as imported by main.rs"
    );
}

#[test]
fn test_transitive_imports_depth() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    fs::create_dir_all(root.join(".git")).unwrap();

    // Create chain: main.rs -> module_a.rs -> module_b.rs -> module_c.rs
    fs::write(
        root.join("main.rs"),
        r#"
mod module_a;
use module_a::func_a;

fn main() {
    func_a();
}
"#,
    )
    .unwrap();

    fs::write(
        root.join("module_a.rs"),
        r#"
mod module_b;
use module_b::func_b;

pub fn func_a() {
    func_b();
}
"#,
    )
    .unwrap();

    fs::write(
        root.join("module_b.rs"),
        r#"
mod module_c;
use module_c::func_c;

pub fn func_b() {
    func_c();
}
"#,
    )
    .unwrap();

    fs::write(
        root.join("module_c.rs"),
        r#"
pub fn func_c() {
    println!("Deep function");
}
"#,
    )
    .unwrap();

    // Test with depth = 2 (should include main, module_a, module_b but not module_c)
    let config = Config {
        paths: Some(vec![root.to_path_buf()]),
        include: Some(vec!["main.rs".to_string()]),
        trace_imports: true,
        semantic_depth: 2,
        ..Default::default()
    };

    let expanded_files = process_files_with_trace_imports(root, &config);

    assert_eq!(
        expanded_files.len(),
        3,
        "With depth=2, should include main.rs, module_a.rs, and module_b.rs"
    );

    let file_names: Vec<String> = expanded_files
        .values()
        .map(|f| f.relative_path.to_string_lossy().to_string())
        .collect();

    assert!(file_names.contains(&"main.rs".to_string()));
    assert!(file_names.contains(&"module_a.rs".to_string()));
    assert!(file_names.contains(&"module_b.rs".to_string()));
    assert!(
        !file_names.contains(&"module_c.rs".to_string()),
        "module_c.rs should not be included with depth=2"
    );

    // Test with depth = 3 (should include all files)
    let config_deep = Config {
        paths: Some(vec![root.to_path_buf()]),
        include: Some(vec!["main.rs".to_string()]),
        trace_imports: true,
        semantic_depth: 3,
        ..Default::default()
    };

    let expanded_files_deep = process_files_with_trace_imports(root, &config_deep);

    assert_eq!(
        expanded_files_deep.len(),
        4,
        "With depth=3, should include all four files"
    );
}

#[test]
fn test_circular_imports_handling() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    fs::create_dir_all(root.join(".git")).unwrap();

    // Create circular dependency: a.rs -> b.rs -> c.rs -> a.rs
    fs::write(
        root.join("a.rs"),
        r#"
mod b;
use b::func_b;

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
use c::func_c;

pub fn func_b() {
    func_c();
}
"#,
    )
    .unwrap();

    fs::write(
        root.join("c.rs"),
        r#"
// This would create a cycle in real Rust, but for testing import tracing
use crate::a::func_a;

pub fn func_c() {
    // Would cause infinite loop if called
}
"#,
    )
    .unwrap();

    let config = Config {
        paths: Some(vec![root.to_path_buf()]),
        include: Some(vec!["a.rs".to_string()]),
        trace_imports: true,
        semantic_depth: 10, // High depth to test cycle prevention
        ..Default::default()
    };

    let expanded_files = process_files_with_trace_imports(root, &config);

    // Should include all 3 files exactly once despite the cycle
    assert_eq!(
        expanded_files.len(),
        3,
        "Should include all files exactly once despite circular imports"
    );

    let file_names: Vec<String> = expanded_files
        .values()
        .map(|f| f.relative_path.to_string_lossy().to_string())
        .collect();

    assert!(file_names.contains(&"a.rs".to_string()));
    assert!(file_names.contains(&"b.rs".to_string()));
    assert!(file_names.contains(&"c.rs".to_string()));
}

#[test]
fn test_wildcard_imports() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    fs::create_dir_all(root.join(".git")).unwrap();

    // Create a module with wildcard import
    fs::write(
        root.join("main.py"),
        r#"
from utils import *
from helpers.math import *

def main():
    result = add(1, 2)
    helper_func()
"#,
    )
    .unwrap();

    fs::write(
        root.join("utils.py"),
        r#"
def helper_func():
    print("Helper function")

def another_func():
    print("Another function")
"#,
    )
    .unwrap();

    fs::create_dir_all(root.join("helpers")).unwrap();
    fs::write(
        root.join("helpers/math.py"),
        r#"
def add(a, b):
    return a + b

def multiply(a, b):
    return a * b
"#,
    )
    .unwrap();

    let config = Config {
        paths: Some(vec![root.to_path_buf()]),
        include: Some(vec!["main.py".to_string()]),
        trace_imports: true,
        semantic_depth: 2,
        ..Default::default()
    };

    let expanded_files = process_files_with_trace_imports(root, &config);

    // Should include main.py, utils.py, and helpers/math.py
    assert!(
        expanded_files.len() >= 3,
        "Should include main file and wildcard imported modules"
    );

    let file_paths: Vec<String> = expanded_files
        .values()
        .map(|f| f.path.to_string_lossy().to_string())
        .collect();

    assert!(file_paths.iter().any(|p| p.ends_with("main.py")));
    assert!(file_paths.iter().any(|p| p.ends_with("utils.py")));
    assert!(file_paths
        .iter()
        .any(|p| p.contains("helpers") && p.ends_with("math.py")));
}

#[test]
fn test_relative_imports() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    fs::create_dir_all(root.join(".git")).unwrap();
    fs::create_dir_all(root.join("src/models")).unwrap();
    fs::create_dir_all(root.join("src/utils")).unwrap();

    // Create file with relative imports
    fs::write(
        root.join("src/models/user.py"),
        r#"
from ..utils.validators import validate_email
from .base import BaseModel

class User(BaseModel):
    def __init__(self, email):
        self.email = validate_email(email)
"#,
    )
    .unwrap();

    fs::write(
        root.join("src/models/base.py"),
        r#"
class BaseModel:
    def save(self):
        pass
"#,
    )
    .unwrap();

    fs::write(
        root.join("src/utils/validators.py"),
        r#"
def validate_email(email):
    return email.lower()
"#,
    )
    .unwrap();

    let config = Config {
        paths: Some(vec![root.to_path_buf()]),
        include: Some(vec!["src/models/user.py".to_string()]),
        trace_imports: true,
        semantic_depth: 2,
        ..Default::default()
    };

    let expanded_files = process_files_with_trace_imports(root, &config);

    // Should resolve relative imports and include all referenced files
    assert!(
        expanded_files.len() >= 3,
        "Should include user.py and its relative imports"
    );

    let file_paths: Vec<String> = expanded_files
        .values()
        .map(|f| f.path.to_string_lossy().to_string())
        .collect();

    // Platform-agnostic path checking
    assert!(file_paths
        .iter()
        .any(|p| p.contains("models") && p.ends_with("user.py")));
    assert!(file_paths
        .iter()
        .any(|p| p.contains("models") && p.ends_with("base.py")));
    assert!(file_paths
        .iter()
        .any(|p| p.contains("utils") && p.ends_with("validators.py")));
}

#[test]
fn test_external_vs_workspace() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    fs::create_dir_all(root.join(".git")).unwrap();

    // Create file with both external and internal imports
    fs::write(
        root.join("app.rs"),
        r#"
// External crates (should not be expanded)
use serde::{Serialize, Deserialize};
use tokio::runtime::Runtime;
use anyhow::Result;

// Internal modules (should be expanded)
mod config;
mod handlers;

use config::AppConfig;
use handlers::handle_request;

fn main() -> Result<()> {
    let config = AppConfig::load()?;
    handle_request()?;
    Ok(())
}
"#,
    )
    .unwrap();

    fs::write(
        root.join("config.rs"),
        r#"
pub struct AppConfig {
    pub port: u16,
}

impl AppConfig {
    pub fn load() -> anyhow::Result<Self> {
        Ok(Self { port: 8080 })
    }
}
"#,
    )
    .unwrap();

    fs::write(
        root.join("handlers.rs"),
        r#"
pub fn handle_request() -> anyhow::Result<()> {
    println!("Handling request");
    Ok(())
}
"#,
    )
    .unwrap();

    let config = Config {
        paths: Some(vec![root.to_path_buf()]),
        include: Some(vec!["app.rs".to_string()]),
        trace_imports: true,
        semantic_depth: 2,
        ..Default::default()
    };

    let expanded_files = process_files_with_trace_imports(root, &config);

    // Should include internal files but not external crates
    assert_eq!(
        expanded_files.len(),
        3,
        "Should include app.rs, config.rs, and handlers.rs but not external crates"
    );

    let file_names: Vec<String> = expanded_files
        .values()
        .map(|f| f.relative_path.to_string_lossy().to_string())
        .collect();

    assert!(file_names.contains(&"app.rs".to_string()));
    assert!(file_names.contains(&"config.rs".to_string()));
    assert!(file_names.contains(&"handlers.rs".to_string()));

    // Verify no external crate files are included
    for file_name in &file_names {
        assert!(
            !file_name.contains("serde")
                && !file_name.contains("tokio")
                && !file_name.contains("anyhow"),
            "Should not include external crate files"
        );
    }
}

#[test]
#[ignore] // TODO: Implement re-export detection for Rust
fn test_re_exports() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    fs::create_dir_all(root.join(".git")).unwrap();
    fs::create_dir_all(root.join("core")).unwrap();

    // Create re-export chain: main -> lib -> core/types
    fs::write(
        root.join("main.rs"),
        r#"
use lib::UserData;

fn main() {
    let user = UserData::new("Alice");
}
"#,
    )
    .unwrap();

    fs::write(
        root.join("lib.rs"),
        r#"
// Re-export from core module
pub use core::types::UserData;

pub mod core;
"#,
    )
    .unwrap();

    fs::write(
        root.join("core/mod.rs"),
        r#"
pub mod types;
"#,
    )
    .unwrap();

    fs::write(
        root.join("core/types.rs"),
        r#"
pub struct UserData {
    name: String,
}

impl UserData {
    pub fn new(name: &str) -> Self {
        Self { name: name.to_string() }
    }
}
"#,
    )
    .unwrap();

    let config = Config {
        paths: Some(vec![root.to_path_buf()]),
        include: Some(vec!["main.rs".to_string()]),
        trace_imports: true,
        semantic_depth: 3,
        ..Default::default()
    };

    let expanded_files = process_files_with_trace_imports(root, &config);

    // Should trace through re-exports to include all files in the chain
    assert!(
        expanded_files.len() >= 3,
        "Should include files through re-export chain"
    );

    let file_paths: Vec<String> = expanded_files
        .values()
        .map(|f| f.path.to_string_lossy().to_string())
        .collect();

    assert!(file_paths.iter().any(|p| p.ends_with("main.rs")));
    assert!(file_paths.iter().any(|p| p.ends_with("lib.rs")));
    assert!(
        file_paths.iter().any(|p| p.ends_with("core/types.rs")),
        "Should trace through re-exports to find the actual type definition"
    );
}

#[test]
fn test_import_expansion_with_token_limit() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    fs::create_dir_all(root.join(".git")).unwrap();

    // Create main file
    fs::write(
        root.join("main.rs"),
        r#"
mod large_module;
mod another_module;

use large_module::process;
use another_module::helper;

fn main() {
    process();
    helper();
}
"#,
    )
    .unwrap();

    // Create a large module that would consume many tokens
    let large_content = format!(
        r#"
pub fn process() {{
    // Large function with lots of content
    {}
}}
"#,
        "let x = 1;\n".repeat(1000) // Create a very large file
    );
    fs::write(root.join("large_module.rs"), large_content).unwrap();

    // Create a small module
    fs::write(
        root.join("another_module.rs"),
        r#"
pub fn helper() {
    println!("Helper");
}
"#,
    )
    .unwrap();

    let config = Config {
        paths: Some(vec![root.to_path_buf()]),
        include: Some(vec!["main.rs".to_string()]),
        trace_imports: true,
        semantic_depth: 2,
        max_tokens: Some(500), // Very low token limit
        ..Default::default()
    };

    let expanded_files = process_files_with_trace_imports(root, &config);

    // Even with token limit, file expansion should work
    // The prioritizer will handle which files to include in output
    assert!(
        expanded_files.len() >= 2,
        "Should still expand files even with token limit"
    );

    // Verify that imports were traced
    let main_file = expanded_files
        .values()
        .find(|f| f.relative_path.to_str().unwrap() == "main.rs")
        .unwrap();

    assert!(
        !main_file.imports.is_empty(),
        "Main file should have imports tracked"
    );
}

#[test]
fn test_typescript_imports() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    fs::create_dir_all(root.join(".git")).unwrap();

    // Create TypeScript files with various import styles
    fs::write(
        root.join("index.ts"),
        r#"
import { UserService } from './services/UserService';
import * as utils from './utils';
import Config from './config';

const service = new UserService();
utils.log('Starting app');
"#,
    )
    .unwrap();

    fs::create_dir_all(root.join("services")).unwrap();
    fs::write(
        root.join("services/UserService.ts"),
        r#"
export class UserService {
    constructor() {}
    
    getUser(id: string) {
        return { id, name: 'Test User' };
    }
}
"#,
    )
    .unwrap();

    fs::write(
        root.join("utils.ts"),
        r#"
export function log(message: string) {
    console.log(message);
}

export function format(text: string) {
    return text.trim();
}
"#,
    )
    .unwrap();

    fs::write(
        root.join("config.ts"),
        r#"
export default {
    apiUrl: 'https://api.example.com',
    timeout: 5000
};
"#,
    )
    .unwrap();

    let config = Config {
        paths: Some(vec![root.to_path_buf()]),
        include: Some(vec!["index.ts".to_string()]),
        trace_imports: true,
        semantic_depth: 2,
        ..Default::default()
    };

    let expanded_files = process_files_with_trace_imports(root, &config);

    // Should include all imported TypeScript files
    assert!(
        expanded_files.len() >= 4,
        "Should include index.ts and all imported modules"
    );

    let file_paths: Vec<String> = expanded_files
        .values()
        .map(|f| f.path.to_string_lossy().to_string())
        .collect();

    assert!(file_paths.iter().any(|p| p.ends_with("index.ts")));
    assert!(file_paths
        .iter()
        .any(|p| p.contains("services") && p.ends_with("UserService.ts")));
    assert!(file_paths.iter().any(|p| p.ends_with("utils.ts")));
    assert!(file_paths.iter().any(|p| p.ends_with("config.ts")));
}

#[test]
fn test_mixed_language_imports() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    fs::create_dir_all(root.join(".git")).unwrap();

    // Create a mixed-language project (common in web projects)
    fs::write(
        root.join("server.py"),
        r#"
from handlers import api_handler
from config import settings

def main():
    api_handler.setup_routes()
    print(f"Server running on port {settings.PORT}")
"#,
    )
    .unwrap();

    fs::write(
        root.join("handlers.py"),
        r#"
def setup_routes():
    print("Setting up API routes")
"#,
    )
    .unwrap();

    fs::write(
        root.join("config.py"),
        r#"
class Settings:
    PORT = 8000
    DEBUG = True

settings = Settings()
"#,
    )
    .unwrap();

    // Also create some frontend files that won't be imported
    fs::write(
        root.join("frontend.js"),
        r#"
import { fetchData } from './api.js';

fetchData('/users');
"#,
    )
    .unwrap();

    fs::write(
        root.join("api.js"),
        r#"
export function fetchData(endpoint) {
    return fetch(endpoint);
}
"#,
    )
    .unwrap();

    let config = Config {
        paths: Some(vec![root.to_path_buf()]),
        include: Some(vec!["server.py".to_string()]), // Only start with Python server
        trace_imports: true,
        semantic_depth: 2,
        ..Default::default()
    };

    let expanded_files = process_files_with_trace_imports(root, &config);

    // Should only include Python files that are imported
    let file_names: Vec<String> = expanded_files
        .values()
        .map(|f| f.relative_path.to_string_lossy().to_string())
        .collect();

    assert!(file_names.contains(&"server.py".to_string()));
    assert!(file_names.contains(&"handlers.py".to_string()));
    assert!(file_names.contains(&"config.py".to_string()));

    // Should not include JavaScript files as they're not imported
    assert!(!file_names.contains(&"frontend.js".to_string()));
    assert!(!file_names.contains(&"api.js".to_string()));
}
