#![cfg(test)]

//! End-to-End tests for context-creator
//!
//! These tests verify complete user workflows from CLI invocation to final output,
//! testing real-world scenarios and edge cases that users might encounter.

use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

/// Helper function to check if content contains a path, handling both Unix and Windows separators
fn contains_path(content: &str, path: &str) -> bool {
    let unix_path = path.replace('\\', "/");
    let windows_path = path.replace('/', "\\");
    content.contains(&unix_path) || content.contains(&windows_path)
}

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

This is an example Rust project for testing context-creator functionality.

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
"""Example Python package for testing context-creator."""

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

A Python package for testing context-creator functionality.

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

    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg(&project_dir)
        .arg("-o")
        .arg(&output_file)
        .arg("--progress");

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
    assert!(content.contains("# Code Context"));
    assert!(content.contains("## Statistics"));
    assert!(content.contains("## File Structure"));
    assert!(content.contains("## Table of Contents"));

    // Should contain specific files from our realistic project
    assert!(content.contains("Cargo.toml"));
    assert!(contains_path(&content, "src/main.rs"));
    assert!(contains_path(&content, "src/lib.rs"));
    assert!(contains_path(&content, "src/handlers/auth.rs"));
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

    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg(&project_dir)
        .arg("-o")
        .arg(&output_file)
        .arg("--max-tokens")
        .arg("5000") // Small limit to force prioritization
        .arg("--verbose");

    cmd.assert()
        .success()
        .stderr(predicate::str::contains("Token limit"))
        .stderr(predicate::str::contains("Selected"))
        .stderr(predicate::str::contains("Structure overhead"));

    assert!(output_file.exists());
    let content = fs::read_to_string(&output_file).unwrap();

    // Should still have basic structure but fewer files
    assert!(content.contains("# Code Context"));
    assert!(content.contains("## Statistics"));

    // Should prioritize important files (main.rs, lib.rs, Cargo.toml)
    assert!(contains_path(&content, "src/main.rs") || content.contains("Cargo.toml"));
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

    let config_file = temp_dir.path().join("context-config.toml");
    fs::write(&config_file, config_content).unwrap();

    let output_file = temp_dir.path().join("config_output.md");

    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg(&project_dir)
        .arg("-o")
        .arg(&output_file)
        .arg("-c")
        .arg(&config_file);

    cmd.assert()
        .success()
        .stderr(predicate::str::contains("Loaded configuration"));

    assert!(output_file.exists());
    let content = fs::read_to_string(&output_file).unwrap();

    // Should contain prioritized files
    assert!(contains_path(&content, "src/main.rs"));
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

    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg(&mixed_dir)
        .arg("-o")
        .arg(&output_file)
        .arg("--progress");

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
    // Test with non-existent path
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg("/nonexistent/directory/path");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Path does not exist"));

    // Test with invalid output directory
    let temp_dir = TempDir::new().unwrap();
    let project_dir = create_realistic_rust_project(temp_dir.path());

    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg(&project_dir)
        .arg("-o")
        .arg("/nonexistent/path/output.md");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Output directory does not exist"));
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
// Module {i}
use std::collections::HashMap;

pub struct Module{i} {{
    data: HashMap<String, String>,
}}

impl Module{i} {{
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
    fn test_module{i}() {{
        let module = Module{i}::new();
        assert_eq!(module.process("test"), "Processed: test");
    }}
}}
"#
        );

        fs::write(large_project.join(format!("src/module_{i}.rs")), content).unwrap();
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

    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg(&large_project)
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
    assert!(content.contains("# Code Context"));
    assert!(content.contains("## Statistics"));

    // Should contain some of the generated modules
    assert!(content.contains("module_") || content.contains("Module"));
}

/// Test end-to-end stdout output (no file output)
#[test]
fn test_e2e_stdout_output() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = create_realistic_rust_project(temp_dir.path());

    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg(&project_dir)
        .arg("--max-tokens")
        .arg("10000")
        .arg("--quiet"); // Suppress progress output to avoid contaminating stdout

    let output = cmd.assert().success().get_output().stdout.clone();
    let output_str = String::from_utf8(output).unwrap();

    assert!(output_str.contains("# Code Context"));
    assert!(contains_path(&output_str, "src/main.rs"));
    assert!(output_str.contains("```rust"));
}

/// Test end-to-end with contextignore file
#[test]
fn test_e2e_with_contextignore() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = create_realistic_rust_project(temp_dir.path());

    // Create .context-creator-ignore file
    fs::write(
        project_dir.join(".context-creator-ignore"),
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

    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg(&project_dir)
        .arg("-o")
        .arg(&output_file)
        .arg("--verbose");

    cmd.assert().success();

    assert!(output_file.exists());
    let content = fs::read_to_string(&output_file).unwrap();

    // Should contain main files
    assert!(contains_path(&content, "src/main.rs"));
    assert!(contains_path(&content, "src/lib.rs"));
    assert!(content.contains("Cargo.toml"));

    // Should NOT contain ignored files/directories
    assert!(!contains_path(&content, "tests/integration_test.rs"));
    assert!(!contains_path(&content, "src/handlers/auth.rs"));
}

/// Test end-to-end verbose output for debugging
#[test]
fn test_e2e_verbose_debugging() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = create_realistic_rust_project(temp_dir.path());
    let output_file = temp_dir.path().join("verbose_output.md");

    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg(&project_dir)
        .arg("-o")
        .arg(&output_file)
        .arg("--verbose")
        .arg("--progress");

    cmd.assert()
        .success()
        .stderr(predicate::str::contains("Starting context-creator"))
        .stderr(predicate::str::contains("Directories:"))
        .stderr(predicate::str::contains("Creating directory walker"))
        .stderr(predicate::str::contains(
            "Creating context generation options",
        ))
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

    // Test with gemini
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg(&project_dir)
        .arg("-o")
        .arg(&output_file)
        .arg("--tool")
        .arg("gemini")
        .arg("--verbose");

    cmd.assert()
        .success()
        .stderr(predicate::str::contains("LLM tool: gemini"));

    // Test with codex
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg(&project_dir)
        .arg("-o")
        .arg(&output_file)
        .arg("--tool")
        .arg("codex")
        .arg("--verbose");

    cmd.assert()
        .success()
        .stderr(predicate::str::contains("LLM tool: codex"));
}

/// Test end-to-end with multiple directories
#[test]
fn test_e2e_multiple_directories() {
    let temp_dir = TempDir::new().unwrap();

    // Create separate Rust and Python projects
    let rust_dir = create_realistic_rust_project(temp_dir.path());
    let python_dir = create_python_project(temp_dir.path());

    let output_file = temp_dir.path().join("multiple_dirs_output.md");

    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg(&rust_dir)
        .arg(&python_dir)
        .arg("-o")
        .arg(&output_file)
        .arg("--progress");

    cmd.assert()
        .success()
        .stderr(predicate::str::contains("Processing directory 1 of 2"))
        .stderr(predicate::str::contains("Processing directory 2 of 2"));

    assert!(output_file.exists());
    let content = fs::read_to_string(&output_file).unwrap();

    // Should contain header indicating multiple directories
    assert!(content.contains("# Code Context - Multiple Directories"));

    // Should contain separate sections for each directory
    assert!(content.contains(&format!("## Directory: {}", rust_dir.display())));
    assert!(content.contains(&format!("## Directory: {}", python_dir.display())));

    // Should contain content from both projects
    assert!(content.contains("Cargo.toml"));
    assert!(content.contains("setup.py"));
    assert!(content.contains("```rust"));
    assert!(content.contains("```python"));
    assert!(content.contains("tokio::main"));
    assert!(content.contains("def process_data"));
}

/// Real-code regression: Rust import tracing must include module declaration files.
#[test]
fn test_e2e_trace_imports_includes_rust_module_roots() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    fs::write(
        project_dir.join("Cargo.toml"),
        r#"
[package]
name = "nested-modules"
version = "0.1.0"
edition = "2021"
"#,
    )
    .unwrap();

    fs::create_dir_all(project_dir.join("src/core")).unwrap();
    fs::create_dir_all(project_dir.join("src/utils")).unwrap();

    fs::write(
        project_dir.join("src/lib.rs"),
        "pub mod core;\npub mod utils;\n",
    )
    .unwrap();
    fs::write(
        project_dir.join("src/core/mod.rs"),
        "pub mod engine;\npub mod config;\n",
    )
    .unwrap();
    fs::write(
        project_dir.join("src/core/engine.rs"),
        r#"
use crate::utils::helpers::format_output;

pub fn run() -> String {
    format_output("Engine running")
}
"#,
    )
    .unwrap();
    fs::write(
        project_dir.join("src/core/config.rs"),
        "pub struct Config { pub debug: bool }\n",
    )
    .unwrap();
    fs::write(project_dir.join("src/utils/mod.rs"), "pub mod helpers;\n").unwrap();
    fs::write(
        project_dir.join("src/utils/helpers.rs"),
        r#"
pub fn format_output(message: &str) -> String {
    format!("[service] {message}")
}
"#,
    )
    .unwrap();
    fs::write(
        project_dir.join("src/main.rs"),
        r#"
use nested_modules::core::engine;

fn main() {
    println!("{}", engine::run());
}
"#,
    )
    .unwrap();

    let output_file = temp_dir.path().join("trace_imports.md");
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.current_dir(project_dir)
        .arg("--include")
        .arg("src/main.rs")
        .arg("--trace-imports")
        .arg("--output-file")
        .arg(&output_file);

    cmd.assert().success();
    let content = fs::read_to_string(&output_file).unwrap();

    assert!(contains_path(&content, "src/main.rs"));
    assert!(contains_path(&content, "src/lib.rs"));
    assert!(contains_path(&content, "src/core/mod.rs"));
    assert!(contains_path(&content, "src/core/engine.rs"));
    assert!(contains_path(&content, "src/utils/mod.rs"));
    assert!(contains_path(&content, "src/utils/helpers.rs"));
    assert!(!contains_path(&content, "src/core/config.rs"));
}

/// Real-code regression: Rust type expansion must include crate-root module declarations.
#[test]
fn test_e2e_include_types_includes_rust_crate_root() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    fs::write(
        project_dir.join("Cargo.toml"),
        r#"
[package]
name = "typed-api"
version = "0.1.0"
edition = "2021"
"#,
    )
    .unwrap();

    fs::create_dir_all(project_dir.join("src")).unwrap();
    fs::write(
        project_dir.join("src/lib.rs"),
        "pub mod core_types;\npub mod domain;\npub mod handlers;\n",
    )
    .unwrap();
    fs::write(
        project_dir.join("src/core_types.rs"),
        r#"
#[derive(Debug, Clone)]
pub struct Timestamp {
    pub seconds: i64,
}

#[derive(Debug)]
pub struct Metadata {
    pub created: Timestamp,
}
"#,
    )
    .unwrap();
    fs::write(
        project_dir.join("src/domain.rs"),
        r#"
use crate::core_types::Metadata;

#[derive(Debug)]
pub struct User {
    pub id: u64,
    pub metadata: Metadata,
}

#[derive(Debug)]
pub struct Session {
    pub token: String,
    pub user: User,
}
"#,
    )
    .unwrap();
    fs::write(
        project_dir.join("src/handlers.rs"),
        r#"
use crate::domain::Session;

pub fn validate_session(session: &Session) -> bool {
    !session.token.is_empty() && session.user.id > 0
}
"#,
    )
    .unwrap();

    let output_file = temp_dir.path().join("include_types.md");
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.current_dir(project_dir)
        .arg("--include")
        .arg("src/handlers.rs")
        .arg("--include-types")
        .arg("--output-file")
        .arg(&output_file);

    cmd.assert().success();
    let content = fs::read_to_string(&output_file).unwrap();

    assert!(contains_path(&content, "src/handlers.rs"));
    assert!(contains_path(&content, "src/domain.rs"));
    assert!(contains_path(&content, "src/core_types.rs"));
    assert!(contains_path(&content, "src/lib.rs"));
    assert!(content.contains("pub struct Session"));
    assert!(content.contains("pub struct User"));
    assert!(content.contains("pub struct Metadata"));
    assert!(content.contains("pub struct Timestamp"));
}

/// Real-code regression: Go import tracing follows local packages in a go.mod module.
#[test]
fn test_e2e_go_trace_imports_includes_local_packages() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    fs::write(
        project_dir.join("go.mod"),
        "module example.com/acme/shop\n\ngo 1.22\n",
    )
    .unwrap();

    fs::create_dir_all(project_dir.join("cmd/checkout")).unwrap();
    fs::create_dir_all(project_dir.join("internal/app")).unwrap();
    fs::create_dir_all(project_dir.join("internal/config")).unwrap();
    fs::create_dir_all(project_dir.join("internal/unused")).unwrap();

    fs::write(
        project_dir.join("cmd/checkout/main.go"),
        r#"
package main

import "example.com/acme/shop/internal/app"

func main() {
    app.Run()
}
"#,
    )
    .unwrap();
    fs::write(
        project_dir.join("internal/app/app.go"),
        r#"
package app

import "example.com/acme/shop/internal/config"

func Run() config.Config {
    return config.Load()
}
"#,
    )
    .unwrap();
    fs::write(
        project_dir.join("internal/config/config.go"),
        r#"
package config

type Config struct {
    Port int
}

func Load() Config {
    return Config{Port: 8080}
}
"#,
    )
    .unwrap();
    fs::write(
        project_dir.join("internal/unused/unused.go"),
        "package unused\n\nfunc NeverCalled() {}\n",
    )
    .unwrap();

    let output_file = temp_dir.path().join("go_trace_imports.md");
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.current_dir(project_dir)
        .arg("--include")
        .arg("cmd/checkout/main.go")
        .arg("--trace-imports")
        .arg("--output-file")
        .arg(&output_file);

    cmd.assert().success();
    let content = fs::read_to_string(&output_file).unwrap();

    assert!(contains_path(&content, "cmd/checkout/main.go"));
    assert!(contains_path(&content, "internal/app/app.go"));
    assert!(contains_path(&content, "internal/config/config.go"));
    assert!(!contains_path(&content, "internal/unused/unused.go"));
    assert!(content.contains("func Run() config.Config"));
    assert!(content.contains("func Load() Config"));
}

/// Real-code regression: Go type expansion resolves package-qualified types.
#[test]
fn test_e2e_go_include_types_includes_package_type_definitions() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    fs::write(
        project_dir.join("go.mod"),
        "module example.com/acme/shop\n\ngo 1.22\n",
    )
    .unwrap();

    fs::create_dir_all(project_dir.join("handlers")).unwrap();
    fs::create_dir_all(project_dir.join("domain")).unwrap();

    fs::write(
        project_dir.join("handlers/handler.go"),
        r#"
package handlers

import "example.com/acme/shop/domain"

type Handler struct{}

func (Handler) Build(user domain.User) domain.Invoice {
    return domain.Invoice{User: user}
}
"#,
    )
    .unwrap();
    fs::write(
        project_dir.join("domain/user.go"),
        r#"
package domain

type User struct {
    ID      string
    Profile Profile
}

type Profile struct {
    Email string
}

type Invoice struct {
    User  User
    Lines []Line
}

type Line struct {
    SKU string
}
"#,
    )
    .unwrap();
    fs::write(
        project_dir.join("domain/unused.go"),
        "package domain\n\ntype Unused struct { Name string }\n",
    )
    .unwrap();

    let output_file = temp_dir.path().join("go_include_types.md");
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.current_dir(project_dir)
        .arg("--include")
        .arg("handlers/handler.go")
        .arg("--include-types")
        .arg("--output-file")
        .arg(&output_file);

    cmd.assert().success();
    let content = fs::read_to_string(&output_file).unwrap();

    assert!(contains_path(&content, "handlers/handler.go"));
    assert!(contains_path(&content, "domain/user.go"));
    assert!(!contains_path(&content, "domain/unused.go"));
    assert!(content.contains("type User struct"));
    assert!(content.contains("type Profile struct"));
    assert!(content.contains("type Invoice struct"));
    assert!(content.contains("type Line struct"));
}

/// Real-code regression: Go caller expansion finds package functions and methods.
#[test]
fn test_e2e_go_include_callers_finds_function_and_method_callers() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    fs::write(
        project_dir.join("go.mod"),
        "module example.com/acme/shop\n\ngo 1.22\n",
    )
    .unwrap();

    fs::create_dir_all(project_dir.join("billing")).unwrap();
    fs::create_dir_all(project_dir.join("cmd/checkout")).unwrap();
    fs::create_dir_all(project_dir.join("api")).unwrap();
    fs::create_dir_all(project_dir.join("internal/unrelated")).unwrap();

    fs::write(
        project_dir.join("billing/calculator.go"),
        r#"
package billing

type Item struct {
    Price int
}

type Money struct {
    Cents int
}

type Service struct{}

func CalculateTotal(items []Item) Money {
    total := 0
    for _, item := range items {
        total += item.Price
    }
    return Money{Cents: total}
}

func (Service) Process(items []Item) Money {
    return CalculateTotal(items)
}
"#,
    )
    .unwrap();
    fs::write(
        project_dir.join("cmd/checkout/main.go"),
        r#"
package main

import "example.com/acme/shop/billing"

func main() {
    total := billing.CalculateTotal(nil)
    service := billing.Service{}
    _ = service.Process(nil)
    _ = total
}
"#,
    )
    .unwrap();
    fs::write(
        project_dir.join("api/quote.go"),
        r#"
package api

import "example.com/acme/shop/billing"

func Quote() billing.Money {
    return billing.CalculateTotal([]billing.Item{{Price: 25}})
}
"#,
    )
    .unwrap();
    fs::write(
        project_dir.join("internal/unrelated/unrelated.go"),
        r#"
package unrelated

func Ignore() string {
    return "not a caller"
}
"#,
    )
    .unwrap();

    let output_file = temp_dir.path().join("go_include_callers.md");
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.current_dir(project_dir)
        .arg("--include")
        .arg("billing/calculator.go")
        .arg("--include-callers")
        .arg("--output-file")
        .arg(&output_file);

    cmd.assert().success();
    let content = fs::read_to_string(&output_file).unwrap();

    assert!(contains_path(&content, "billing/calculator.go"));
    assert!(contains_path(&content, "cmd/checkout/main.go"));
    assert!(contains_path(&content, "api/quote.go"));
    assert!(!contains_path(&content, "internal/unrelated/unrelated.go"));
    assert!(content.contains("func CalculateTotal(items []Item) Money"));
    assert!(content.contains("func Quote() billing.Money"));
}

/// Real-code regression: Go imports packages, so trace-imports must include all package files.
#[test]
fn test_e2e_go_trace_imports_includes_all_files_in_imported_packages() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    fs::write(
        project_dir.join("go.mod"),
        "module example.com/acme/shop\n\ngo 1.22\n",
    )
    .unwrap();

    fs::create_dir_all(project_dir.join("cmd/server")).unwrap();
    fs::create_dir_all(project_dir.join("internal/app")).unwrap();
    fs::create_dir_all(project_dir.join("internal/config")).unwrap();

    fs::write(
        project_dir.join("cmd/server/main.go"),
        r#"
package main

import "example.com/acme/shop/internal/app"

func main() {
    app.Run()
}
"#,
    )
    .unwrap();
    fs::write(
        project_dir.join("internal/app/app.go"),
        r#"
package app

const Name = "checkout"
"#,
    )
    .unwrap();
    fs::write(
        project_dir.join("internal/app/runner.go"),
        r#"
package app

import "example.com/acme/shop/internal/config"

func Run() config.Config {
    return config.Load(Name)
}
"#,
    )
    .unwrap();
    fs::write(
        project_dir.join("internal/app/runner_test.go"),
        "package app\n\nfunc TestRun() {}\n",
    )
    .unwrap();
    fs::write(
        project_dir.join("internal/config/config.go"),
        r#"
package config

type Config struct {
    Name string
}

func Load(name string) Config {
    return Config{Name: name}
}
"#,
    )
    .unwrap();

    let output_file = temp_dir.path().join("go_package_trace.md");
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.current_dir(project_dir)
        .arg("--include")
        .arg("cmd/server/main.go")
        .arg("--trace-imports")
        .arg("--output-file")
        .arg(&output_file);

    cmd.assert().success();
    let content = fs::read_to_string(&output_file).unwrap();

    assert!(contains_path(&content, "cmd/server/main.go"));
    assert!(contains_path(&content, "internal/app/app.go"));
    assert!(contains_path(&content, "internal/app/runner.go"));
    assert!(contains_path(&content, "internal/config/config.go"));
    assert!(!contains_path(&content, "internal/app/runner_test.go"));
    assert!(content.contains("func Run() config.Config"));
    assert!(content.contains("func Load(name string) Config"));
}

/// Real-code regression: Go import parsing covers grouped, aliased, raw, blank, and dot imports.
#[test]
fn test_e2e_go_trace_imports_handles_import_forms() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    fs::write(
        project_dir.join("go.mod"),
        "module example.com/acme/shop\n\ngo 1.22\n",
    )
    .unwrap();

    fs::create_dir_all(project_dir.join("cmd/tools")).unwrap();
    fs::create_dir_all(project_dir.join("internal/config")).unwrap();
    fs::create_dir_all(project_dir.join("internal/rawpkg")).unwrap();
    fs::create_dir_all(project_dir.join("internal/sideeffect")).unwrap();
    fs::create_dir_all(project_dir.join("internal/dot")).unwrap();

    fs::write(
        project_dir.join("cmd/tools/main.go"),
        r#"
package main

import (
    "fmt"
    cfg "example.com/acme/shop/internal/config"
    rawpkg `example.com/acme/shop/internal/rawpkg`
    _ "example.com/acme/shop/internal/sideeffect"
    . "example.com/acme/shop/internal/dot"
    "github.com/shopspring/decimal"
)

func main() {
    fmt.Println(cfg.Load().Name, rawpkg.Value, Helper())
    _ = decimal.NewFromInt(1)
}
"#,
    )
    .unwrap();
    fs::write(
        project_dir.join("internal/config/config.go"),
        "package config\n\ntype Config struct { Name string }\n\nfunc Load() Config { return Config{Name: \"ok\"} }\n",
    )
    .unwrap();
    fs::write(
        project_dir.join("internal/rawpkg/rawpkg.go"),
        "package rawpkg\n\nconst Value = \"raw\"\n",
    )
    .unwrap();
    fs::write(
        project_dir.join("internal/sideeffect/sideeffect.go"),
        "package sideeffect\n\nfunc init() {}\n",
    )
    .unwrap();
    fs::write(
        project_dir.join("internal/dot/dot.go"),
        "package dot\n\nfunc Helper() string { return \"dot\" }\n",
    )
    .unwrap();

    let output_file = temp_dir.path().join("go_import_forms.md");
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.current_dir(project_dir)
        .arg("--include")
        .arg("cmd/tools/main.go")
        .arg("--trace-imports")
        .arg("--output-file")
        .arg(&output_file);

    cmd.assert().success();
    let content = fs::read_to_string(&output_file).unwrap();

    assert!(contains_path(&content, "cmd/tools/main.go"));
    assert!(contains_path(&content, "internal/config/config.go"));
    assert!(contains_path(&content, "internal/rawpkg/rawpkg.go"));
    assert!(contains_path(&content, "internal/sideeffect/sideeffect.go"));
    assert!(contains_path(&content, "internal/dot/dot.go"));
    assert!(content.contains("func Helper() string"));
}

/// Real-code regression: Go type expansion handles pointers, slices, maps, channels, generics, and aliases.
#[test]
fn test_e2e_go_include_types_handles_complex_type_shapes_across_files() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    fs::write(
        project_dir.join("go.mod"),
        "module example.com/acme/shop\n\ngo 1.22\n",
    )
    .unwrap();

    fs::create_dir_all(project_dir.join("handlers")).unwrap();
    fs::create_dir_all(project_dir.join("domain")).unwrap();

    fs::write(
        project_dir.join("handlers/handler.go"),
        r#"
package handlers

import model "example.com/acme/shop/domain"

type Envelope[T any] struct {
    Value T
}

type Handler struct {
    Users  []*model.User
    ByID   map[string]model.Account
    Events chan model.Event
}

func (Handler) Build(input *model.User, updates []model.Account) Envelope[model.Event] {
    return Envelope[model.Event]{Value: model.Event{User: input}}
}
"#,
    )
    .unwrap();
    fs::write(
        project_dir.join("domain/user.go"),
        r#"
package domain

type User struct {
    ID      string
    Profile Profile
}
"#,
    )
    .unwrap();
    fs::write(
        project_dir.join("domain/account.go"),
        "package domain\n\ntype Account = User\n",
    )
    .unwrap();
    fs::write(
        project_dir.join("domain/event.go"),
        "package domain\n\ntype Event struct { User *User }\n",
    )
    .unwrap();
    fs::write(
        project_dir.join("domain/profile.go"),
        "package domain\n\ntype Profile struct { Email string }\n",
    )
    .unwrap();
    fs::write(
        project_dir.join("domain/unused.go"),
        "package domain\n\ntype Unused struct { Name string }\n",
    )
    .unwrap();

    let output_file = temp_dir.path().join("go_complex_types.md");
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.current_dir(project_dir)
        .arg("--include")
        .arg("handlers/handler.go")
        .arg("--include-types")
        .arg("--output-file")
        .arg(&output_file);

    cmd.assert().success();
    let content = fs::read_to_string(&output_file).unwrap();

    assert!(contains_path(&content, "handlers/handler.go"));
    assert!(contains_path(&content, "domain/user.go"));
    assert!(contains_path(&content, "domain/account.go"));
    assert!(contains_path(&content, "domain/event.go"));
    assert!(contains_path(&content, "domain/profile.go"));
    assert!(!contains_path(&content, "domain/unused.go"));
    assert!(content.contains("type Account = User"));
    assert!(content.contains("type Event struct"));
    assert!(content.contains("type Profile struct"));
}

/// Real-code regression: Go caller expansion should not match unrelated package selectors by name only.
#[test]
fn test_e2e_go_include_callers_ignores_unrelated_selector_same_name() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    fs::write(
        project_dir.join("go.mod"),
        "module example.com/acme/logger\n\ngo 1.22\n",
    )
    .unwrap();

    fs::create_dir_all(project_dir.join("printer")).unwrap();
    fs::create_dir_all(project_dir.join("cmd/app")).unwrap();
    fs::create_dir_all(project_dir.join("cmd/fmtcaller")).unwrap();

    fs::write(
        project_dir.join("printer/printer.go"),
        r#"
package printer

func Println(message string) string {
    return "[app] " + message
}
"#,
    )
    .unwrap();
    fs::write(
        project_dir.join("cmd/app/main.go"),
        r#"
package main

import "example.com/acme/logger/printer"

func main() {
    _ = printer.Println("hello")
}
"#,
    )
    .unwrap();
    fs::write(
        project_dir.join("cmd/fmtcaller/main.go"),
        r#"
package main

import "fmt"

func main() {
    fmt.Println("not the app printer")
}
"#,
    )
    .unwrap();

    let output_file = temp_dir.path().join("go_callers_no_false_positive.md");
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.current_dir(project_dir)
        .arg("--include")
        .arg("printer/printer.go")
        .arg("--include-callers")
        .arg("--output-file")
        .arg(&output_file);

    cmd.assert().success();
    let content = fs::read_to_string(&output_file).unwrap();

    assert!(contains_path(&content, "printer/printer.go"));
    assert!(contains_path(&content, "cmd/app/main.go"));
    assert!(!contains_path(&content, "cmd/fmtcaller/main.go"));
    assert!(content.contains("func Println(message string) string"));
}

/// Real-code regression: Swift import tracing follows SwiftPM target imports transitively.
#[test]
fn test_e2e_swift_trace_imports_includes_swiftpm_targets() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    fs::write(
        project_dir.join("Package.swift"),
        r#"// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "CheckoutApp",
    products: [
        .executable(name: "CheckoutApp", targets: ["App"])
    ],
    targets: [
        .executableTarget(name: "App", dependencies: ["Core"]),
        .target(name: "Core", dependencies: ["Config"]),
        .target(name: "Config"),
        .target(name: "Unused")
    ]
)
"#,
    )
    .unwrap();

    fs::create_dir_all(project_dir.join("Sources/App")).unwrap();
    fs::create_dir_all(project_dir.join("Sources/Core")).unwrap();
    fs::create_dir_all(project_dir.join("Sources/Core/Documentation.docc/Resources/code-files"))
        .unwrap();
    fs::create_dir_all(project_dir.join("Sources/Config")).unwrap();
    fs::create_dir_all(project_dir.join("Sources/Unused")).unwrap();

    fs::write(
        project_dir.join("Sources/App/main.swift"),
        r#"
import Core

let summary = CoreRunner().run()
print(summary.title)
"#,
    )
    .unwrap();
    fs::write(
        project_dir.join("Sources/Core/Core.swift"),
        r#"
import Foundation

public struct CoreRunner {
    public init() {}

    public func run() -> Summary {
        return buildSummary()
    }
}
"#,
    )
    .unwrap();
    fs::write(
        project_dir.join("Sources/Core/Builder.swift"),
        r#"
import Config

public func buildSummary() -> Summary {
    let config = AppConfig.default
    return Summary(title: config.name)
}

public struct Summary {
    public let title: String
}
"#,
    )
    .unwrap();
    fs::write(
        project_dir.join("Sources/Config/AppConfig.swift"),
        r#"
public struct AppConfig {
    public let name: String

    public static let `default` = AppConfig(name: "checkout")
}
"#,
    )
    .unwrap();
    fs::write(
        project_dir.join("Sources/Core/Documentation.docc/Resources/code-files/Tutorial.swift"),
        r#"
public struct TutorialOnlyType {
    public let title: String
}
"#,
    )
    .unwrap();
    fs::write(
        project_dir.join("Sources/Unused/Unused.swift"),
        "public struct Unused { public init() {} }\n",
    )
    .unwrap();

    let output_file = temp_dir.path().join("swift_trace_imports.md");
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.current_dir(project_dir)
        .arg("--include")
        .arg("Sources/App/main.swift")
        .arg("--trace-imports")
        .arg("--output-file")
        .arg(&output_file);

    cmd.assert().success();
    let content = fs::read_to_string(&output_file).unwrap();

    assert!(contains_path(&content, "Sources/App/main.swift"));
    assert!(contains_path(&content, "Sources/Core/Core.swift"));
    assert!(contains_path(&content, "Sources/Core/Builder.swift"));
    assert!(contains_path(&content, "Sources/Config/AppConfig.swift"));
    assert!(!contains_path(
        &content,
        "Sources/Core/Documentation.docc/Resources/code-files/Tutorial.swift"
    ));
    assert!(!contains_path(&content, "Sources/Unused/Unused.swift"));
    assert!(content.contains("public struct CoreRunner"));
    assert!(content.contains("public func buildSummary() -> Summary"));
    assert!(content.contains("public struct AppConfig"));
}

/// Real-code regression: Swift type expansion resolves imported target types and complex type shapes.
#[test]
fn test_e2e_swift_include_types_resolves_imported_swiftpm_types() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    fs::write(
        project_dir.join("Package.swift"),
        r#"// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "Billing",
    targets: [
        .target(name: "Handlers", dependencies: ["Models"]),
        .target(name: "Models")
    ]
)
"#,
    )
    .unwrap();

    fs::create_dir_all(project_dir.join("Sources/Handlers")).unwrap();
    fs::create_dir_all(project_dir.join("Sources/Models")).unwrap();

    fs::write(
        project_dir.join("Sources/Handlers/Handler.swift"),
        r#"
import Foundation
import struct Models.User
import Models

public protocol UserRepository {
    func loadUser(id: String) -> User?
}

public struct BillingHandler {
    public let repository: any UserRepository
    public let profiles: [Profile]
    public let invoicesByID: Dictionary<String, Invoice>

    public func buildInvoice(for user: User, lines: [LineItem]) -> Invoice {
        return Invoice(user: user, lines: lines)
    }
}
"#,
    )
    .unwrap();
    fs::write(
        project_dir.join("Sources/Models/User.swift"),
        r#"
public struct User {
    public let id: String
    public let profile: Profile
}

public struct Profile {
    public let email: String
}
"#,
    )
    .unwrap();
    fs::write(
        project_dir.join("Sources/Models/Invoice.swift"),
        r#"
public struct Invoice {
    public let user: User
    public let lines: [LineItem]
}

public struct LineItem {
    public let sku: String
}
"#,
    )
    .unwrap();
    fs::write(
        project_dir.join("Sources/Models/Unused.swift"),
        "public struct UnusedModel { public let id: String }\n",
    )
    .unwrap();

    let output_file = temp_dir.path().join("swift_include_types.md");
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.current_dir(project_dir)
        .arg("--include")
        .arg("Sources/Handlers/Handler.swift")
        .arg("--include-types")
        .arg("--output-file")
        .arg(&output_file);

    cmd.assert().success();
    let content = fs::read_to_string(&output_file).unwrap();

    assert!(contains_path(&content, "Sources/Handlers/Handler.swift"));
    assert!(contains_path(&content, "Sources/Models/User.swift"));
    assert!(contains_path(&content, "Sources/Models/Invoice.swift"));
    assert!(!contains_path(&content, "Sources/Models/Unused.swift"));
    assert!(content.contains("public struct User"));
    assert!(content.contains("public struct Profile"));
    assert!(content.contains("public struct Invoice"));
    assert!(content.contains("public struct LineItem"));
}

/// Real-code regression: Swift type expansion resolves protocol conformances and property wrappers
/// from imported SwiftPM targets.
#[test]
fn test_e2e_swift_include_types_resolves_protocols_and_property_wrappers() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    fs::write(
        project_dir.join("Package.swift"),
        r#"// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "Tools",
    targets: [
        .executableTarget(name: "CLI", dependencies: ["ArgumentParser"]),
        .target(name: "ArgumentParser")
    ]
)
"#,
    )
    .unwrap();

    fs::create_dir_all(project_dir.join("Sources/CLI")).unwrap();
    fs::create_dir_all(project_dir.join("Sources/ArgumentParser/Parsable Types")).unwrap();
    fs::create_dir_all(project_dir.join("Sources/ArgumentParser/Parsable Properties")).unwrap();

    fs::write(
        project_dir.join("Sources/CLI/Command.swift"),
        r#"
import ArgumentParser

@main
struct BuildTool: ParsableCommand {
    static let configuration = CommandConfiguration(
        abstract: "Build the package.")

    @Flag(help: "Enable verbose logging.")
    var verbose = false
}
"#,
    )
    .unwrap();
    fs::write(
        project_dir.join("Sources/ArgumentParser/Parsable Types/ParsableCommand.swift"),
        r#"
public protocol ParsableCommand {
    static var configuration: CommandConfiguration { get }
}
"#,
    )
    .unwrap();
    fs::write(
        project_dir.join("Sources/ArgumentParser/Parsable Types/CommandConfiguration.swift"),
        r#"
public struct CommandConfiguration {
    public let abstract: String
}
"#,
    )
    .unwrap();
    fs::write(
        project_dir.join("Sources/ArgumentParser/Parsable Properties/Flag.swift"),
        r#"
@propertyWrapper
public struct Flag {
    public var wrappedValue: Bool

    public init(help: String) {
        self.wrappedValue = false
    }
}
"#,
    )
    .unwrap();

    let output_file = temp_dir.path().join("swift_include_protocol_types.md");
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.current_dir(project_dir)
        .arg("--include")
        .arg("Sources/CLI/Command.swift")
        .arg("--include-types")
        .arg("--output-file")
        .arg(&output_file);

    cmd.assert().success();
    let content = fs::read_to_string(&output_file).unwrap();

    assert!(contains_path(&content, "Sources/CLI/Command.swift"));
    assert!(contains_path(
        &content,
        "Sources/ArgumentParser/Parsable Types/ParsableCommand.swift"
    ));
    assert!(contains_path(
        &content,
        "Sources/ArgumentParser/Parsable Types/CommandConfiguration.swift"
    ));
    assert!(contains_path(
        &content,
        "Sources/ArgumentParser/Parsable Properties/Flag.swift"
    ));
    assert!(content.contains("public protocol ParsableCommand"));
    assert!(content.contains("public struct CommandConfiguration"));
    assert!(content.contains("public struct Flag"));
}

/// Real-code regression: Swift type expansion handles nested types, generic
/// constraints, existential protocol compositions, and actors in the same target.
#[test]
fn test_e2e_swift_include_types_resolves_complex_same_target_types() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    fs::write(
        project_dir.join("Package.swift"),
        r#"// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "ComplexTypes",
    targets: [
        .target(name: "App")
    ]
)
"#,
    )
    .unwrap();

    fs::create_dir_all(project_dir.join("Sources/App")).unwrap();

    fs::write(
        project_dir.join("Sources/App/Handler.swift"),
        r#"
import Foundation

typealias ResponseList = [API.Response]

public struct Handler<Service: WorkflowService> where Service.Output == API.Response {
    public let service: Service
    public let worker: JobWorker
    public let formatter: any ResponseFormatting & Sendable

    public func handle(_ response: API.Response) -> ResponseList {
        formatter.render(response)
        return [response]
    }
}
"#,
    )
    .unwrap();
    fs::write(
        project_dir.join("Sources/App/DomainModels.swift"),
        r#"
public enum API {
    public struct Response: Sendable {
        public let status: Status
    }

    public enum Status: Sendable {
        case ok
    }
}
"#,
    )
    .unwrap();
    fs::write(
        project_dir.join("Sources/App/ConcurrencyModels.swift"),
        "public actor JobWorker { public init() {} }\n",
    )
    .unwrap();
    fs::write(
        project_dir.join("Sources/App/Protocols.swift"),
        r#"
public protocol WorkflowService {
    associatedtype Output
}

public protocol ResponseFormatting {
    func render(_ response: API.Response)
}
"#,
    )
    .unwrap();

    let output_file = temp_dir
        .path()
        .join("swift_complex_same_target_types.paths");
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.current_dir(project_dir)
        .arg("--include")
        .arg("Sources/App/Handler.swift")
        .arg("--include-types")
        .arg("--style")
        .arg("paths")
        .arg("--output-file")
        .arg(&output_file);

    cmd.assert().success();
    let content = fs::read_to_string(&output_file).unwrap();
    let paths = content.lines().collect::<Vec<_>>();

    assert!(paths.contains(&"Sources/App/Handler.swift"));
    assert!(paths.contains(&"Sources/App/DomainModels.swift"));
    assert!(paths.contains(&"Sources/App/ConcurrencyModels.swift"));
    assert!(paths.contains(&"Sources/App/Protocols.swift"));
}

/// Real-code regression: Swift caller expansion finds same-target and importing-target callers.
#[test]
fn test_e2e_swift_include_callers_finds_function_and_method_callers() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    fs::write(
        project_dir.join("Package.swift"),
        r#"// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "Checkout",
    targets: [
        .target(name: "Billing"),
        .executableTarget(name: "App", dependencies: ["Billing"]),
        .target(name: "API", dependencies: ["Billing"]),
        .target(name: "Formatting")
    ]
)
"#,
    )
    .unwrap();

    fs::create_dir_all(project_dir.join("Sources/Billing")).unwrap();
    fs::create_dir_all(project_dir.join("Sources/App")).unwrap();
    fs::create_dir_all(project_dir.join("Sources/API")).unwrap();
    fs::create_dir_all(project_dir.join("Sources/Formatting")).unwrap();

    fs::write(
        project_dir.join("Sources/Billing/Calculator.swift"),
        r#"
public struct Item {
    public let price: Int
}

public struct Money {
    public let cents: Int
}

public func calculateTotal(_ items: [Item]) -> Money {
    let total = items.reduce(0) { partial, item in partial + item.price }
    return Money(cents: total)
}

public struct BillingService {
    public init() {}

    public func process(_ items: [Item]) -> Money {
        return calculateTotal(items)
    }
}
"#,
    )
    .unwrap();
    fs::write(
        project_dir.join("Sources/App/main.swift"),
        r#"
import Billing

let total = calculateTotal([Item(price: 25)])
let service = BillingService()
_ = service.process([Item(price: total.cents)])
"#,
    )
    .unwrap();
    fs::write(
        project_dir.join("Sources/API/QuoteController.swift"),
        r#"
import Billing

public struct QuoteController {
    public func quote() -> Money {
        return calculateTotal([Item(price: 15)])
    }
}
"#,
    )
    .unwrap();
    fs::write(
        project_dir.join("Sources/Formatting/Logger.swift"),
        r#"
public func calculateTotal(_ values: [String]) -> String {
    return values.joined(separator: ",")
}

public func render() -> String {
    return calculateTotal(["not", "billing"])
}
"#,
    )
    .unwrap();

    let output_file = temp_dir.path().join("swift_include_callers.md");
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.current_dir(project_dir)
        .arg("--include")
        .arg("Sources/Billing/Calculator.swift")
        .arg("--include-callers")
        .arg("--output-file")
        .arg(&output_file);

    cmd.assert().success();
    let content = fs::read_to_string(&output_file).unwrap();

    assert!(contains_path(&content, "Sources/Billing/Calculator.swift"));
    assert!(contains_path(&content, "Sources/App/main.swift"));
    assert!(contains_path(&content, "Sources/API/QuoteController.swift"));
    assert!(!contains_path(&content, "Sources/Formatting/Logger.swift"));
    assert!(content.contains("public func calculateTotal(_ items: [Item]) -> Money"));
    assert!(content.contains("public func quote() -> Money"));
}

/// Real-code regression: Swift import parsing handles Foundation, @testable, and declaration imports.
#[test]
fn test_e2e_swift_trace_imports_handles_import_forms() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    fs::write(
        project_dir.join("Package.swift"),
        r#"// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "ImportForms",
    targets: [
        .target(name: "Feature", dependencies: ["Models", "Support"]),
        .target(name: "Models"),
        .target(name: "Support")
    ]
)
"#,
    )
    .unwrap();

    fs::create_dir_all(project_dir.join("Sources/Feature")).unwrap();
    fs::create_dir_all(project_dir.join("Sources/Models")).unwrap();
    fs::create_dir_all(project_dir.join("Sources/Support")).unwrap();

    fs::write(
        project_dir.join("Sources/Feature/Feature.swift"),
        r#"
import Foundation
import struct Models.User
@testable import Support

public func makeFeature() -> FeatureState {
    return FeatureState(user: User(id: SupportID.value))
}

public struct FeatureState {
    public let user: User
}
"#,
    )
    .unwrap();
    fs::write(
        project_dir.join("Sources/Models/User.swift"),
        "public struct User { public let id: String }\n",
    )
    .unwrap();
    fs::write(
        project_dir.join("Sources/Support/Support.swift"),
        "public enum SupportID { public static let value = \"fixture\" }\n",
    )
    .unwrap();

    let output_file = temp_dir.path().join("swift_import_forms.md");
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.current_dir(project_dir)
        .arg("--include")
        .arg("Sources/Feature/Feature.swift")
        .arg("--trace-imports")
        .arg("--output-file")
        .arg(&output_file);

    cmd.assert().success();
    let content = fs::read_to_string(&output_file).unwrap();

    assert!(contains_path(&content, "Sources/Feature/Feature.swift"));
    assert!(contains_path(&content, "Sources/Models/User.swift"));
    assert!(contains_path(&content, "Sources/Support/Support.swift"));
    assert!(content.contains("public struct User"));
    assert!(content.contains("public enum SupportID"));
}

/// Real-code regression: Swift import parsing handles access-level imports and
/// SPI/preconcurrency attributes used by modern Swift packages.
#[test]
fn test_e2e_swift_trace_imports_handles_modern_import_modifiers() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    fs::write(
        project_dir.join("Package.swift"),
        r#"// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "ModernImports",
    targets: [
        .target(name: "Feature", dependencies: ["Logging", "Support", "Diagnostics"]),
        .target(name: "Logging"),
        .target(name: "Support"),
        .target(name: "Diagnostics")
    ]
)
"#,
    )
    .unwrap();

    fs::create_dir_all(project_dir.join("Sources/Feature")).unwrap();
    fs::create_dir_all(project_dir.join("Sources/Logging")).unwrap();
    fs::create_dir_all(project_dir.join("Sources/Support")).unwrap();
    fs::create_dir_all(project_dir.join("Sources/Diagnostics")).unwrap();

    fs::write(
        project_dir.join("Sources/Feature/Feature.swift"),
        r#"
public import Logging
package import Support
@_spi(Testing) @preconcurrency import Diagnostics

public struct Feature {
    public let logger: Logger
    let support: SupportValue
    let diagnostics: DiagnosticRecorder
}
"#,
    )
    .unwrap();
    fs::write(
        project_dir.join("Sources/Logging/Logger.swift"),
        "public struct Logger { public init() {} }\n",
    )
    .unwrap();
    fs::write(
        project_dir.join("Sources/Support/SupportValue.swift"),
        "public struct SupportValue { public init() {} }\n",
    )
    .unwrap();
    fs::write(
        project_dir.join("Sources/Diagnostics/DiagnosticRecorder.swift"),
        "public struct DiagnosticRecorder { public init() {} }\n",
    )
    .unwrap();

    let output_file = temp_dir.path().join("swift_modern_import_modifiers.md");
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.current_dir(project_dir)
        .arg("--include")
        .arg("Sources/Feature/Feature.swift")
        .arg("--trace-imports")
        .arg("--output-file")
        .arg(&output_file);

    cmd.assert().success();
    let content = fs::read_to_string(&output_file).unwrap();

    assert!(contains_path(&content, "Sources/Feature/Feature.swift"));
    assert!(contains_path(&content, "Sources/Logging/Logger.swift"));
    assert!(contains_path(
        &content,
        "Sources/Support/SupportValue.swift"
    ));
    assert!(contains_path(
        &content,
        "Sources/Diagnostics/DiagnosticRecorder.swift"
    ));
}

/// Real-code regression: Swift allows semicolon-separated statements, including
/// multiple import declarations on the same physical line.
#[test]
fn test_e2e_swift_trace_imports_handles_semicolon_separated_imports() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    fs::write(
        project_dir.join("Package.swift"),
        r#"// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "SemicolonImports",
    targets: [
        .target(name: "Feature", dependencies: ["Logging", "Support", "Diagnostics"]),
        .target(name: "Logging"),
        .target(name: "Support"),
        .target(name: "Diagnostics")
    ]
)
"#,
    )
    .unwrap();

    fs::create_dir_all(project_dir.join("Sources/Feature")).unwrap();
    fs::create_dir_all(project_dir.join("Sources/Logging")).unwrap();
    fs::create_dir_all(project_dir.join("Sources/Support")).unwrap();
    fs::create_dir_all(project_dir.join("Sources/Diagnostics")).unwrap();

    fs::write(
        project_dir.join("Sources/Feature/Feature.swift"),
        r#"
public import Logging; package import Support; @_spi(Testing) @preconcurrency import Diagnostics

public struct Feature {
    public let logger: Logger
    let support: SupportValue
    let diagnostics: DiagnosticRecorder
}
"#,
    )
    .unwrap();
    fs::write(
        project_dir.join("Sources/Logging/Logger.swift"),
        "public struct Logger { public init() {} }\n",
    )
    .unwrap();
    fs::write(
        project_dir.join("Sources/Support/SupportValue.swift"),
        "public struct SupportValue { public init() {} }\n",
    )
    .unwrap();
    fs::write(
        project_dir.join("Sources/Diagnostics/DiagnosticRecorder.swift"),
        "public struct DiagnosticRecorder { public init() {} }\n",
    )
    .unwrap();

    let output_file = temp_dir.path().join("swift_semicolon_imports.md");
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.current_dir(project_dir)
        .arg("--include")
        .arg("Sources/Feature/Feature.swift")
        .arg("--trace-imports")
        .arg("--output-file")
        .arg(&output_file);

    cmd.assert().success();
    let content = fs::read_to_string(&output_file).unwrap();

    assert!(contains_path(&content, "Sources/Feature/Feature.swift"));
    assert!(contains_path(&content, "Sources/Logging/Logger.swift"));
    assert!(contains_path(
        &content,
        "Sources/Support/SupportValue.swift"
    ));
    assert!(contains_path(
        &content,
        "Sources/Diagnostics/DiagnosticRecorder.swift"
    ));
}

/// Real-code regression: Swift import tracing follows custom SwiftPM target
/// paths like Alamofire's `path: "Source"` layout.
#[test]
fn test_e2e_swift_trace_imports_resolves_custom_target_path() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    fs::write(
        project_dir.join("Package.swift"),
        r#"// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "Networking",
    products: [
        .library(name: "Networking", targets: ["Networking"])
    ],
    targets: [
        .target(name: "Networking", path: "Source")
    ]
)
"#,
    )
    .unwrap();

    fs::create_dir_all(project_dir.join("Source/Core")).unwrap();
    fs::create_dir_all(project_dir.join("Source/Features")).unwrap();
    fs::create_dir_all(project_dir.join("Examples/WatchApp")).unwrap();

    fs::write(
        project_dir.join("Examples/WatchApp/ContentView.swift"),
        r#"
import Networking
import SwiftUI

let client = NetworkClient(session: HTTPSession())
"#,
    )
    .unwrap();
    fs::write(
        project_dir.join("Source/Core/NetworkClient.swift"),
        r#"
public struct NetworkClient {
    public let session: HTTPSession
}
"#,
    )
    .unwrap();
    fs::write(
        project_dir.join("Source/Core/HTTPSession.swift"),
        r#"
public struct HTTPSession {
    public init() {}
}
"#,
    )
    .unwrap();
    fs::write(
        project_dir.join("Source/Features/HTTPLogger.swift"),
        r#"
public struct HTTPLogger {
    public init() {}
}
"#,
    )
    .unwrap();

    let output_file = temp_dir.path().join("swift_custom_target_path.md");
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.current_dir(project_dir)
        .arg("--include")
        .arg("Examples/WatchApp/ContentView.swift")
        .arg("--trace-imports")
        .arg("--output-file")
        .arg(&output_file);

    cmd.assert().success();
    let content = fs::read_to_string(&output_file).unwrap();

    assert!(contains_path(
        &content,
        "Examples/WatchApp/ContentView.swift"
    ));
    assert!(contains_path(&content, "Source/Core/NetworkClient.swift"));
    assert!(contains_path(&content, "Source/Core/HTTPSession.swift"));
    assert!(contains_path(&content, "Source/Features/HTTPLogger.swift"));
    assert!(content.contains("public struct NetworkClient"));
    assert!(content.contains("public struct HTTPSession"));
    assert!(content.contains("public struct HTTPLogger"));
}

/// Real-code regression: semantic expansion should preserve project-relative
/// paths for nested single-file Swift inputs instead of emitting bare file names.
#[test]
fn test_e2e_swift_include_types_keeps_nested_single_file_paths() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    fs::write(
        project_dir.join("Package.swift"),
        r#"// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "Networking",
    targets: [
        .target(name: "Networking", path: "Source")
    ]
)
"#,
    )
    .unwrap();

    fs::create_dir_all(project_dir.join("Source/Core")).unwrap();
    fs::create_dir_all(project_dir.join("Source/Features")).unwrap();

    fs::write(
        project_dir.join("Source/Core/Session.swift"),
        r#"
public struct Session {
    public let interceptor: RequestInterceptor
    public let monitor: EventMonitor
}
"#,
    )
    .unwrap();
    fs::write(
        project_dir.join("Source/Features/RequestInterceptor.swift"),
        "public struct RequestInterceptor { public init() {} }\n",
    )
    .unwrap();
    fs::write(
        project_dir.join("Source/Features/EventMonitor.swift"),
        "public struct EventMonitor { public init() {} }\n",
    )
    .unwrap();

    let output_file = temp_dir.path().join("swift_nested_single_file_paths.paths");
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.current_dir(project_dir)
        .arg("--include")
        .arg("Source/Core/Session.swift")
        .arg("--include-types")
        .arg("--style")
        .arg("paths")
        .arg("--output-file")
        .arg(&output_file);

    cmd.assert().success();
    let content = fs::read_to_string(&output_file).unwrap();
    let paths = content.lines().collect::<Vec<_>>();

    assert!(paths.contains(&"Source/Core/Session.swift"));
    assert!(paths.contains(&"Source/Features/RequestInterceptor.swift"));
    assert!(paths.contains(&"Source/Features/EventMonitor.swift"));
    assert!(!paths.contains(&"Session.swift"));
}

/// Real-code regression: semantic expansion canonicalizes symlinked single-file
/// inputs before merging expanded files. On macOS this covers `/tmp` vs
/// `/private/tmp`; on other Unix systems it covers explicit project symlinks.
#[cfg(unix)]
#[test]
fn test_e2e_swift_include_types_dedupes_symlinked_single_file_paths() {
    use std::os::unix::fs::symlink;

    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path().join("project");
    let linked_project_dir = temp_dir.path().join("project-link");
    fs::create_dir_all(&project_dir).unwrap();
    symlink(&project_dir, &linked_project_dir).unwrap();

    fs::write(
        project_dir.join("Package.swift"),
        r#"// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "Networking",
    targets: [
        .target(name: "Networking", path: "Source")
    ]
)
"#,
    )
    .unwrap();

    fs::create_dir_all(project_dir.join("Source/Core")).unwrap();
    fs::create_dir_all(project_dir.join("Source/Features")).unwrap();

    fs::write(
        project_dir.join("Source/Core/Session.swift"),
        r#"
public struct Session {
    public let interceptor: RequestInterceptor
}
"#,
    )
    .unwrap();
    fs::write(
        project_dir.join("Source/Features/RequestInterceptor.swift"),
        "public struct RequestInterceptor { public init() {} }\n",
    )
    .unwrap();

    let output_file = temp_dir
        .path()
        .join("swift_symlinked_single_file_paths.paths");
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg(linked_project_dir.join("Source/Core/Session.swift"))
        .arg("--include-types")
        .arg("--style")
        .arg("paths")
        .arg("--output-file")
        .arg(&output_file);

    cmd.assert().success();
    let content = fs::read_to_string(&output_file).unwrap();
    let paths = content.lines().collect::<Vec<_>>();

    assert_eq!(
        paths
            .iter()
            .filter(|path| **path == "Source/Core/Session.swift")
            .count(),
        1
    );
    assert!(paths.contains(&"Source/Features/RequestInterceptor.swift"));
    assert!(!paths.contains(&"Session.swift"));
}

/// Real-code regression: Swift import tracing follows flat SwiftPM target paths
/// like SnapKit's `.target(name: "SnapKit", path: "Sources")` layout.
#[test]
fn test_e2e_swift_trace_imports_resolves_flat_sources_target_path() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    fs::write(
        project_dir.join("Package.swift"),
        r#"// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "LayoutKit",
    products: [
        .library(name: "LayoutKit", targets: ["LayoutKit"])
    ],
    targets: [
        .target(name: "LayoutKit", path: "Sources"),
        .testTarget(name: "LayoutKitTests", dependencies: ["LayoutKit"])
    ]
)
"#,
    )
    .unwrap();

    fs::create_dir_all(project_dir.join("Sources")).unwrap();
    fs::create_dir_all(project_dir.join("Tests/LayoutKitTests")).unwrap();

    fs::write(
        project_dir.join("Tests/LayoutKitTests/ConstraintTests.swift"),
        r#"
import XCTest
@testable import LayoutKit

final class ConstraintTests: XCTestCase {
    func testMaker() {
        _ = ConstraintMaker(description: ConstraintDescription())
    }
}
"#,
    )
    .unwrap();
    fs::write(
        project_dir.join("Sources/ConstraintMaker.swift"),
        r#"
public struct ConstraintMaker {
    public let description: ConstraintDescription
}
"#,
    )
    .unwrap();
    fs::write(
        project_dir.join("Sources/ConstraintDescription.swift"),
        r#"
public struct ConstraintDescription {
    public init() {}
}
"#,
    )
    .unwrap();

    let output_file = temp_dir.path().join("swift_flat_sources_target_path.md");
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.current_dir(project_dir)
        .arg("--include")
        .arg("Tests/LayoutKitTests/ConstraintTests.swift")
        .arg("--trace-imports")
        .arg("--output-file")
        .arg(&output_file);

    cmd.assert().success();
    let content = fs::read_to_string(&output_file).unwrap();

    assert!(contains_path(
        &content,
        "Tests/LayoutKitTests/ConstraintTests.swift"
    ));
    assert!(contains_path(&content, "Sources/ConstraintMaker.swift"));
    assert!(contains_path(
        &content,
        "Sources/ConstraintDescription.swift"
    ));
    assert!(content.contains("public struct ConstraintMaker"));
    assert!(content.contains("public struct ConstraintDescription"));
}

/// Real-code regression: Xcode-style Swift apps often do not have a
/// Package.swift manifest. Type expansion still needs to resolve sibling
/// feature/model/service groups from the project root.
#[test]
fn test_e2e_swift_include_types_resolves_xcode_style_project_without_manifest() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    fs::create_dir_all(project_dir.join("App/Features/Profile")).unwrap();
    fs::create_dir_all(project_dir.join("App/Models")).unwrap();
    fs::create_dir_all(project_dir.join("App/Services")).unwrap();
    fs::create_dir_all(project_dir.join("App/Unused")).unwrap();

    fs::write(
        project_dir.join("App/Features/Profile/ProfileViewModel.swift"),
        r#"
import Foundation

final class ProfileViewModel {
    let profile: UserProfile
    let state: ProfileState
    let worker: ProfileRefreshWorker

    init(profile: UserProfile, state: ProfileState, worker: ProfileRefreshWorker) {
        self.profile = profile
        self.state = state
        self.worker = worker
    }
}
"#,
    )
    .unwrap();
    fs::write(
        project_dir.join("App/Models/ProfileModels.swift"),
        r#"
struct UserProfile {
    let displayName: String
}

enum ProfileState {
    case loading
    case loaded(UserProfile)
}
"#,
    )
    .unwrap();
    fs::write(
        project_dir.join("App/Services/ProfileServices.swift"),
        "actor ProfileRefreshWorker { func refresh() async {} }\n",
    )
    .unwrap();
    fs::write(
        project_dir.join("App/Unused/Unused.swift"),
        "struct UnusedProfileFixture { let name: String }\n",
    )
    .unwrap();

    let output_file = temp_dir.path().join("swift_xcode_style_project.paths");
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.current_dir(project_dir)
        .arg("--include")
        .arg("App/Features/Profile/ProfileViewModel.swift")
        .arg("--include-types")
        .arg("--style")
        .arg("paths")
        .arg("--output-file")
        .arg(&output_file);

    cmd.assert().success();
    let content = fs::read_to_string(&output_file).unwrap();
    let paths = content.lines().collect::<Vec<_>>();

    assert!(paths.contains(&"App/Features/Profile/ProfileViewModel.swift"));
    assert!(paths.contains(&"App/Models/ProfileModels.swift"));
    assert!(paths.contains(&"App/Services/ProfileServices.swift"));
    assert!(!paths.contains(&"App/Unused/Unused.swift"));
}
