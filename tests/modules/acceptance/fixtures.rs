//! Common test fixtures and project structures for acceptance tests

#![allow(dead_code)] // These fixtures will be used in later test phases

use super::builders::*;
use std::path::PathBuf;
use tempfile::TempDir;

/// Create a simple Python project with basic structure
pub fn create_python_basic_project() -> (TempDir, PathBuf) {
    PythonProjectBuilder::new()
        .add_file(
            "src/main.py",
            r#"
def main():
    print("Hello from Python")
    
if __name__ == "__main__":
    main()
"#,
        )
        .add_file(
            "src/utils.py",
            r#"
def helper():
    return "Helper function"
    
def calculate_sum(a, b):
    return a + b
"#,
        )
        .add_file(
            "tests/test_main.py",
            r#"
import unittest
from src.main import main

class TestMain(unittest.TestCase):
    def test_main(self):
        # Test implementation
        pass
"#,
        )
        .build()
}

/// Create a Python project with imports and function calls
pub fn create_python_with_imports() -> (TempDir, PathBuf) {
    PythonProjectBuilder::new()
        .add_file(
            "utils.py",
            r#"
def calculate_price(quantity, unit_price):
    """Calculate total price"""
    return quantity * unit_price

def format_currency(amount):
    """Format amount as currency"""
    return f"${amount:.2f}"
"#,
        )
        .add_file(
            "main.py",
            r#"
from utils import calculate_price, format_currency

def process_order(items):
    total = 0
    for item in items:
        price = calculate_price(item['quantity'], item['price'])
        total += price
    return format_currency(total)

if __name__ == "__main__":
    order = [{'quantity': 2, 'price': 10.50}]
    print(process_order(order))
"#,
        )
        .add_file(
            "api.py",
            r#"
from utils import calculate_price

def get_item_price(item_id, quantity):
    # Simulate API call
    unit_price = 10.0  # Mock price
    return calculate_price(quantity, unit_price)
"#,
        )
        .add_file(
            "other.py",
            r#"
# This file doesn't import utils
def unrelated_function():
    return "Not related to pricing"
"#,
        )
        .build()
}

/// Create a TypeScript project with basic structure
pub fn create_typescript_basic_project() -> (TempDir, PathBuf) {
    TypeScriptProjectBuilder::new()
        .add_file(
            "src/index.ts",
            r#"
function main(): void {
    console.log("Hello from TypeScript");
}

main();
"#,
        )
        .add_file(
            "components/Button.tsx",
            r#"
interface ButtonProps {
    label: string;
    onClick: () => void;
}

export function Button({ label, onClick }: ButtonProps): JSX.Element {
    return <button onClick={onClick}>{label}</button>;
}
"#,
        )
        .add_file(
            "package.json",
            r#"{
    "name": "test-project",
    "version": "1.0.0"
}"#,
        )
        .build()
}

/// Create a TypeScript project with exports and imports
pub fn create_typescript_with_exports() -> (TempDir, PathBuf) {
    TypeScriptProjectBuilder::new()
        .add_file(
            "src/utils.ts",
            r#"
export function formatDate(date: Date): string {
    return date.toISOString().split('T')[0];
}

export function parseDate(dateString: string): Date {
    return new Date(dateString);
}
"#,
        )
        .add_file(
            "src/components/Calendar.tsx",
            r#"
import { formatDate } from '../utils';

interface CalendarProps {
    currentDate: Date;
}

export function Calendar({ currentDate }: CalendarProps): JSX.Element {
    const formattedDate = formatDate(currentDate);
    return <div>Current date: {formattedDate}</div>;
}
"#,
        )
        .add_file(
            "src/types.ts",
            r#"
export interface IUser {
    id: string;
    name: string;
    email: string;
    createdAt: Date;
}

export type UserRole = 'admin' | 'user' | 'guest';
"#,
        )
        .add_file(
            "src/handlers.ts",
            r#"
import { IUser } from './types';

export function handleUserCreation(userData: IUser): void {
    console.log('Creating user:', userData.name);
}
"#,
        )
        .build()
}

/// Create a Rust project with basic structure
pub fn create_rust_basic_project() -> (TempDir, PathBuf) {
    RustProjectBuilder::new()
        .add_file(
            "src/main.rs",
            r#"
fn main() {
    println!("Hello from Rust");
}
"#,
        )
        .add_file(
            "src/lib.rs",
            r#"
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        assert_eq!(add(2, 2), 4);
    }
}
"#,
        )
        .add_file("target/debug/my_app", "binary file")
        .build()
}

/// Create a Rust project with modules and function calls
pub fn create_rust_with_modules() -> (TempDir, PathBuf) {
    RustProjectBuilder::new()
        .add_file(
            "Cargo.toml",
            r#"[package]
name = "my_lib"
version = "0.1.0"
edition = "2021"
"#,
        )
        .add_file(
            "src/lib.rs",
            r#"
pub mod parsing;
pub mod processing;
"#,
        )
        .add_file(
            "src/parsing.rs",
            r#"
pub fn parse_line(line: &str) -> Vec<String> {
    line.split_whitespace()
        .map(|s| s.to_string())
        .collect()
}

pub fn parse_file(content: &str) -> Vec<Vec<String>> {
    content.lines()
        .map(parse_line)
        .collect()
}
"#,
        )
        .add_file(
            "src/main.rs",
            r#"
use my_lib::parsing::parse_line;

fn main() {
    let line = "hello world rust";
    let tokens = parse_line(line);
    println!("Tokens: {:?}", tokens);
}
"#,
        )
        .add_file(
            "src/processing.rs",
            r#"
use crate::parsing::parse_file;

pub fn process_content(content: &str) -> usize {
    let parsed = parse_file(content);
    parsed.len()
}
"#,
        )
        .build()
}

/// Create a Rust project with structs and type usage
pub fn create_rust_with_types() -> (TempDir, PathBuf) {
    RustProjectBuilder::new()
        .add_file(
            "src/models.rs",
            r#"
#[derive(Debug, Clone)]
pub struct User {
    pub id: u64,
    pub name: String,
    pub email: String,
}

impl User {
    pub fn new(id: u64, name: String, email: String) -> Self {
        User { id, name, email }
    }
}

#[derive(Debug)]
pub struct Config {
    pub database_url: String,
    pub port: u16,
}
"#,
        )
        .add_file(
            "src/processing.rs",
            r#"
use crate::models::User;

pub fn process_user(user: &User) -> String {
    format!("Processing user: {} ({})", user.name, user.email)
}

pub fn validate_user(user: &User) -> bool {
    !user.email.is_empty() && user.email.contains('@')
}
"#,
        )
        .add_file(
            "src/handlers.rs",
            r#"
use crate::models::{User, Config};

pub fn handle_request(config: &Config, user: User) -> Result<(), String> {
    if config.database_url.is_empty() {
        return Err("Invalid config".to_string());
    }
    
    println!("Handling request for user: {}", user.name);
    Ok(())
}
"#,
        )
        .build()
}

/// Create a mixed project structure for testing ignore patterns
pub fn create_project_with_test_files() -> (TempDir, PathBuf) {
    TypeScriptProjectBuilder::new()
        .add_file(
            "src/utils.ts",
            r#"
export function add(a: number, b: number): number {
    return a + b;
}
"#,
        )
        .add_file(
            "src/utils.test.ts",
            r#"
import { add } from './utils';

test('add function', () => {
    expect(add(2, 2)).toBe(4);
});
"#,
        )
        .add_file(
            "src/api.ts",
            r#"
export function fetchData(): Promise<any> {
    return Promise.resolve({ data: 'test' });
}
"#,
        )
        .add_file(
            "src/api.spec.ts",
            r#"
import { fetchData } from './api';

describe('fetchData', () => {
    it('should return data', async () => {
        const result = await fetchData();
        expect(result.data).toBe('test');
    });
});
"#,
        )
        .build()
}
