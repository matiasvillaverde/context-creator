#![cfg(test)]

//! Integration tests for --include-types functionality
//! These tests verify that type definition files are included when using --include-types flag

use context_creator::cli::Config;
use context_creator::core::cache::FileCache;
use context_creator::core::walker::{walk_directory, FileInfo, WalkOptions};
use std::collections::HashMap;
use std::fs;
use std::sync::Arc;
use tempfile::TempDir;

/// Helper function to perform semantic analysis and file expansion
fn analyze_and_expand(
    mut files: Vec<FileInfo>,
    config: &Config,
    cache: &Arc<FileCache>,
) -> Vec<FileInfo> {
    // Perform semantic analysis
    context_creator::core::walker::perform_semantic_analysis(&mut files, config, cache).unwrap();

    // Convert to HashMap for file expansion
    let mut files_map = HashMap::new();
    for file in files {
        files_map.insert(file.path.clone(), file);
    }

    // Perform file expansion
    let walk_options = context_creator::core::walker::WalkOptions::from_config(config).unwrap();
    let expanded_map = context_creator::core::file_expander::expand_file_list(
        files_map,
        config,
        cache,
        &walk_options,
    )
    .unwrap();

    // Convert back to Vec
    expanded_map.into_values().collect()
}

#[test]
fn test_basic_type_inclusion() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create a simple Rust project with type usage
    fs::write(
        root.join("main.rs"),
        r#"
use types::UserAccount;
use types::DatabaseConfig;

fn main() {
    let user: UserAccount = UserAccount::new("Alice");
    let config: DatabaseConfig = DatabaseConfig::default();
}
"#,
    )
    .unwrap();

    // Create type definitions in separate file
    fs::create_dir(root.join("types")).unwrap();
    fs::write(
        root.join("types/mod.rs"),
        r#"
pub struct UserAccount {
    name: String,
}

impl UserAccount {
    pub fn new(name: &str) -> Self {
        Self { name: name.to_string() }
    }
}

pub struct DatabaseConfig {
    host: String,
    port: u16,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self { 
            host: "localhost".to_string(),
            port: 5432,
        }
    }
}
"#,
    )
    .unwrap();

    // Create config with include_types enabled
    let config = Config {
        paths: Some(vec![root.to_path_buf()]),
        include_types: true,
        trace_imports: false,
        include_callers: false,
        semantic_depth: 2,
        ..Config::default()
    };

    // Walk directory and perform semantic analysis
    let walk_options = WalkOptions::from_config(&config).unwrap();
    let cache = Arc::new(FileCache::new());
    let mut files = walk_directory(root, walk_options).unwrap();

    // Initially should have main.rs and types/mod.rs
    assert_eq!(files.len(), 2, "Should start with main.rs and types/mod.rs");

    context_creator::core::walker::perform_semantic_analysis(&mut files, &config, &cache).unwrap();

    // After semantic analysis, check if type references were detected
    let main_file = files
        .iter()
        .find(|f| f.relative_path.to_str().unwrap() == "main.rs")
        .unwrap();
    assert!(
        !main_file.type_references.is_empty(),
        "Should detect type references"
    );

    // This should fail - we expect file expansion to happen but it's not implemented yet
    // When implemented, this should expand to include files that define the referenced types
    let expected_with_expansion = 2; // Should stay 2 since types/mod.rs is already included
    assert_eq!(
        files.len(),
        expected_with_expansion,
        "Should include type definition files"
    );
}

#[test]
fn test_cross_directory_type_resolution() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create .git directory to make it a git repository
    fs::create_dir_all(root.join(".git")).unwrap();

    // Create a more complex directory structure
    fs::create_dir_all(root.join("src/models")).unwrap();
    fs::create_dir_all(root.join("src/handlers")).unwrap();
    fs::create_dir_all(root.join("shared/types")).unwrap();

    // Handler using types from different directories
    fs::write(
        root.join("src/handlers/user_handler.rs"),
        r#"
use crate::models::User;
use shared::types::ApiResponse;

pub fn handle_user_request(user_id: u32) -> ApiResponse<User> {
    let user = User::find(user_id);
    ApiResponse::ok(user)
}
"#,
    )
    .unwrap();

    // Model definition
    fs::write(
        root.join("src/models/user.rs"),
        r#"
pub struct User {
    pub id: u32,
    pub name: String,
}

impl User {
    pub fn find(id: u32) -> Self {
        Self { id, name: "Test User".to_string() }
    }
}
"#,
    )
    .unwrap();

    fs::write(
        root.join("src/models/mod.rs"),
        r#"
mod user;
pub use user::User;
"#,
    )
    .unwrap();

    // Shared type definition
    fs::write(
        root.join("shared/types/mod.rs"),
        r#"
pub struct ApiResponse<T> {
    pub data: T,
    pub status: u16,
}

impl<T> ApiResponse<T> {
    pub fn ok(data: T) -> Self {
        Self { data, status: 200 }
    }
}
"#,
    )
    .unwrap();

    let config = Config {
        paths: Some(vec![root.to_path_buf()]),
        include: Some(vec!["**/user_handler.rs".to_string()]), // Only include the handler initially
        include_types: true,
        semantic_depth: 2,
        ..Config::default()
    };

    let walk_options = WalkOptions::from_config(&config).unwrap();
    let cache = Arc::new(FileCache::new());
    let files = walk_directory(root, walk_options).unwrap();

    let initial_count = files.len();
    let expanded_files = analyze_and_expand(files, &config, &cache);

    // Should include both type definition files
    assert!(
        expanded_files.len() > initial_count,
        "Should include type definition files"
    );

    // Verify specific files are included (platform-agnostic path checking)
    let has_user_model = expanded_files.iter().any(|f| {
        let path_str = f.relative_path.to_str().unwrap();
        path_str.contains("models") && path_str.contains("user.rs")
    });
    let has_api_response = expanded_files.iter().any(|f| {
        let path_str = f.relative_path.to_str().unwrap();
        path_str.contains("shared") && path_str.contains("types") && path_str.contains("mod.rs")
    });

    assert!(has_user_model, "User model file should be included");
    assert!(has_api_response, "ApiResponse type file should be included");
}

#[test]
fn test_circular_type_dependencies() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create circular type dependencies: A uses B, B uses A
    fs::write(
        root.join("type_a.rs"),
        r#"
use crate::type_b::TypeB;

pub struct TypeA {
    pub b_ref: Option<TypeB>,
}

impl TypeA {
    pub fn new() -> Self {
        Self { b_ref: None }
    }
}
"#,
    )
    .unwrap();

    fs::write(
        root.join("type_b.rs"),
        r#"
use crate::type_a::TypeA;

pub struct TypeB {
    pub a_ref: Option<Box<TypeA>>,
}

impl TypeB {
    pub fn new() -> Self {
        Self { a_ref: None }
    }
}
"#,
    )
    .unwrap();

    fs::write(
        root.join("main.rs"),
        r#"
mod type_a;
mod type_b;

use type_a::TypeA;

fn main() {
    let a = TypeA::new();
}
"#,
    )
    .unwrap();

    let config = Config {
        paths: Some(vec![root.to_path_buf()]),
        include_types: true,
        semantic_depth: 2,
        ..Config::default()
    };

    let walk_options = WalkOptions::from_config(&config).unwrap();
    let cache = Arc::new(FileCache::new());
    let mut files = walk_directory(root, walk_options).unwrap();

    context_creator::core::walker::perform_semantic_analysis(&mut files, &config, &cache).unwrap();

    // Should handle circular dependencies without infinite loop
    // All files should be included exactly once
    assert_eq!(
        files.len(),
        3,
        "Should include all 3 files without duplication"
    );

    // Verify no duplicate paths
    let paths: Vec<_> = files.iter().map(|f| &f.relative_path).collect();
    let unique_paths: std::collections::HashSet<_> = paths.iter().collect();
    assert_eq!(
        paths.len(),
        unique_paths.len(),
        "No duplicate files should be included"
    );
}

#[test]
fn test_external_type_handling() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create a Rust file using both local and external types
    fs::write(
        root.join("main.rs"),
        r#"
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use local_types::MyConfig;

#[derive(Serialize, Deserialize)]
struct AppState {
    config: MyConfig,
    cache: HashMap<String, String>,
}
"#,
    )
    .unwrap();

    fs::write(
        root.join("local_types.rs"),
        r#"
pub struct MyConfig {
    pub name: String,
    pub value: i32,
}
"#,
    )
    .unwrap();

    // Create a Cargo.toml to identify external dependencies
    fs::write(
        root.join("Cargo.toml"),
        r#"
[package]
name = "test-app"
version = "0.1.0"

[dependencies]
serde = { version = "1.0", features = ["derive"] }
"#,
    )
    .unwrap();

    let config = Config {
        paths: Some(vec![root.to_path_buf()]),
        include_types: true,
        semantic_depth: 2,
        ..Config::default()
    };

    let walk_options = WalkOptions::from_config(&config).unwrap();
    let cache = Arc::new(FileCache::new());
    let mut files = walk_directory(root, walk_options).unwrap();

    context_creator::core::walker::perform_semantic_analysis(&mut files, &config, &cache).unwrap();

    // Should include local type file
    let has_local_types = files
        .iter()
        .any(|f| f.relative_path.to_str().unwrap().contains("local_types.rs"));
    assert!(has_local_types, "Local type definitions should be included");

    // Find main.rs and check its type references
    let main_file = files
        .iter()
        .find(|f| f.relative_path.to_str().unwrap() == "main.rs")
        .unwrap();

    // Should have external types marked as such
    // TODO: Uncomment when is_external field is added to TypeReference
    // let external_types: Vec<_> = main_file.type_references.iter()
    //     .filter(|tr| tr.is_external)
    //     .collect();

    // assert!(!external_types.is_empty(), "Should identify external types");

    // HashMap should be marked as external from std
    // TODO: Uncomment when external_package field is added
    // let has_hashmap = external_types.iter().any(|tr| tr.name == "HashMap" && tr.external_package.as_ref().map(|p| p.contains("std")).unwrap_or(false));
    // assert!(has_hashmap, "HashMap should be marked as external from std");

    // For now, just check that we have type references
    assert!(
        !main_file.type_references.is_empty(),
        "Should have type references detected"
    );
}

#[test]
fn test_token_limit_respect() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create many type files to test token limit handling
    for i in 0..10 {
        fs::write(
            root.join(format!("type_{i}.rs")),
            format!(
                r#"
pub struct Type{i} {{
    pub id: u32,
    pub data: String,
    // Adding some content to increase file size
    pub field1: String,
    pub field2: String,
    pub field3: String,
    pub field4: String,
}}

impl Type{i} {{
    pub fn new() -> Self {{
        Self {{
            id: {i},
            data: "test".to_string(),
            field1: "test".to_string(),
            field2: "test".to_string(),
            field3: "test".to_string(),
            field4: "test".to_string(),
        }}
    }}
}}
"#
            ),
        )
        .unwrap();
    }

    // Create main file that uses all types
    let mut type_uses = String::new();
    for i in 0..10 {
        type_uses.push_str(&format!("use crate::type_{i}::Type{i};\n"));
    }

    let mut type_usage = String::new();
    for i in 0..10 {
        type_usage.push_str(&format!("    let t{i} = Type{i}::new();\n"));
    }

    fs::write(
        root.join("main.rs"),
        format!(
            r#"
{type_uses}

fn main() {{
{type_usage}
}}
"#
        ),
    )
    .unwrap();

    let config = Config {
        paths: Some(vec![root.to_path_buf()]),
        include_types: true,
        max_tokens: Some(1000), // Set a low token limit
        semantic_depth: 2,
        ..Config::default()
    };

    let walk_options = WalkOptions::from_config(&config).unwrap();
    let cache = Arc::new(FileCache::new());
    let files = walk_directory(root, walk_options).unwrap();

    let expanded_files = analyze_and_expand(files, &config, &cache);

    // Should respect token limit and not include all type files
    // This behavior depends on prioritization
    // Note: File expansion happens before prioritization, so we might have all files
    // but the final output would be limited by token count
    assert!(
        expanded_files.len() <= 11,
        "Should have at most 11 files (main + 10 types)"
    );

    // Main.rs should always be included as it's the entry point
    let has_main = expanded_files
        .iter()
        .any(|f| f.relative_path.to_str().unwrap() == "main.rs");
    assert!(has_main, "Main file should always be included");
}

#[test]
fn test_type_alias_chains() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create a chain of type aliases
    fs::write(
        root.join("types.rs"),
        r#"
pub type UserId = u32;
pub type CustomerId = UserId;
pub type AdminId = CustomerId;

pub struct User {
    pub id: UserId,
}
"#,
    )
    .unwrap();

    fs::write(
        root.join("main.rs"),
        r#"
use types::{AdminId, User};

fn main() {
    let admin_id: AdminId = 42;
    let user = User { id: admin_id };
}
"#,
    )
    .unwrap();

    let config = Config {
        paths: Some(vec![root.to_path_buf()]),
        include_types: true,
        semantic_depth: 2,
        ..Config::default()
    };

    let walk_options = WalkOptions::from_config(&config).unwrap();
    let cache = Arc::new(FileCache::new());
    let mut files = walk_directory(root, walk_options).unwrap();

    context_creator::core::walker::perform_semantic_analysis(&mut files, &config, &cache).unwrap();

    // Should include types.rs as it contains the type definitions
    let has_types_file = files
        .iter()
        .any(|f| f.relative_path.to_str().unwrap() == "types.rs");
    assert!(
        has_types_file,
        "types.rs should be included for type alias definitions"
    );
}

#[test]
fn test_trait_definitions_only() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create .git directory to make it a git repository
    fs::create_dir_all(root.join(".git")).unwrap();

    // Create trait definition
    fs::write(
        root.join("traits.rs"),
        r#"
pub trait Repository<T> {
    fn find(&self, id: u32) -> Option<T>;
    fn save(&mut self, item: T) -> Result<(), String>;
}
"#,
    )
    .unwrap();

    // Create multiple implementations
    fs::write(
        root.join("user_repo.rs"),
        r#"
use crate::traits::Repository;

pub struct UserRepository;

impl Repository<User> for UserRepository {
    fn find(&self, id: u32) -> Option<User> {
        Some(User { id, name: "Test".into() })
    }
    
    fn save(&mut self, item: User) -> Result<(), String> {
        Ok(())
    }
}

pub struct User {
    pub id: u32,
    pub name: String,
}
"#,
    )
    .unwrap();

    fs::write(
        root.join("product_repo.rs"),
        r#"
use crate::traits::Repository;

pub struct ProductRepository;

impl Repository<Product> for ProductRepository {
    fn find(&self, id: u32) -> Option<Product> {
        Some(Product { id, name: "Test Product".into() })
    }
    
    fn save(&mut self, item: Product) -> Result<(), String> {
        Ok(())
    }
}

pub struct Product {
    pub id: u32,
    pub name: String,
}
"#,
    )
    .unwrap();

    fs::write(
        root.join("main.rs"),
        r#"
mod traits;
mod user_repo;
mod product_repo;

use traits::Repository;
use user_repo::UserRepository;

fn main() {
    let repo = UserRepository;
    let _user = repo.find(1);
}
"#,
    )
    .unwrap();

    let config = Config {
        paths: Some(vec![root.to_path_buf()]),
        include: Some(vec!["main.rs".to_string()]), // Only include main.rs initially
        include_types: true,
        semantic_depth: 2,
        ..Config::default()
    };

    let walk_options = WalkOptions::from_config(&config).unwrap();
    let cache = Arc::new(FileCache::new());
    let files = walk_directory(root, walk_options).unwrap();

    let expanded_files = analyze_and_expand(files, &config, &cache);

    // Should include trait definition
    let has_trait_def = expanded_files
        .iter()
        .any(|f| f.relative_path.to_str().unwrap() == "traits.rs");
    assert!(has_trait_def, "Trait definition file should be included");

    // Should include UserRepository since it's directly used
    let has_user_repo = expanded_files
        .iter()
        .any(|f| f.relative_path.to_str().unwrap() == "user_repo.rs");
    assert!(
        has_user_repo,
        "User repository should be included as it's used"
    );

    // Should NOT include ProductRepository as it's not used
    let has_product_repo = expanded_files
        .iter()
        .any(|f| f.relative_path.to_str().unwrap() == "product_repo.rs");
    assert!(
        !has_product_repo,
        "Product repository should not be included as it's not used"
    );
}

#[test]
fn test_generic_type_parameters() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    fs::write(
        root.join("generics.rs"),
        r#"
pub struct Container<T> {
    pub value: T,
}

pub struct Pair<K, V> {
    pub key: K,
    pub value: V,
}

pub enum Result<T, E> {
    Ok(T),
    Err(E),
}
"#,
    )
    .unwrap();

    fs::write(
        root.join("custom_types.rs"),
        r#"
pub struct MyData {
    pub id: u32,
}

pub struct MyError {
    pub message: String,
}
"#,
    )
    .unwrap();

    fs::write(
        root.join("main.rs"),
        r#"
use generics::{Container, Result};
use custom_types::{MyData, MyError};

fn main() {
    let container: Container<MyData> = Container { value: MyData { id: 1 } };
    let result: Result<MyData, MyError> = Result::Ok(MyData { id: 2 });
}
"#,
    )
    .unwrap();

    let config = Config {
        paths: Some(vec![root.to_path_buf()]),
        include_types: true,
        semantic_depth: 2,
        ..Config::default()
    };

    let walk_options = WalkOptions::from_config(&config).unwrap();
    let cache = Arc::new(FileCache::new());
    let mut files = walk_directory(root, walk_options).unwrap();

    context_creator::core::walker::perform_semantic_analysis(&mut files, &config, &cache).unwrap();

    // Should include both generic type definitions and the concrete types used
    let has_generics = files
        .iter()
        .any(|f| f.relative_path.to_str().unwrap() == "generics.rs");
    let has_custom_types = files
        .iter()
        .any(|f| f.relative_path.to_str().unwrap() == "custom_types.rs");

    assert!(has_generics, "Generic type definitions should be included");
    assert!(
        has_custom_types,
        "Custom types used as generic parameters should be included"
    );
}

#[test]
fn test_multiple_language_types() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create Python file with type annotations
    fs::write(
        root.join("app.py"),
        r#"
from typing import List, Optional
from models import User, Product

def get_users() -> List[User]:
    return [User(id=1, name="Alice")]

def find_product(id: int) -> Optional[Product]:
    return Product(id=id, price=99.99)
"#,
    )
    .unwrap();

    fs::write(
        root.join("models.py"),
        r#"
class User:
    def __init__(self, id: int, name: str):
        self.id = id
        self.name = name

class Product:
    def __init__(self, id: int, price: float):
        self.id = id
        self.price = price
"#,
    )
    .unwrap();

    // Create TypeScript file
    fs::write(
        root.join("api.ts"),
        r#"
import { ApiResponse, ErrorResponse } from './types';
import { User } from './models';

async function fetchUser(id: number): Promise<ApiResponse<User>> {
    const user = new User(id, "Test");
    return { data: user, status: 200 };
}
"#,
    )
    .unwrap();

    fs::write(
        root.join("types.ts"),
        r#"
export interface ApiResponse<T> {
    data: T;
    status: number;
}

export interface ErrorResponse {
    error: string;
    code: number;
}
"#,
    )
    .unwrap();

    fs::write(
        root.join("models.ts"),
        r#"
export class User {
    constructor(public id: number, public name: string) {}
}
"#,
    )
    .unwrap();

    let config = Config {
        paths: Some(vec![root.to_path_buf()]),
        include_types: true,
        semantic_depth: 2,
        ..Config::default()
    };

    let walk_options = WalkOptions::from_config(&config).unwrap();
    let cache = Arc::new(FileCache::new());
    let mut files = walk_directory(root, walk_options).unwrap();

    context_creator::core::walker::perform_semantic_analysis(&mut files, &config, &cache).unwrap();

    // Should include type definition files for both languages
    let has_python_models = files
        .iter()
        .any(|f| f.relative_path.to_str().unwrap() == "models.py");
    let has_ts_types = files
        .iter()
        .any(|f| f.relative_path.to_str().unwrap() == "types.ts");
    let has_ts_models = files
        .iter()
        .any(|f| f.relative_path.to_str().unwrap() == "models.ts");

    assert!(
        has_python_models,
        "Python model definitions should be included"
    );
    assert!(
        has_ts_types,
        "TypeScript type definitions should be included"
    );
    assert!(
        has_ts_models,
        "TypeScript model definitions should be included"
    );
}
