//! End-to-End tests for code-digest
//!
//! These tests verify complete user workflows from CLI invocation to final output,
//! testing real-world scenarios and edge cases that users might encounter.

use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

/// Create a realistic Rust project structure for testing
fn create_realistic_rust_project(temp_dir: &Path) -> std::path::PathBuf {
    let project_dir = temp_dir.join("rust_project");
    fs::create_dir_all(&project_dir).unwrap();

    // Create Cargo.toml
    fs::write(
        project_dir.join("Cargo.toml"),
        r#"
[package]
name = "example-project"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { version = "1.0", features = ["derive"] }
tokio = { version = "1.0", features = ["full"] }
anyhow = "1.0"

[dev-dependencies]
tempfile = "3.0"
"#,
    )
    .unwrap();

    // Create main application structure
    fs::create_dir_all(project_dir.join("src")).unwrap();
    fs::write(
        project_dir.join("src/main.rs"),
        r#"
//! Example application main entry point

use anyhow::Result;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    database_url: String,
    port: u16,
    workers: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            database_url: "postgresql://localhost/app".to_string(),
            port: 8080,
            workers: 4,
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::default();
    
    println!("Starting server on port {}", config.port);
    println!("Database URL: {}", config.database_url);
    println!("Workers: {}", config.workers);
    
    // Simulate some async work
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    
    Ok(())
}

fn calculate_fibonacci(n: u32) -> u64 {
    match n {
        0 => 0,
        1 => 1,
        _ => calculate_fibonacci(n - 1) + calculate_fibonacci(n - 2),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fibonacci() {
        assert_eq!(calculate_fibonacci(0), 0);
        assert_eq!(calculate_fibonacci(1), 1);
        assert_eq!(calculate_fibonacci(10), 55);
    }
    
    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.port, 8080);
        assert_eq!(config.workers, 4);
    }
}
"#,
    )
    .unwrap();

    fs::write(
        project_dir.join("src/lib.rs"),
        r#"
//! Library module for example project

pub mod handlers;
pub mod models;
pub mod utils;

use anyhow::Result;
use std::collections::HashMap;

/// Database connection pool
pub struct Database {
    connections: HashMap<String, String>,
}

impl Database {
    pub fn new() -> Self {
        Self {
            connections: HashMap::new(),
        }
    }
    
    pub fn connect(&mut self, name: &str, url: &str) -> Result<()> {
        self.connections.insert(name.to_string(), url.to_string());
        Ok(())
    }
    
    pub fn execute_query(&self, query: &str) -> Result<Vec<String>> {
        // Simulate query execution
        Ok(vec![format!("Result for: {}", query)])
    }
}

impl Default for Database {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_creation() {
        let db = Database::new();
        assert!(db.connections.is_empty());
    }
    
    #[test]
    fn test_database_connection() {
        let mut db = Database::new();
        assert!(db.connect("main", "postgresql://localhost/test").is_ok());
        assert_eq!(db.connections.len(), 1);
    }
}
"#,
    )
    .unwrap();

    // Create subdirectories and modules
    fs::create_dir_all(project_dir.join("src/handlers")).unwrap();
    fs::write(
        project_dir.join("src/handlers/mod.rs"),
        r#"
//! HTTP request handlers

pub mod auth;
pub mod api;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Response<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> Response<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }
    
    pub fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message),
        }
    }
}
"#,
    )
    .unwrap();

    fs::write(
        project_dir.join("src/handlers/auth.rs"),
        r#"
//! Authentication handlers

use super::Response;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub id: u64,
    pub username: String,
    pub email: String,
}

pub fn login(request: LoginRequest) -> Response<User> {
    // Simulate authentication
    if request.username == "admin" && request.password == "password" {
        Response::success(User {
            id: 1,
            username: request.username,
            email: "admin@example.com".to_string(),
        })
    } else {
        Response::error("Invalid credentials".to_string())
    }
}

pub fn logout() -> Response<()> {
    Response::success(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_login() {
        let request = LoginRequest {
            username: "admin".to_string(),
            password: "password".to_string(),
        };
        let response = login(request);
        assert!(response.success);
        assert!(response.data.is_some());
    }
    
    #[test]
    fn test_invalid_login() {
        let request = LoginRequest {
            username: "user".to_string(),
            password: "wrong".to_string(),
        };
        let response = login(request);
        assert!(!response.success);
        assert!(response.error.is_some());
    }
}
"#,
    )
    .unwrap();

    fs::write(
        project_dir.join("src/handlers/api.rs"),
        r#"
//! API handlers

use super::Response;
use crate::models::Product;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ListRequest {
    pub page: Option<u32>,
    pub limit: Option<u32>,
}

pub fn list_products(request: ListRequest) -> Response<Vec<Product>> {
    let page = request.page.unwrap_or(1);
    let limit = request.limit.unwrap_or(10);
    
    // Simulate product list
    let products = (1..=limit).map(|i| Product {
        id: (page - 1) * limit + i,
        name: format!("Product {}", i),
        price: 10.0 + (i as f64),
        description: format!("Description for product {}", i),
    }).collect();
    
    Response::success(products)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_products_default() {
        let request = ListRequest { page: None, limit: None };
        let response = list_products(request);
        assert!(response.success);
        assert_eq!(response.data.unwrap().len(), 10);
    }
    
    #[test]
    fn test_list_products_custom() {
        let request = ListRequest { page: Some(2), limit: Some(5) };
        let response = list_products(request);
        assert!(response.success);
        let products = response.data.unwrap();
        assert_eq!(products.len(), 5);
        assert_eq!(products[0].id, 6); // Second page, first item
    }
}
"#,
    )
    .unwrap();

    fs::create_dir_all(project_dir.join("src/models")).unwrap();
    fs::write(
        project_dir.join("src/models/mod.rs"),
        r#"
//! Data models

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub id: u32,
    pub name: String,
    pub price: f64,
    pub description: String,
}

impl Product {
    pub fn new(id: u32, name: String, price: f64, description: String) -> Self {
        Self { id, name, price, description }
    }
    
    pub fn formatted_price(&self) -> String {
        format!("${:.2}", self.price)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_product_creation() {
        let product = Product::new(
            1,
            "Test Product".to_string(),
            19.99,
            "A test product".to_string(),
        );
        assert_eq!(product.id, 1);
        assert_eq!(product.name, "Test Product");
    }
    
    #[test]
    fn test_formatted_price() {
        let product = Product::new(1, "Test".to_string(), 19.99, "Test".to_string());
        assert_eq!(product.formatted_price(), "$19.99");
    }
}
"#,
    )
    .unwrap();

    fs::create_dir_all(project_dir.join("src/utils")).unwrap();
    fs::write(
        project_dir.join("src/utils/mod.rs"),
        r#"
//! Utility functions

use std::collections::HashMap;

pub fn slugify(input: &str) -> String {
    input
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

pub fn parse_query_params(query: &str) -> HashMap<String, String> {
    query
        .split('&')
        .filter_map(|pair| {
            let mut split = pair.split('=');
            match (split.next(), split.next()) {
                (Some(key), Some(value)) => Some((key.to_string(), value.to_string())),
                _ => None,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slugify() {
        assert_eq!(slugify("Hello World!"), "hello-world");
        assert_eq!(slugify("This is a Test"), "this-is-a-test");
        assert_eq!(slugify("NoSpaces"), "nospaces");
    }
    
    #[test]
    fn test_parse_query_params() {
        let params = parse_query_params("name=John&age=30&city=NYC");
        assert_eq!(params.len(), 3);
        assert_eq!(params.get("name"), Some(&"John".to_string()));
        assert_eq!(params.get("age"), Some(&"30".to_string()));
    }
}
"#,
    )
    .unwrap();

    // Create tests directory
    fs::create_dir_all(project_dir.join("tests")).unwrap();
    fs::write(
        project_dir.join("tests/integration_test.rs"),
        r#"
//! Integration tests

use example_project::Database;

#[test]
fn test_database_integration() {
    let mut db = Database::new();
    assert!(db.connect("test", "sqlite::memory:").is_ok());
    
    let results = db.execute_query("SELECT * FROM users").unwrap();
    assert!(!results.is_empty());
}
"#,
    )
    .unwrap();

    // Create README and other files
    fs::write(
        project_dir.join("README.md"),
        r#"
# Example Project

This is an example Rust project for testing code-digest functionality.

## Features

- Async web server with Tokio
- Database abstraction layer
- Authentication system
- REST API endpoints
- Comprehensive test coverage

## Usage

```bash
cargo run
```

## Testing

```bash
cargo test
```

## Building

```bash
cargo build --release
```
"#,
    )
    .unwrap();

    fs::write(
        project_dir.join(".gitignore"),
        r#"
/target/
**/*.rs.bk
*.pdb
.DS_Store
*.log
"#,
    )
    .unwrap();

    // Create a config file
    fs::write(
        project_dir.join("config.toml"),
        r#"
[database]
url = "postgresql://localhost/example"
max_connections = 10

[server]
host = "0.0.0.0"
port = 8080
workers = 4

[logging]
level = "info"
file = "app.log"
"#,
    )
    .unwrap();

    project_dir
}

/// Create a Python project for testing multi-language support
fn create_python_project(temp_dir: &Path) -> std::path::PathBuf {
    let project_dir = temp_dir.join("python_project");
    fs::create_dir_all(&project_dir).unwrap();

    // Create setup.py
    fs::write(
        project_dir.join("setup.py"),
        r#"
from setuptools import setup, find_packages

setup(
    name="example-python-project",
    version="0.1.0",
    packages=find_packages(),
    install_requires=[
        "requests>=2.25.0",
        "click>=7.0",
        "pydantic>=1.8.0",
    ],
    entry_points={
        "console_scripts": [
            "example=example.cli:main",
        ],
    },
)
"#,
    )
    .unwrap();

    // Create package structure
    fs::create_dir_all(project_dir.join("example")).unwrap();
    fs::write(
        project_dir.join("example/__init__.py"),
        r#"
"""Example Python package for testing code-digest."""

__version__ = "0.1.0"
__author__ = "Test Author"

from .core import process_data, DataProcessor
from .utils import slugify, parse_config

__all__ = ["process_data", "DataProcessor", "slugify", "parse_config"]
"#,
    )
    .unwrap();

    fs::write(
        project_dir.join("example/core.py"),
        r#"
"""Core functionality for the example package."""

import json
from typing import Dict, List, Optional
from dataclasses import dataclass


@dataclass
class DataProcessor:
    """Process data with various transformations."""
    
    name: str
    config: Dict[str, str]
    
    def process(self, data: List[Dict]) -> List[Dict]:
        """Process a list of data items."""
        processed = []
        
        for item in data:
            processed_item = self._transform_item(item)
            if self._validate_item(processed_item):
                processed.append(processed_item)
                
        return processed
    
    def _transform_item(self, item: Dict) -> Dict:
        """Transform a single data item."""
        transformed = item.copy()
        
        # Apply transformations based on config
        if "lowercase_names" in self.config:
            if "name" in transformed:
                transformed["name"] = transformed["name"].lower()
                
        if "add_timestamp" in self.config:
            import time
            transformed["processed_at"] = time.time()
            
        return transformed
    
    def _validate_item(self, item: Dict) -> bool:
        """Validate a processed item."""
        required_fields = self.config.get("required_fields", "").split(",")
        
        for field in required_fields:
            field = field.strip()
            if field and field not in item:
                return False
                
        return True


def process_data(data: List[Dict], processor_name: str = "default") -> List[Dict]:
    """Process data using the default processor."""
    config = {
        "lowercase_names": "true",
        "add_timestamp": "true",
        "required_fields": "name,id"
    }
    
    processor = DataProcessor(processor_name, config)
    return processor.process(data)


def load_config(config_path: str) -> Dict:
    """Load configuration from a file."""
    try:
        with open(config_path, "r") as f:
            return json.load(f)
    except FileNotFoundError:
        return {}
"#,
    )
    .unwrap();

    fs::write(
        project_dir.join("example/utils.py"),
        r#"
"""Utility functions for the example package."""

import re
from typing import Dict, Any


def slugify(text: str) -> str:
    """Convert text to URL-friendly slug."""
    # Convert to lowercase and replace spaces/special chars with hyphens
    slug = re.sub(r'[^\w\s-]', '', text.lower())
    slug = re.sub(r'[-\s]+', '-', slug)
    return slug.strip('-')


def parse_config(config_str: str) -> Dict[str, Any]:
    """Parse configuration string into dictionary."""
    config = {}
    
    for line in config_str.strip().split('\n'):
        line = line.strip()
        if '=' in line and not line.startswith('#'):
            key, value = line.split('=', 1)
            key = key.strip()
            value = value.strip()
            
            # Try to convert to appropriate type
            if value.lower() in ('true', 'false'):
                config[key] = value.lower() == 'true'
            elif value.isdigit():
                config[key] = int(value)
            else:
                config[key] = value
                
    return config


def format_response(data: Any, status: str = "success") -> Dict:
    """Format API response in standard format."""
    return {
        "status": status,
        "data": data,
        "timestamp": __import__("time").time()
    }
"#,
    )
    .unwrap();

    fs::write(
        project_dir.join("example/cli.py"),
        r#"
"""Command-line interface for the example package."""

import click
import json
from .core import process_data, load_config
from .utils import slugify, parse_config


@click.group()
@click.version_option()
def main():
    """Example CLI application."""
    pass


@main.command()
@click.argument('input_file', type=click.File('r'))
@click.option('--output', '-o', type=click.File('w'), default='-')
@click.option('--processor', '-p', default='default', help='Processor name')
def process(input_file, output, processor):
    """Process data from input file."""
    try:
        data = json.load(input_file)
        processed = process_data(data, processor)
        json.dump(processed, output, indent=2)
        click.echo(f"Processed {len(processed)} items", err=True)
    except Exception as e:
        click.echo(f"Error: {e}", err=True)
        raise click.Abort()


@main.command()
@click.argument('text')
def slug(text):
    """Convert text to URL slug."""
    click.echo(slugify(text))


if __name__ == '__main__':
    main()
"#,
    )
    .unwrap();

    // Create tests
    fs::create_dir_all(project_dir.join("tests")).unwrap();
    fs::write(
        project_dir.join("tests/test_core.py"),
        r#"
"""Tests for core functionality."""

import pytest
from example.core import DataProcessor, process_data


def test_data_processor_creation():
    """Test DataProcessor creation."""
    processor = DataProcessor("test", {"setting": "value"})
    assert processor.name == "test"
    assert processor.config["setting"] == "value"


def test_process_data():
    """Test process_data function."""
    data = [
        {"id": 1, "name": "Test Item"},
        {"id": 2, "name": "Another Item"}
    ]
    
    result = process_data(data)
    assert len(result) == 2
    assert result[0]["name"] == "test item"  # Should be lowercase
    assert "processed_at" in result[0]


def test_validation():
    """Test item validation."""
    processor = DataProcessor("test", {"required_fields": "id,name"})
    
    valid_item = {"id": 1, "name": "Test"}
    invalid_item = {"id": 1}  # Missing name
    
    assert processor._validate_item(valid_item) is True
    assert processor._validate_item(invalid_item) is False
"#,
    )
    .unwrap();

    fs::write(
        project_dir.join("tests/test_utils.py"),
        r#"
"""Tests for utility functions."""

import pytest
from example.utils import slugify, parse_config, format_response


def test_slugify():
    """Test slugify function."""
    assert slugify("Hello World!") == "hello-world"
    assert slugify("This is a Test") == "this-is-a-test"
    assert slugify("NoSpaces") == "nospaces"


def test_parse_config():
    """Test parse_config function."""
    config_str = """
    name=Test App
    debug=true
    port=8080
    # This is a comment
    """
    
    config = parse_config(config_str)
    assert config["name"] == "Test App"
    assert config["debug"] is True
    assert config["port"] == 8080


def test_format_response():
    """Test format_response function."""
    response = format_response({"key": "value"})
    assert response["status"] == "success"
    assert response["data"]["key"] == "value"
    assert "timestamp" in response
"#,
    )
    .unwrap();

    fs::write(
        project_dir.join("README.md"),
        r#"
# Example Python Project

A Python package for testing code-digest functionality.

## Installation

```bash
pip install -e .
```

## Usage

```python
from example import process_data, slugify

data = [{"id": 1, "name": "Test"}]
processed = process_data(data)
print(processed)
```

## CLI

```bash
example process data.json
example slug "Hello World"
```
"#,
    )
    .unwrap();

    project_dir
}

/// Test basic end-to-end markdown generation
#[test]
fn test_e2e_basic_markdown_generation() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = create_realistic_rust_project(temp_dir.path());
    let output_file = temp_dir.path().join("output.md");

    let mut cmd = Command::cargo_bin("code-digest").unwrap();
    cmd.arg("-d").arg(&project_dir).arg("-o").arg(&output_file).arg("--progress");

    cmd.assert()
        .success()
        .stderr(predicate::str::contains("Scanning directory"))
        .stderr(predicate::str::contains("Found"))
        .stderr(predicate::str::contains("files"))
        .stdout(predicate::str::contains("Written to"));

    // Verify output file was created and has expected structure
    assert!(output_file.exists());
    let content = fs::read_to_string(&output_file).unwrap();

    // Should contain basic structure
    assert!(content.contains("# Code Digest"));
    assert!(content.contains("## Statistics"));
    assert!(content.contains("## File Structure"));
    assert!(content.contains("## Table of Contents"));

    // Should contain specific files from our realistic project
    assert!(content.contains("Cargo.toml"));
    assert!(content.contains("src/main.rs"));
    assert!(content.contains("src/lib.rs"));
    assert!(content.contains("src/handlers/auth.rs"));
    assert!(content.contains("README.md"));

    // Should contain actual code content
    assert!(content.contains("tokio::main"));
    assert!(
        content.contains("Database") || content.contains("Config") || content.contains("serde")
    );
    assert!(content.contains("Deserialize") || content.contains("Serialize"));

    // Check that it's properly formatted markdown
    assert!(content.contains("```rust"));
    assert!(content.contains("```toml"));
    assert!(content.contains("```markdown"));
}

/// Test end-to-end with token limits and prioritization
#[test]
fn test_e2e_with_token_limits() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = create_realistic_rust_project(temp_dir.path());
    let output_file = temp_dir.path().join("limited_output.md");

    let mut cmd = Command::cargo_bin("code-digest").unwrap();
    cmd.arg("-d")
        .arg(&project_dir)
        .arg("-o")
        .arg(&output_file)
        .arg("--max-tokens")
        .arg("5000")  // Small limit to force prioritization
        .arg("--verbose");

    cmd.assert()
        .success()
        .stderr(predicate::str::contains("Token limit"))
        .stderr(predicate::str::contains("Selected"))
        .stderr(predicate::str::contains("Structure overhead"));

    assert!(output_file.exists());
    let content = fs::read_to_string(&output_file).unwrap();

    // Should still have basic structure but fewer files
    assert!(content.contains("# Code Digest"));
    assert!(content.contains("## Statistics"));

    // Should prioritize important files (main.rs, lib.rs, Cargo.toml)
    assert!(content.contains("src/main.rs") || content.contains("Cargo.toml"));
}

/// Test end-to-end with configuration file
#[test]
fn test_e2e_with_config_file() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = create_realistic_rust_project(temp_dir.path());

    // Create a configuration file
    let config_content = r#"
ignore = ["tests/*.rs", "*.log"]

[defaults]
max_tokens = 15000
progress = true
verbose = true

[[priorities]]
pattern = "src/main.rs"
weight = 200.0

[[priorities]]
pattern = "*.toml"
weight = 150.0
"#;

    let config_file = temp_dir.path().join("digest-config.toml");
    fs::write(&config_file, config_content).unwrap();

    let output_file = temp_dir.path().join("config_output.md");

    let mut cmd = Command::cargo_bin("code-digest").unwrap();
    cmd.arg("-d").arg(&project_dir).arg("-o").arg(&output_file).arg("-c").arg(&config_file);

    cmd.assert().success().stderr(predicate::str::contains("Loaded configuration"));

    assert!(output_file.exists());
    let content = fs::read_to_string(&output_file).unwrap();

    // Should contain prioritized files
    assert!(content.contains("src/main.rs"));
    assert!(content.contains("Cargo.toml"));

    // Should exclude ignored test files (if token limit allows, they should be deprioritized)
    // Note: This is harder to test directly since it depends on token limits and file sizes
}

/// Test end-to-end with multi-language project
#[test]
fn test_e2e_multi_language_project() {
    let temp_dir = TempDir::new().unwrap();
    let rust_dir = create_realistic_rust_project(temp_dir.path());
    let python_dir = create_python_project(temp_dir.path());

    // Create a mixed project by copying Python files into Rust project
    let mixed_dir = temp_dir.path().join("mixed_project");
    fs::create_dir_all(&mixed_dir).unwrap();

    // Helper function to copy directories recursively
    fn copy_dir(src: &Path, dst: &Path) {
        if src.is_dir() {
            fs::create_dir_all(dst).unwrap();
            for entry in fs::read_dir(src).unwrap() {
                let entry = entry.unwrap();
                let src_path = entry.path();
                let dst_path = dst.join(entry.file_name());
                copy_dir(&src_path, &dst_path);
            }
        } else {
            if let Some(parent) = dst.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::copy(src, dst).unwrap();
        }
    }

    copy_dir(&rust_dir, &mixed_dir);

    // Add Python files
    let python_target = mixed_dir.join("python");
    copy_dir(&python_dir, &python_target);

    let output_file = temp_dir.path().join("mixed_output.md");

    let mut cmd = Command::cargo_bin("code-digest").unwrap();
    cmd.arg("-d").arg(&mixed_dir).arg("-o").arg(&output_file).arg("--progress");

    cmd.assert()
        .success()
        .stderr(predicate::str::contains("Scanning directory"))
        .stderr(predicate::str::contains("Found"));

    assert!(output_file.exists());
    let content = fs::read_to_string(&output_file).unwrap();

    // Should contain both languages
    assert!(content.contains("```rust"));
    assert!(content.contains("```python"));
    assert!(content.contains("```toml"));

    // Should contain files from both projects
    assert!(content.contains("Cargo.toml"));
    assert!(content.contains("setup.py"));
    assert!(content.contains("tokio::main"));
    assert!(content.contains("def process_data"));
}

/// Test end-to-end error handling scenarios
#[test]
fn test_e2e_error_handling() {
    // Test with non-existent directory
    let mut cmd = Command::cargo_bin("code-digest").unwrap();
    cmd.arg("-d").arg("/nonexistent/directory/path");

    cmd.assert().failure().stderr(predicate::str::contains("Directory does not exist"));

    // Test with invalid output directory
    let temp_dir = TempDir::new().unwrap();
    let project_dir = create_realistic_rust_project(temp_dir.path());

    let mut cmd = Command::cargo_bin("code-digest").unwrap();
    cmd.arg("-d").arg(&project_dir).arg("-o").arg("/nonexistent/path/output.md");

    cmd.assert().failure().stderr(predicate::str::contains("Output directory does not exist"));
}

/// Test end-to-end with large project (performance test)
#[test]
fn test_e2e_large_project_performance() {
    let temp_dir = TempDir::new().unwrap();
    let large_project = temp_dir.path().join("large_project");
    fs::create_dir_all(&large_project).unwrap();

    // Create many files to test performance
    fs::create_dir_all(large_project.join("src")).unwrap();
    for i in 0..50 {
        let content = format!(
            r#"
// Module {}
use std::collections::HashMap;

pub struct Module{} {{
    data: HashMap<String, String>,
}}

impl Module{} {{
    pub fn new() -> Self {{
        Self {{
            data: HashMap::new(),
        }}
    }}
    
    pub fn process(&self, input: &str) -> String {{
        format!("Processed: {{}}", input)
    }}
}}

#[cfg(test)]
mod tests {{
    use super::*;
    
    #[test]
    fn test_module{}() {{
        let module = Module{}::new();
        assert_eq!(module.process("test"), "Processed: test");
    }}
}}
"#,
            i, i, i, i, i
        );

        fs::write(large_project.join(format!("src/module_{}.rs", i)), content).unwrap();
    }

    // Create a Cargo.toml for the large project
    fs::write(
        large_project.join("Cargo.toml"),
        r#"
[package]
name = "large-project"
version = "0.1.0"
edition = "2021"
"#,
    )
    .unwrap();

    let output_file = temp_dir.path().join("large_output.md");

    let mut cmd = Command::cargo_bin("code-digest").unwrap();
    cmd.arg("-d")
        .arg(&large_project)
        .arg("-o")
        .arg(&output_file)
        .arg("--max-tokens")
        .arg("100000")
        .arg("--progress");

    cmd.assert()
        .success()
        .stderr(predicate::str::contains("Scanning directory"))
        .stderr(predicate::str::contains("Found"));

    assert!(output_file.exists());
    let content = fs::read_to_string(&output_file).unwrap();

    // Should have processed the large project efficiently
    assert!(content.contains("# Code Digest"));
    assert!(content.contains("## Statistics"));

    // Should contain some of the generated modules
    assert!(content.contains("module_") || content.contains("Module"));
}

/// Test end-to-end stdout output (no file output)
#[test]
fn test_e2e_stdout_output() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = create_realistic_rust_project(temp_dir.path());

    let mut cmd = Command::cargo_bin("code-digest").unwrap();
    cmd.arg("-d").arg(&project_dir).arg("--max-tokens").arg("10000").arg("--quiet"); // Suppress progress output to avoid contaminating stdout

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("# Code Digest"))
        .stdout(predicate::str::contains("src/main.rs"))
        .stdout(predicate::str::contains("```rust"));
}

/// Test end-to-end with digestignore file
#[test]
fn test_e2e_with_digestignore() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = create_realistic_rust_project(temp_dir.path());

    // Create .digestignore file
    fs::write(
        project_dir.join(".digestignore"),
        r#"
tests/
*.log
target/
.git/
src/handlers/
"#,
    )
    .unwrap();

    let output_file = temp_dir.path().join("ignored_output.md");

    let mut cmd = Command::cargo_bin("code-digest").unwrap();
    cmd.arg("-d").arg(&project_dir).arg("-o").arg(&output_file).arg("--verbose");

    cmd.assert().success();

    assert!(output_file.exists());
    let content = fs::read_to_string(&output_file).unwrap();

    // Should contain main files
    assert!(content.contains("src/main.rs"));
    assert!(content.contains("src/lib.rs"));
    assert!(content.contains("Cargo.toml"));

    // Should NOT contain ignored files/directories
    assert!(!content.contains("tests/integration_test.rs"));
    assert!(!content.contains("src/handlers/auth.rs"));
}

/// Test end-to-end verbose output for debugging
#[test]
fn test_e2e_verbose_debugging() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = create_realistic_rust_project(temp_dir.path());
    let output_file = temp_dir.path().join("verbose_output.md");

    let mut cmd = Command::cargo_bin("code-digest").unwrap();
    cmd.arg("-d").arg(&project_dir).arg("-o").arg(&output_file).arg("--verbose").arg("--progress");

    cmd.assert()
        .success()
        .stderr(predicate::str::contains("Starting code-digest"))
        .stderr(predicate::str::contains("Directory:"))
        .stderr(predicate::str::contains("Creating directory walker"))
        .stderr(predicate::str::contains("Creating markdown digest"))
        .stderr(predicate::str::contains("File list:"))
        .stderr(predicate::str::contains("Scanning directory"))
        .stderr(predicate::str::contains("Generating markdown"));

    assert!(output_file.exists());
}

/// Test end-to-end with different LLM tool options (without actually calling them)
#[test]
fn test_e2e_llm_tool_selection() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = create_realistic_rust_project(temp_dir.path());
    let output_file = temp_dir.path().join("tool_output.md");

    // Test with gemini-cli
    let mut cmd = Command::cargo_bin("code-digest").unwrap();
    cmd.arg("-d")
        .arg(&project_dir)
        .arg("-o")
        .arg(&output_file)
        .arg("--tool")
        .arg("gemini-cli")
        .arg("--verbose");

    cmd.assert().success().stderr(predicate::str::contains("LLM tool: gemini-cli"));

    // Test with codex
    let mut cmd = Command::cargo_bin("code-digest").unwrap();
    cmd.arg("-d")
        .arg(&project_dir)
        .arg("-o")
        .arg(&output_file)
        .arg("--tool")
        .arg("codex")
        .arg("--verbose");

    cmd.assert().success().stderr(predicate::str::contains("LLM tool: codex"));
}
