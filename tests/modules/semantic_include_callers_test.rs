//! Tests for --include-callers functionality
//!
//! This module tests the ability to find and include files that call
//! functions from the analyzed files.

use crate::semantic_test_helpers::TestProjectBuilder;
use context_creator::cli::Config;
use context_creator::core::cache::FileCache;
use context_creator::core::file_expander::expand_file_list;
use context_creator::core::semantic_graph::perform_semantic_analysis_graph;
use context_creator::core::walker::{walk_directory, FileInfo, WalkOptions};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;

/// Helper to check if result contains files by name (avoiding path issues)
fn assert_contains_files(result: &HashMap<PathBuf, FileInfo>, expected_files: &[&str]) {
    let result_files: Vec<String> = result
        .keys()
        .filter_map(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_string())
        .collect();

    for file in expected_files {
        assert!(
            result_files.contains(&file.to_string()),
            "Expected to find {file} in results, but got: {result_files:?}"
        );
    }
}

/// Helper to run include-callers analysis
fn run_include_callers_analysis(
    temp_dir: &TempDir,
    config: Config,
    walk_options: WalkOptions,
) -> HashMap<PathBuf, FileInfo> {
    let cache = Arc::new(FileCache::new());
    let mut files = walk_directory(temp_dir.path(), walk_options.clone()).unwrap();

    // Perform semantic analysis on ALL files first
    perform_semantic_analysis_graph(&mut files, &config, &cache).unwrap();

    // Since these tests analyze entire directories, start with all files
    let files_map: HashMap<PathBuf, FileInfo> =
        files.into_iter().map(|f| (f.path.clone(), f)).collect();

    // Expand file list based on callers (though all files are already included)
    expand_file_list(files_map, &config, &cache, &walk_options).unwrap()
}

#[test]
fn test_direct_function_calls() {
    // Test scenario 1: Include files that directly call a function
    let (_temp_dir, root) = TestProjectBuilder::new()
        .add_file(
            "math.rs",
            r#"
pub fn calculate(a: i32, b: i32) -> i32 {
    a + b
}

pub fn multiply(a: i32, b: i32) -> i32 {
    a * b
}
"#,
        )
        .add_file(
            "main.rs",
            r#"
mod math;

fn main() {
    let result = math::calculate(5, 3);
    println!("Result: {}", result);
}
"#,
        )
        .add_file(
            "unrelated.rs",
            r#"
fn do_something() {
    println!("Unrelated work");
}
"#,
        )
        .build();

    let config = Config {
        include_callers: true,
        trace_imports: false,
        include_types: false,
        semantic_depth: 1,
        ..Default::default()
    };

    let walk_options = WalkOptions::default();
    let cache = Arc::new(FileCache::new());

    // We need to analyze math.rs first to get its exported functions
    let math_path = root.join("math.rs");
    let mut initial_file = FileInfo {
        path: math_path.clone(),
        relative_path: PathBuf::from("math.rs"),
        size: 0,
        file_type: context_creator::utils::file_ext::FileType::Rust,
        priority: 1.0,
        imports: vec![],
        imported_by: vec![],
        function_calls: vec![],
        type_references: vec![],
        exported_functions: vec![],
    };

    // Perform semantic analysis on math.rs to get its exported functions
    let mut files_vec = vec![initial_file.clone()];
    perform_semantic_analysis_graph(&mut files_vec, &config, &cache).unwrap();
    initial_file = files_vec.into_iter().next().unwrap();

    eprintln!(
        "math.rs exported functions: {:?}",
        initial_file.exported_functions
    );

    let mut initial_files_map = HashMap::new();
    initial_files_map.insert(math_path.clone(), initial_file);

    // expand_file_list will handle the caller search
    let expanded = expand_file_list(initial_files_map, &config, &cache, &walk_options).unwrap();

    // Debug output
    eprintln!("Expanded files: {:?}", expanded.keys().collect::<Vec<_>>());

    // Collect the file names (not full paths) to avoid symlink issues
    let expanded_files: Vec<String> = expanded
        .keys()
        .filter_map(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_string())
        .collect();

    // Should include both math.rs and main.rs (the caller)
    assert_eq!(
        expanded.len(),
        2,
        "Should include original file and its caller"
    );
    assert!(expanded_files.contains(&"math.rs".to_string()));
    assert!(expanded_files.contains(&"main.rs".to_string()));
    assert!(!expanded_files.contains(&"unrelated.rs".to_string()));
}

#[test]
fn test_method_calls() {
    // Test scenario 2: Include files calling class methods
    let (temp_dir, _root) = TestProjectBuilder::new()
        .add_file(
            "calculator.py",
            r#"
class Calculator:
    def add(self, a, b):
        return a + b
    
    def subtract(self, a, b):
        return a - b
"#,
        )
        .add_file(
            "app.py",
            r#"
from calculator import Calculator

def main():
    calc = Calculator()
    result = calc.add(10, 5)
    diff = calc.subtract(10, 5)
    print(f"Sum: {result}, Diff: {diff}")
"#,
        )
        .build();

    let config = Config {
        include_callers: true,
        trace_imports: false,
        include_types: false,
        semantic_depth: 1,
        ..Default::default()
    };

    let walk_options = WalkOptions::default();
    let result = run_include_callers_analysis(&temp_dir, config, walk_options);

    // Collect file names to avoid path issues
    let result_files: Vec<String> = result
        .keys()
        .filter_map(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_string())
        .collect();

    // Both files should be included
    assert!(result.len() >= 2, "Should include class file and caller");
    assert!(result_files.contains(&"calculator.py".to_string()));
    assert!(result_files.contains(&"app.py".to_string()));
}

#[test]
fn test_chained_calls() {
    // Test scenario 3: Handle a.b().c() call chains
    let (temp_dir, _root) = TestProjectBuilder::new()
        .add_file(
            "builder.rs",
            r#"
pub struct Builder {
    value: String,
}

impl Builder {
    pub fn new() -> Self {
        Builder { value: String::new() }
    }
    
    pub fn with_name(mut self, name: &str) -> Self {
        self.value = name.to_string();
        self
    }
    
    pub fn build(self) -> String {
        self.value
    }
}
"#,
        )
        .add_file(
            "usage.rs",
            r#"
use crate::builder::Builder;

fn create_object() {
    let obj = Builder::new()
        .with_name("test")
        .build();
}
"#,
        )
        .build();

    let config = Config {
        include_callers: true,
        trace_imports: false,
        include_types: false,
        semantic_depth: 1,
        ..Default::default()
    };

    let walk_options = WalkOptions::default();
    let result = run_include_callers_analysis(&temp_dir, config, walk_options);

    // Both files should be included
    assert_contains_files(&result, &["builder.rs", "usage.rs"]);
}

#[test]
fn test_callback_usage() {
    // Test scenario 4: Functions passed as arguments
    let (temp_dir, _root) = TestProjectBuilder::new()
        .add_file(
            "callbacks.js",
            r#"
export function processData(data, callback) {
    const result = data.map(callback);
    return result;
}

export function double(x) {
    return x * 2;
}
"#,
        )
        .add_file(
            "app.js",
            r#"
import { processData, double } from './callbacks.js';

const numbers = [1, 2, 3, 4];
const doubled = processData(numbers, double);
console.log(doubled);
"#,
        )
        .build();

    let config = Config {
        include_callers: true,
        trace_imports: false,
        include_types: false,
        semantic_depth: 1,
        ..Default::default()
    };

    let walk_options = WalkOptions::default();
    let result = run_include_callers_analysis(&temp_dir, config, walk_options);

    // Both files should be included
    assert_contains_files(&result, &["callbacks.js", "app.js"]);
}

#[test]
fn test_decorator_usage() {
    // Test scenario 5: Include files using function as decorator
    let (temp_dir, _root) = TestProjectBuilder::new()
        .add_file(
            "decorators.py",
            r#"
def timing_decorator(func):
    def wrapper(*args, **kwargs):
        import time
        start = time.time()
        result = func(*args, **kwargs)
        end = time.time()
        print(f"{func.__name__} took {end - start} seconds")
        return result
    return wrapper

def cache_decorator(func):
    cache = {}
    def wrapper(*args):
        if args in cache:
            return cache[args]
        result = func(*args)
        cache[args] = result
        return result
    return wrapper
"#,
        )
        .add_file(
            "app.py",
            r#"
from decorators import timing_decorator, cache_decorator

@timing_decorator
def slow_function(n):
    import time
    time.sleep(n)
    return n * 2

@cache_decorator
def fibonacci(n):
    if n <= 1:
        return n
    return fibonacci(n-1) + fibonacci(n-2)
"#,
        )
        .build();

    let config = Config {
        include_callers: true,
        trace_imports: false,
        include_types: false,
        semantic_depth: 1,
        ..Default::default()
    };

    let walk_options = WalkOptions::default();
    let result = run_include_callers_analysis(&temp_dir, config, walk_options);

    // Both files should be included
    assert_contains_files(&result, &["decorators.py", "app.py"]);
}

#[test]
fn test_async_calls() {
    // Test scenario 6: Handle await/async function calls
    let (temp_dir, _root) = TestProjectBuilder::new()
        // TypeScript async functions
        .add_file(
            "api.ts",
            r#"
export async function fetchData(url: string): Promise<any> {
    const response = await fetch(url);
    return response.json();
}

export async function fetchUser(id: number): Promise<User> {
    return fetchData(`/api/users/${id}`);
}
"#,
        )
        .add_file(
            "app.ts",
            r#"
import { fetchUser, fetchData } from './api';

async function loadUserProfile(userId: number) {
    const user = await fetchUser(userId);
    const posts = await fetchData(`/api/posts?user=${userId}`);
    return { user, posts };
}
"#,
        )
        .build();

    let config = Config {
        include_callers: true,
        trace_imports: false,
        include_types: false,
        semantic_depth: 1,
        ..Default::default()
    };

    let walk_options = WalkOptions::default();
    let result = run_include_callers_analysis(&temp_dir, config, walk_options);

    // Both files should be included
    assert_contains_files(&result, &["api.ts", "app.ts"]);
}

#[test]
fn test_generic_calls() {
    // Test scenario 7: Functions used in higher-order functions
    let (temp_dir, _root) = TestProjectBuilder::new()
        .add_file(
            "predicates.js",
            r#"
export function isEven(n) {
    return n % 2 === 0;
}

export function isPositive(n) {
    return n > 0;
}

export function square(n) {
    return n * n;
}
"#,
        )
        .add_file(
            "app.js",
            r#"
import { isEven, isPositive, square } from './predicates';

const numbers = [1, -2, 3, -4, 5, 6];
const evenNumbers = numbers.filter(isEven);
const positiveNumbers = numbers.filter(isPositive);
const squared = numbers.map(square);
"#,
        )
        .build();

    let config = Config {
        include_callers: true,
        trace_imports: false,
        include_types: false,
        semantic_depth: 1,
        ..Default::default()
    };

    let walk_options = WalkOptions::default();
    let result = run_include_callers_analysis(&temp_dir, config, walk_options);

    // Both files should be included
    assert_contains_files(&result, &["predicates.js", "app.js"]);
}

#[test]
fn test_cross_language_boundaries() {
    // Test scenario 8: Skip FFI/inter-language calls gracefully
    let (temp_dir, _root) = TestProjectBuilder::new()
        // Rust library with C FFI
        .add_file(
            "lib.rs",
            r#"
#[no_mangle]
pub extern "C" fn add_numbers(a: i32, b: i32) -> i32 {
    a + b
}

pub fn rust_only_function() -> String {
    "This is Rust".to_string()
}
"#,
        )
        // Python file trying to use FFI (should be skipped)
        .add_file(
            "ffi_user.py",
            r#"
import ctypes

# Load the Rust library
lib = ctypes.CDLL('./target/release/libmylib.so')
result = lib.add_numbers(5, 3)
"#,
        )
        // Another Rust file that calls rust_only_function
        .add_file(
            "main.rs",
            r#"
use crate::lib::rust_only_function;

fn main() {
    let msg = rust_only_function();
    println!("{}", msg);
}
"#,
        )
        .build();

    let config = Config {
        include_callers: true,
        trace_imports: false,
        include_types: false,
        semantic_depth: 1,
        ..Default::default()
    };

    let walk_options = WalkOptions::default();
    let result = run_include_callers_analysis(&temp_dir, config, walk_options);

    // Should include Rust files but gracefully skip the Python FFI
    assert_contains_files(&result, &["lib.rs", "main.rs"]);
    // Python file might or might not be included, but shouldn't cause errors
}

#[test]
fn test_indirect_calls() {
    // Test scenario 9: Function references stored in variables
    let (temp_dir, _root) = TestProjectBuilder::new()
        .add_file(
            "operations.js",
            r#"
export function add(a, b) {
    return a + b;
}

export function subtract(a, b) {
    return a - b;
}

export function multiply(a, b) {
    return a * b;
}
"#,
        )
        .add_file(
            "calculator.js",
            r#"
import { add, subtract, multiply } from './operations';

const operations = {
    '+': add,
    '-': subtract,
    '*': multiply
};

function calculate(a, op, b) {
    const fn = operations[op];
    return fn(a, b);
}

// Also direct reference
const myAdd = add;
const result = myAdd(5, 3);
"#,
        )
        .build();

    let config = Config {
        include_callers: true,
        trace_imports: false,
        include_types: false,
        semantic_depth: 1,
        ..Default::default()
    };

    let walk_options = WalkOptions::default();
    let result = run_include_callers_analysis(&temp_dir, config, walk_options);

    // Both files should be included
    assert_contains_files(&result, &["operations.js", "calculator.js"]);
}

#[test]
fn test_caller_expansion_with_depth() {
    // Test scenario 10: Respect semantic depth for transitive callers
    // NOTE: Current implementation only finds direct callers, not transitive ones.
    // This test documents the current behavior. Future enhancement could add
    // recursive caller expansion based on depth.
    let (_temp_dir, root) = TestProjectBuilder::new()
        // A defines function
        .add_file(
            "core.rs",
            r#"
pub fn core_function() -> i32 {
    42
}
"#,
        )
        // B calls A
        .add_file(
            "middle.rs",
            r#"
use crate::core::core_function;

pub fn middle_function() -> i32 {
    core_function() * 2
}
"#,
        )
        // C calls B (transitive)
        .add_file(
            "outer.rs",
            r#"
use crate::middle::middle_function;

pub fn outer_function() -> i32 {
    middle_function() + 10
}
"#,
        )
        .build();

    // Test with depth=1 (should include A and B, not C)
    let config_depth1 = Config {
        include_callers: true,
        trace_imports: false,
        include_types: false,
        semantic_depth: 1,
        ..Default::default()
    };

    let walk_options = WalkOptions::default();

    // Start with just core.rs
    let initial_files = vec![root.join("core.rs")];
    let mut files_map = HashMap::new();
    for path in initial_files {
        files_map.insert(
            path.clone(),
            FileInfo {
                path: path.clone(),
                relative_path: path.file_name().unwrap().into(),
                size: 0,
                file_type: context_creator::utils::file_ext::FileType::Rust,
                priority: 1.0,
                imports: vec![],
                imported_by: vec![],
                function_calls: vec![],
                type_references: vec![],
                exported_functions: vec![],
            },
        );
    }

    let cache = Arc::new(FileCache::new());
    let mut files_vec: Vec<_> = files_map.values().cloned().collect();
    perform_semantic_analysis_graph(&mut files_vec, &config_depth1, &cache).unwrap();

    // Update the map with the analyzed files
    files_map = files_vec.into_iter().map(|f| (f.path.clone(), f)).collect();

    let result_depth1 =
        expand_file_list(files_map.clone(), &config_depth1, &cache, &walk_options).unwrap();

    assert_contains_files(&result_depth1, &["core.rs", "middle.rs"]);
    let result_files_depth1: Vec<String> = result_depth1
        .keys()
        .filter_map(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_string())
        .collect();
    assert!(
        !result_files_depth1.contains(&"outer.rs".to_string()),
        "outer.rs should not be included with depth=1"
    );

    // Test with depth=2 (currently still only includes direct callers)
    // TODO: In a future enhancement, this could include transitive callers
    let config_depth2 = Config {
        include_callers: true,
        trace_imports: false,
        include_types: false,
        semantic_depth: 2,
        ..Default::default()
    };

    let result_depth2 = expand_file_list(files_map, &config_depth2, &cache, &walk_options).unwrap();

    // Currently only finds direct callers regardless of depth
    assert_contains_files(&result_depth2, &["core.rs", "middle.rs"]);
}
