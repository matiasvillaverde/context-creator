use context_creator::cli::Config;
use context_creator::core::cache::FileCache;
use context_creator::core::walker::{walk_directory, FileInfo, WalkOptions};
use std::collections::HashMap;
use std::fs;
use std::sync::Arc;
use tempfile::TempDir;

fn main() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create the same structure as the failing test
    fs::create_dir_all(root.join("src/models")).unwrap();
    fs::create_dir_all(root.join("src/handlers")).unwrap();
    fs::create_dir_all(root.join("shared/types")).unwrap();

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
    ).unwrap();

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
    ).unwrap();

    fs::write(
        root.join("src/models/mod.rs"),
        r#"
mod user;
pub use user::User;
"#,
    ).unwrap();

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
    ).unwrap();

    let config = Config {
        paths: Some(vec![root.to_path_buf()]),
        include: Some(vec!["**/user_handler.rs".to_string()]),
        include_types: true,
        semantic_depth: 2,
        ..Config::default()
    };

    let walk_options = WalkOptions::from_config(&config).unwrap();
    let cache = Arc::new(FileCache::new());
    let mut files = walk_directory(root, walk_options).unwrap();

    println!("Initial files found: {}", files.len());
    for file in &files {
        println!("  - {}", file.relative_path.display());
    }

    // Perform semantic analysis
    context_creator::core::walker::perform_semantic_analysis(&mut files, &config, &cache).unwrap();

    // Check what type references were found
    for file in &files {
        println!("\nFile: {}", file.relative_path.display());
        println!("  Type references: {}", file.type_references.len());
        for type_ref in &file.type_references {
            println!("    - {}: {:?} (definition_path: {:?})", 
                    type_ref.name, type_ref.module, type_ref.definition_path);
        }
    }

    // Convert to HashMap for file expansion
    let mut files_map = HashMap::new();
    for file in files {
        files_map.insert(file.path.clone(), file);
    }

    println!("\nBefore expansion: {} files", files_map.len());

    // Perform file expansion
    let expanded_map = context_creator::core::file_expander::expand_file_list(files_map, &config, &cache).unwrap();

    println!("After expansion: {} files", expanded_map.len());
    for file in expanded_map.values() {
        println!("  - {}", file.relative_path.display());
    }
}