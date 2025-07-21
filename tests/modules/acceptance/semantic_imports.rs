//! Category 3: Semantic Analysis - Trace Imports Tests
//!
//! These tests validate the --trace-imports functionality

#![cfg(test)]
#![allow(clippy::needless_borrow)] // project_root is PathBuf, function expects &Path

use super::fixtures::*;
use super::helpers::*;

#[test]
fn scenario_3_1_python_direct_module_import() {
    // Scenario 3.1 (Python): Direct module import
    // CLI Flags: --trace-imports main.py
    // Project Sketch: main.py (imports utils), utils.py (defines functions), api.py (unrelated)
    // Assertion: Output must contain main.py and utils.py. It must NOT contain api.py

    let (_temp_dir, project_root) = create_python_with_imports();

    // Run with trace-imports flag - start from main.py and trace its imports
    let output = run_context_creator(&["--include", "main.py", "--trace-imports"], &project_root);

    // Verify assertions
    assert_contains_file(&output, "main.py");
    assert_contains_file(&output, "utils.py"); // imported by main.py

    // Should NOT include files that aren't imported
    assert_not_contains_file(&output, "api.py");
    assert_not_contains_file(&output, "other.py");

    // Verify the imported functions are present
    assert_contains_code(&output, "def calculate_price(quantity, unit_price):");
    assert_contains_code(&output, "def format_currency(amount):");
}

#[test]
fn scenario_3_2_typescript_relative_file_import() {
    // Scenario 3.2 (TypeScript): Relative file import
    // CLI Flags: --trace-imports src/components/Calendar.tsx
    // Project Sketch: Calendar.tsx (imports ../utils), src/utils.ts (exports formatDate)
    // Assertion: Markdown must contain src/components/Calendar.tsx and src/utils.ts

    let (_temp_dir, project_root) = create_typescript_with_exports();

    // Run with trace-imports flag
    let output = run_context_creator(
        &[
            "--include",
            "src/components/Calendar.tsx",
            "--trace-imports",
        ],
        &project_root,
    );

    // Verify assertions
    assert_contains_file(&output, "src/components/Calendar.tsx");
    assert_contains_file(&output, "src/utils.ts"); // imported by Calendar.tsx

    // Should NOT include files that aren't imported
    assert_not_contains_file(&output, "src/types.ts");
    assert_not_contains_file(&output, "src/handlers.ts");

    // Verify the imported function is present
    assert_contains_code(&output, "export function formatDate(date: Date): string");
}

#[test]
fn scenario_3_3_rust_crate_module_import() {
    // Scenario 3.3 (Rust): Crate/module import
    // CLI Flags: --trace-imports src/main.rs
    // Project Sketch: src/main.rs (uses my_lib::parsing), src/parsing.rs (module)
    // Assertion: Markdown must contain src/main.rs and src/parsing.rs

    let (_temp_dir, project_root) = create_rust_with_modules();

    // Run with trace-imports flag
    let output = run_context_creator(
        &["--include", "src/main.rs", "--trace-imports"],
        &project_root,
    );

    // Verify assertions
    assert_contains_file(&output, "src/main.rs");
    assert_contains_file(&output, "src/parsing.rs"); // imported by main.rs

    // Should also include lib.rs since it exports the module
    assert_contains_file(&output, "src/lib.rs");

    // Should NOT include processing.rs (not imported by main.rs)
    assert_not_contains_file(&output, "src/processing.rs");

    // Verify the imported function is present
    assert_contains_code(&output, "pub fn parse_line(line: &str) -> Vec<String>");
}

#[test]
fn test_deep_import_chain() {
    // Test tracing imports through multiple levels
    use super::builders::*;

    let (_temp_dir, project_root) = PythonProjectBuilder::new()
        .add_file(
            "database.py",
            r#"
# Low-level database module
def connect():
    return "db_connection"
"#,
        )
        .add_file(
            "models.py",
            r#"
from database import connect

def get_user(user_id):
    conn = connect()
    return {"id": user_id, "conn": conn}
"#,
        )
        .add_file(
            "service.py",
            r#"
from models import get_user

def fetch_user_data(user_id):
    user = get_user(user_id)
    return f"User data: {user}"
"#,
        )
        .add_file(
            "api.py",
            r#"
from service import fetch_user_data

def handle_user_request(request):
    user_id = request.get("user_id")
    return fetch_user_data(user_id)
"#,
        )
        .build();

    // Start from api.py and trace all imports
    let output = run_context_creator(&["--include", "api.py", "--trace-imports"], &project_root);

    // Should include the entire import chain
    assert_contains_file(&output, "api.py");
    assert_contains_file(&output, "service.py");
    assert_contains_file(&output, "models.py");
    assert_contains_file(&output, "database.py");
}

#[test]
fn test_circular_imports() {
    // Test handling of circular imports
    use super::builders::*;

    let (_temp_dir, project_root) = TypeScriptProjectBuilder::new()
        .add_file(
            "src/moduleA.ts",
            r#"
import { functionB } from './moduleB';

export function functionA(): string {
    return "A calls " + functionB();
}
"#,
        )
        .add_file(
            "src/moduleB.ts",
            r#"
import { functionA } from './moduleA';

export function functionB(): string {
    return "B";
}

export function callA(): string {
    return functionA();
}
"#,
        )
        .add_file(
            "src/index.ts",
            r#"
import { functionA } from './moduleA';

console.log(functionA());
"#,
        )
        .build();

    // Should handle circular imports without infinite loop
    let output = run_context_creator(
        &["--include", "src/index.ts", "--trace-imports"],
        &project_root,
    );

    // Should include all files in the circular reference
    assert_contains_file(&output, "src/index.ts");
    assert_contains_file(&output, "src/moduleA.ts");
    assert_contains_file(&output, "src/moduleB.ts");
}

#[test]
#[ignore = "Requires deeper changes to Rust module resolution to include intermediate module files"]
fn test_import_from_subdirectories() {
    // Test imports from nested directories
    use super::builders::*;

    let (_temp_dir, project_root) = RustProjectBuilder::new()
        .add_file(
            "src/lib.rs",
            r#"
pub mod core;
pub mod utils;
"#,
        )
        .add_file(
            "src/core/mod.rs",
            r#"
pub mod engine;
pub mod config;
"#,
        )
        .add_file(
            "src/core/engine.rs",
            r#"
use crate::utils::helpers::format_output;

pub fn run() {
    let output = format_output("Engine running");
    println!("{}", output);
}
"#,
        )
        .add_file(
            "src/core/config.rs",
            r#"
pub struct Config {
    pub debug: bool,
}
"#,
        )
        .add_file(
            "src/utils/mod.rs",
            r#"
pub mod helpers;
"#,
        )
        .add_file(
            "src/utils/helpers.rs",
            r#"
pub fn format_output(msg: &str) -> String {
    format!("[{}] {}", chrono::Local::now(), msg)
}
"#,
        )
        .add_file(
            "src/main.rs",
            r#"
use my_lib::core::engine;

fn main() {
    engine::run();
}
"#,
        )
        .build();

    // Trace imports from main.rs
    let output = run_context_creator(
        &["--include", "src/main.rs", "--trace-imports", "--verbose"],
        &project_root,
    );

    // Should include main.rs and all transitively imported files
    assert_contains_file(&output, "src/main.rs");
    assert_contains_file(&output, "src/lib.rs");
    assert_contains_file(&output, "src/core/mod.rs");
    assert_contains_file(&output, "src/core/engine.rs");
    assert_contains_file(&output, "src/utils/mod.rs");
    assert_contains_file(&output, "src/utils/helpers.rs");

    // Should NOT include config.rs (not imported by the chain)
    assert_not_contains_file(&output, "src/core/config.rs");
}

#[test]
fn test_import_only_used_exports() {
    // Test that we include files based on actual imports, not all exports
    use super::builders::*;

    let (_temp_dir, project_root) = TypeScriptProjectBuilder::new()
        .add_file(
            "src/utils.ts",
            r#"
export function usedFunction(): string {
    return "I am used";
}

export function unusedFunction(): string {
    return "I am not imported anywhere";
}

export const USED_CONSTANT = 42;
export const UNUSED_CONSTANT = 100;
"#,
        )
        .add_file(
            "src/consumer.ts",
            r#"
import { usedFunction, USED_CONSTANT } from './utils';

export function consume(): void {
    console.log(usedFunction());
    console.log(USED_CONSTANT);
}
"#,
        )
        .add_file(
            "src/other.ts",
            r#"
// This file doesn't import anything from utils
export function unrelated(): void {
    console.log("I don't use utils");
}
"#,
        )
        .build();

    // Start from consumer.ts
    let output = run_context_creator(
        &["--include", "src/consumer.ts", "--trace-imports"],
        &project_root,
    );

    // Should include consumer and utils
    assert_contains_file(&output, "src/consumer.ts");
    assert_contains_file(&output, "src/utils.ts");

    // Should NOT include other.ts
    assert_not_contains_file(&output, "src/other.ts");

    // Both used and unused exports should be in utils.ts content
    assert_contains_code(&output, "export function usedFunction()");
    assert_contains_code(&output, "export function unusedFunction()");
}
