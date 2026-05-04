//! Category 5: Complex Flag Combinations Tests
//!
//! These tests validate combinations of multiple semantic analysis flags

#![cfg(test)]
#![allow(clippy::needless_borrow)] // project_root is PathBuf, function expects &Path

use super::helpers::*;

#[test]
fn scenario_5_1_combine_callers_with_ignore() {
    // Scenario 5.1: Combining semantic flags with ignore patterns
    // CLI Flags: --include-callers --ignore "**/test_*.py"
    // Project Sketch: main.py (calls utils), utils.py, test_utils.py (also calls utils)
    // Assertion: Output contains main.py and utils.py, but NOT test_utils.py

    use super::builders::*;

    let (_temp_dir, project_root) = PythonProjectBuilder::new()
        .add_file(
            "utils.py",
            r#"
def calculate(a, b):
    """Calculate sum of two numbers"""
    return a + b

def multiply(a, b):
    """Multiply two numbers"""
    return a * b
"#,
        )
        .add_file(
            "main.py",
            r#"
from utils import calculate, multiply

def main():
    result = calculate(5, 3)
    product = multiply(4, 2)
    print(f"Sum: {result}, Product: {product}")

if __name__ == "__main__":
    main()
"#,
        )
        .add_file(
            "test_utils.py",
            r#"
import unittest
from utils import calculate, multiply

class TestUtils(unittest.TestCase):
    def test_calculate(self):
        self.assertEqual(calculate(2, 3), 5)
    
    def test_multiply(self):
        self.assertEqual(multiply(3, 4), 12)
"#,
        )
        .add_file(
            "other.py",
            r#"
# This file doesn't use utils
def other_function():
    return "unrelated"
"#,
        )
        .build();

    // Include utils.py and find its callers, but ignore test files
    let output = run_context_creator(
        &[
            "--include",
            "utils.py",
            "--include-callers",
            "--ignore",
            "**/test_*.py",
        ],
        &project_root,
    );

    // Should include utils.py and main.py
    assert_contains_file(&output, "utils.py");
    assert_contains_file(&output, "main.py");

    // Should NOT include test files (ignored) or unrelated files
    assert_not_contains_file(&output, "test_utils.py");
    assert_not_contains_file(&output, "other.py");
}

#[test]
fn test_trace_imports_with_types() {
    // Test combining --trace-imports with --include-types
    use super::builders::*;

    let (_temp_dir, project_root) = TypeScriptProjectBuilder::new()
        .add_file(
            "src/types.ts",
            r#"
export interface User {
    id: string;
    name: string;
    email: string;
}

export interface Session {
    token: string;
    user: User;
}
"#,
        )
        .add_file(
            "src/auth.ts",
            r#"
import { User, Session } from './types';

export function createSession(user: User): Session {
    return {
        token: generateToken(),
        user
    };
}

function generateToken(): string {
    return 'token-' + Date.now();
}
"#,
        )
        .add_file(
            "src/handlers.ts",
            r#"
import { createSession } from './auth';
import { User } from './types';

export function handleLogin(username: string): any {
    const user: User = {
        id: '123',
        name: username,
        email: username + '@example.com'
    };
    
    return createSession(user);
}
"#,
        )
        .build();

    // Start from handlers.ts and trace both imports and types
    let output = run_context_creator(
        &[
            "--include",
            "src/handlers.ts",
            "--trace-imports",
            "--include-types",
        ],
        &project_root,
    );

    // Should include all files through both import and type dependencies
    assert_contains_file(&output, "src/handlers.ts");
    assert_contains_file(&output, "src/auth.ts"); // imported
    assert_contains_file(&output, "src/types.ts"); // types used

    // Verify both User and Session types are present
    assert_contains_code(&output, "export interface User");
    assert_contains_code(&output, "export interface Session");
}

#[test]
fn test_all_semantic_flags() {
    // Test all three semantic flags together
    use super::builders::*;

    let (_temp_dir, project_root) = RustProjectBuilder::new()
        .add_file(
            "src/lib.rs",
            r#"
pub mod types;
pub mod core;
pub mod api;
"#,
        )
        .add_file(
            "src/types.rs",
            r#"
#[derive(Debug, Clone)]
pub struct Config {
    pub host: String,
    pub port: u16,
}

#[derive(Debug)]
pub struct Request {
    pub path: String,
    pub body: Vec<u8>,
}
"#,
        )
        .add_file(
            "src/core.rs",
            r#"
use crate::types::{Config, Request};

pub fn process_request(config: &Config, request: &Request) -> String {
    format!("Processing {} on {}:{}", request.path, config.host, config.port)
}

pub fn validate_request(request: &Request) -> bool {
    !request.path.is_empty()
}
"#,
        )
        .add_file(
            "src/api.rs",
            r#"
use crate::core::{process_request, validate_request};
use crate::types::{Config, Request};

pub fn handle_api_request(config: &Config, path: &str) -> String {
    let request = Request {
        path: path.to_string(),
        body: vec![],
    };
    
    if validate_request(&request) {
        process_request(config, &request)
    } else {
        "Invalid request".to_string()
    }
}
"#,
        )
        .add_file(
            "src/main.rs",
            r#"
use my_lib::api::handle_api_request;
use my_lib::types::Config;

fn main() {
    let config = Config {
        host: "localhost".to_string(),
        port: 8080,
    };
    
    let result = handle_api_request(&config, "/users");
    println!("{}", result);
}
"#,
        )
        .build();

    // Start from main.rs with all semantic flags
    let output = run_context_creator(
        &[
            "--include",
            "src/main.rs",
            "--trace-imports",
            "--include-callers",
            "--include-types",
            "--verbose",
        ],
        &project_root,
    );

    // Also run with stderr capture to see debug output
    let mut cmd = super::helpers::context_creator_cmd();
    cmd.current_dir(&project_root).args([
        "--include",
        "src/main.rs",
        "--trace-imports",
        "--include-callers",
        "--include-types",
        "--verbose",
    ]);
    let result = cmd.output().expect("Failed to run command");
    let stderr = String::from_utf8_lossy(&result.stderr);
    eprintln!("\n=== STDERR OUTPUT ===\n{stderr}");

    // Should include everything through various semantic relationships
    assert_contains_file(&output, "src/main.rs");
    assert_contains_file(&output, "src/lib.rs");
    assert_contains_file(&output, "src/api.rs"); // imported
    assert_contains_file(&output, "src/core.rs"); // called by api.rs
    assert_contains_file(&output, "src/types.rs"); // types used

    // Verify key code elements
    assert_contains_code(&output, "pub struct Config");
    assert_contains_code(&output, "pub fn handle_api_request");
    assert_contains_code(&output, "pub fn process_request");
}

#[test]
fn test_semantic_with_glob_patterns() {
    // Test semantic flags with glob include patterns
    use super::builders::*;

    let (_temp_dir, project_root) = PythonProjectBuilder::new()
        .add_file(
            "src/models/user.py",
            r#"
class User:
    def __init__(self, id, name):
        self.id = id
        self.name = name
"#,
        )
        .add_file(
            "src/models/product.py",
            r#"
class Product:
    def __init__(self, id, price):
        self.id = id
        self.price = price
"#,
        )
        .add_file(
            "src/services/user_service.py",
            r#"
from src.models.user import User

def get_user(user_id):
    return User(user_id, "Test User")
"#,
        )
        .add_file(
            "src/api/endpoints.py",
            r#"
from src.services.user_service import get_user

def handle_user_request(user_id):
    user = get_user(user_id)
    return {"id": user.id, "name": user.name}
"#,
        )
        .add_file(
            "tests/test_api.py",
            r#"
from src.api.endpoints import handle_user_request

def test_user_endpoint():
    result = handle_user_request(123)
    assert result["id"] == 123
"#,
        )
        .build();

    // Use glob to include all service files and trace their imports
    let output = run_context_creator(
        &[
            "--include",
            "src/services/*.py",
            "--trace-imports",
            "--ignore",
            "tests/**",
            "--verbose",
        ],
        &project_root,
    );

    // Also run with stderr capture to see debug output
    let mut cmd = super::helpers::context_creator_cmd();
    cmd.current_dir(&project_root).args([
        "--include",
        "src/services/*.py",
        "--trace-imports",
        "--verbose",
    ]);
    let result = cmd.output().expect("Failed to run command");
    let stderr = String::from_utf8_lossy(&result.stderr);
    eprintln!("\n=== STDERR OUTPUT ===\n{stderr}");

    // Should include services and their dependencies
    assert_contains_file(&output, "src/services/user_service.py");
    assert_contains_file(&output, "src/models/user.py"); // imported

    // Should NOT include files not in the import chain or ignored
    assert_not_contains_file(&output, "src/models/product.py");
    assert_not_contains_file(&output, "src/api/endpoints.py");
    assert_not_contains_file(&output, "tests/test_api.py");
}

#[test]
fn scenario_5_2_mock_repository_test() {
    // Scenario 5.2: Remote repository analysis with semantic expansion
    use assert_cmd::prelude::*;
    use predicates::prelude::*;
    use std::fs;
    use std::process::Command;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let mock_bin_dir = temp_dir.path().join("bin");
    fs::create_dir(&mock_bin_dir).unwrap();

    let mock_gh_path = mock_bin_dir.join("gh");
    let mock_git_path = mock_bin_dir.join("git");

    #[cfg(unix)]
    {
        fs::write(
            &mock_gh_path,
            r#"#!/bin/sh
exit 1
"#,
        )
        .unwrap();

        let script = r#"#!/bin/sh
if [ "$1" = "clone" ]; then
    for last; do true; done
    target_dir="$last"
    mkdir -p "$target_dir/src"
    cat > "$target_dir/Cargo.toml" <<'EOF'
[package]
name = "mock-remote"
version = "0.1.0"
edition = "2021"
EOF
    cat > "$target_dir/src/lib.rs" <<'EOF'
mod config;

pub fn run() -> String {
    config::load()
}
EOF
    cat > "$target_dir/src/config.rs" <<'EOF'
pub fn load() -> String {
    "loaded".to_string()
}
EOF
    echo '# Mock Repo' > "$target_dir/README.md"
    exit 0
fi
if [ "$1" = "--version" ]; then
    echo "git version 2.40.0"
    exit 0
fi
exit 1
"#;
        fs::write(&mock_git_path, script).unwrap();

        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&mock_gh_path, fs::Permissions::from_mode(0o755)).unwrap();
        fs::set_permissions(&mock_git_path, fs::Permissions::from_mode(0o755)).unwrap();
    }

    #[cfg(windows)]
    {
        fs::write(
            mock_gh_path.with_extension("bat"),
            "@echo off\r\nexit /b 1\r\n",
        )
        .unwrap();
        fs::write(
            mock_gh_path.with_extension("cmd"),
            "@echo off\r\nexit /b 1\r\n",
        )
        .unwrap();
        let script = r#"@echo off
if "%~1" == "--version" goto version
if "%~1" == "clone" goto clone
exit /b 1

:version
echo git version 2.40.0
exit /b 0

:clone
rem context-creator invokes: git clone --depth 1 <repo_url> <target_dir>
set "target_dir=%~5"
if "%target_dir%" == "" exit /b 1
mkdir "%target_dir%\src" 2>nul
echo [package] > "%target_dir%\Cargo.toml"
echo name = "mock-remote" >> "%target_dir%\Cargo.toml"
echo version = "0.1.0" >> "%target_dir%\Cargo.toml"
echo edition = "2021" >> "%target_dir%\Cargo.toml"
echo mod config; > "%target_dir%\src\lib.rs"
echo pub fn run() -^> String { config::load() } >> "%target_dir%\src\lib.rs"
echo pub fn load() -^> String { "loaded".to_string() } > "%target_dir%\src\config.rs"
echo # Mock Repo > "%target_dir%\README.md"
exit /b 0
"#;
        fs::write(mock_git_path.with_extension("bat"), script).unwrap();
        fs::write(mock_git_path.with_extension("cmd"), script).unwrap();
    }

    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.env("CONTEXT_CREATOR_GH", mock_tool_path(&mock_gh_path))
        .env("CONTEXT_CREATOR_GIT", mock_tool_path(&mock_git_path))
        .arg("--remote")
        .arg("https://github.com/fake/repo")
        .arg("--include")
        .arg("src/lib.rs")
        .arg("--trace-imports");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("src/lib.rs"))
        .stdout(predicate::str::contains("src/config.rs"))
        .stdout(predicate::str::contains("README.md").not());
}

fn mock_tool_path(path: &std::path::Path) -> std::path::PathBuf {
    #[cfg(windows)]
    {
        path.with_extension("cmd")
    }

    #[cfg(not(windows))]
    {
        path.to_path_buf()
    }
}

#[test]
fn test_semantic_depth_limiting() {
    // Test that semantic depth parameter limits import/caller traversal
    use super::builders::*;

    let (_temp_dir, project_root) = PythonProjectBuilder::new()
        .add_file("a.py", "import b")
        .add_file("b.py", "import c")
        .add_file("c.py", "import d")
        .add_file("d.py", "import e")
        .add_file("e.py", "# End of chain")
        .build();

    // Test with depth 2 (should be limited)
    let output = run_context_creator(
        &[
            "--include",
            "a.py",
            "--trace-imports",
            "--semantic-depth",
            "2",
        ],
        &project_root,
    );

    // Should include a.py and some imports, but not the entire chain
    assert_contains_file(&output, "a.py");
    assert_contains_file(&output, "b.py");

    // The exact depth limit depends on the implementation
    // Just verify it doesn't include everything
    let file_count = ["a.py", "b.py", "c.py", "d.py", "e.py"]
        .iter()
        .filter(|f| output.contains(*f))
        .count();

    assert!(
        file_count < 5,
        "Semantic depth should limit import traversal"
    );
}
