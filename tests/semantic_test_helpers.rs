//! Helper functions for semantic analysis tests

use code_digest::cli::Config;
use code_digest::core::cache::FileCache;
use code_digest::core::walker::{walk_directory, FileInfo, WalkOptions};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tempfile::TempDir;

/// Test project builder for creating file structures
pub struct TestProjectBuilder {
    temp_dir: TempDir,
    files: Vec<(PathBuf, String)>,
}

impl TestProjectBuilder {
    /// Create a new test project builder
    pub fn new() -> Self {
        Self {
            temp_dir: TempDir::new().unwrap(),
            files: Vec::new(),
        }
    }

    /// Add a file to the test project
    pub fn add_file<P: AsRef<Path>>(mut self, path: P, content: &str) -> Self {
        self.files
            .push((path.as_ref().to_path_buf(), content.to_string()));
        self
    }

    /// Build the test project and return the root path
    pub fn build(self) -> (TempDir, PathBuf) {
        let root = self.temp_dir.path().to_path_buf();

        for (path, content) in self.files {
            let full_path = root.join(&path);
            if let Some(parent) = full_path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(full_path, content).unwrap();
        }

        (self.temp_dir, root)
    }
}

impl Default for TestProjectBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Analyze a test project with semantic analysis enabled
pub fn analyze_project_with_options(
    root: &Path,
    trace_imports: bool,
    include_callers: bool,
    include_types: bool,
    semantic_depth: usize,
) -> Vec<FileInfo> {
    let config = Config {
        paths: Some(vec![root.to_path_buf()]),
        trace_imports,
        include_callers,
        include_types,
        semantic_depth,
        ..Config::default()
    };

    let walk_options = WalkOptions::from_config(&config).unwrap();
    let cache = Arc::new(FileCache::new());

    let mut files = walk_directory(root, walk_options).unwrap();
    code_digest::core::walker::perform_semantic_analysis(&mut files, &config, &cache).unwrap();

    files
}

/// Analyze a test project with all semantic features enabled
pub fn analyze_test_project(root: &Path) -> Vec<FileInfo> {
    analyze_project_with_options(root, true, true, true, 3)
}

/// Find a file by its relative path in the results
pub fn find_file<'a>(files: &'a [FileInfo], path: &str) -> Option<&'a FileInfo> {
    files.iter().find(|f| {
        let path_str = f.relative_path.to_str().unwrap();
        // Handle both Unix and Windows path separators
        let normalized_path = path_str.replace('\\', "/");
        normalized_path == path || path_str == path
    })
}

/// Find a file by name (without caring about the directory)
#[allow(dead_code)]
pub fn find_file_by_name<'a>(files: &'a [FileInfo], name: &str) -> Option<&'a FileInfo> {
    files.iter().find(|f| {
        f.relative_path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n == name)
            .unwrap_or(false)
    })
}

/// Assert that a file has specific imports
pub fn assert_has_imports(file: &FileInfo, expected_imports: &[&str]) {
    for expected in expected_imports {
        let found = file.imports.iter().any(|import| {
            import
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n == *expected)
                .unwrap_or(false)
        });
        assert!(
            found,
            "Expected import '{}' not found in {:?}. Actual imports: {:?}",
            expected,
            file.relative_path,
            file.imports
                .iter()
                .filter_map(|p| p.file_name())
                .filter_map(|n| n.to_str())
                .collect::<Vec<_>>()
        );
    }
}

/// Assert that a file is imported by specific files
pub fn assert_imported_by(file: &FileInfo, expected_importers: &[&str]) {
    for expected in expected_importers {
        let found = file.imported_by.iter().any(|importer| {
            importer
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n == *expected)
                .unwrap_or(false)
        });
        assert!(
            found,
            "Expected to be imported by '{}' but wasn't. Actual importers: {:?}",
            expected,
            file.imported_by
                .iter()
                .filter_map(|p| p.file_name())
                .filter_map(|n| n.to_str())
                .collect::<Vec<_>>()
        );
    }
}

/// Create a simple Rust project for testing
pub fn create_rust_test_project() -> (TempDir, PathBuf) {
    TestProjectBuilder::new()
        .add_file(
            "main.rs",
            r#"
mod lib;
mod utils;

use crate::utils::helper;

fn main() {
    lib::greet("World");
    helper();
    let user = lib::User::new("Alice");
    println!("{}", user.name);
}
"#,
        )
        .add_file(
            "lib.rs",
            r#"
pub struct User {
    pub name: String,
}

impl User {
    pub fn new(name: &str) -> Self {
        User { name: name.to_string() }
    }
}

pub fn greet(name: &str) {
    println!("Hello, {}!", name);
}
"#,
        )
        .add_file(
            "utils.rs",
            r#"
pub fn helper() {
    println!("Helper function");
}

pub fn unused() {
    println!("This function is not called");
}
"#,
        )
        .build()
}

/// Create a Python test project
#[allow(dead_code)]
pub fn create_python_test_project() -> (TempDir, PathBuf) {
    TestProjectBuilder::new()
        .add_file(
            "main.py",
            r#"
import os
from lib import greet, User
from utils import helper

def main():
    greet("World")
    helper()
    user = User("Alice")
    print(user.name)

if __name__ == "__main__":
    main()
"#,
        )
        .add_file(
            "lib.py",
            r#"
class User:
    def __init__(self, name):
        self.name = name

def greet(name):
    print(f"Hello, {name}!")
"#,
        )
        .add_file(
            "utils.py",
            r#"
def helper():
    print("Helper function")

def unused():
    print("This function is not called")
"#,
        )
        .build()
}

/// Create a JavaScript test project
#[allow(dead_code)]
pub fn create_javascript_test_project() -> (TempDir, PathBuf) {
    TestProjectBuilder::new()
        .add_file(
            "main.js",
            r#"
import { greet, User } from './lib.js';
import { helper } from './utils.js';

function main() {
    greet("World");
    helper();
    const user = new User("Alice");
    console.log(user.name);
}

main();
"#,
        )
        .add_file(
            "lib.js",
            r#"
export class User {
    constructor(name) {
        this.name = name;
    }
}

export function greet(name) {
    console.log(`Hello, ${name}!`);
}
"#,
        )
        .add_file(
            "utils.js",
            r#"
export function helper() {
    console.log("Helper function");
}

export function unused() {
    console.log("This function is not called");
}
"#,
        )
        .build()
}

/// Create a TypeScript test project
pub fn create_typescript_test_project() -> (TempDir, PathBuf) {
    TestProjectBuilder::new()
        .add_file(
            "main.ts",
            r#"
import { greet, User } from './lib';
import { helper } from './utils';
import type { Config } from './types';

const config: Config = {
    debug: true,
    name: "test"
};

function main(): void {
    greet("World");
    helper();
    const user = new User("Alice");
    console.log(user.name);
}

main();
"#,
        )
        .add_file(
            "lib.ts",
            r#"
export class User {
    constructor(public name: string) {}
}

export function greet(name: string): void {
    console.log(`Hello, ${name}!`);
}
"#,
        )
        .add_file(
            "utils.ts",
            r#"
export function helper(): void {
    console.log("Helper function");
}

export function unused(): void {
    console.log("This function is not called");
}
"#,
        )
        .add_file(
            "types.ts",
            r#"
export interface Config {
    debug: boolean;
    name: string;
}

export type Status = 'active' | 'inactive';
"#,
        )
        .build()
}

/// Create a project with circular dependencies
pub fn create_circular_deps_project() -> (TempDir, PathBuf) {
    TestProjectBuilder::new()
        .add_file(
            "a.rs",
            r#"
mod b;
use crate::b::func_b;

pub fn func_a() {
    println!("In func_a");
    func_b();
}
"#,
        )
        .add_file(
            "b.rs",
            r#"
mod c;
use crate::c::func_c;

pub fn func_b() {
    println!("In func_b");
    func_c();
}
"#,
        )
        .add_file(
            "c.rs",
            r#"
mod a;
use crate::a::func_a;

pub fn func_c() {
    println!("In func_c");
    // Would cause infinite recursion if called
    // func_a();
}
"#,
        )
        .build()
}

/// Create a deep dependency chain project
pub fn create_deep_dependency_chain(depth: usize) -> (TempDir, PathBuf) {
    let mut builder = TestProjectBuilder::new();

    for i in 0..depth {
        let content = if i == 0 {
            r#"
mod mod1;
use crate::mod1::func1;

fn main() {
    func1();
}
"#
            .to_string()
        } else if i < depth - 1 {
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
            format!(
                r#"
pub fn func{i}() {{
    println!("End of chain at depth {i}");
}}
"#
            )
        };

        builder = builder.add_file(format!("mod{i}.rs"), &content);
    }

    builder.build()
}
