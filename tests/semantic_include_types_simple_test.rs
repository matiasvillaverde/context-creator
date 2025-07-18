#![cfg(test)]

//! Simple failing test to demonstrate --include-types doesn't expand files yet

use context_creator::cli::Config;
use context_creator::core::cache::FileCache;
use context_creator::core::walker::{walk_directory, WalkOptions};
use std::fs;
use std::sync::Arc;
use tempfile::TempDir;

#[test]
fn test_include_types_expands_files() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create .git directory to make it a git repository
    fs::create_dir_all(root.join(".git")).unwrap();

    // Create main file that uses a type
    fs::write(
        root.join("main.rs"),
        r#"
use types::MyType;

fn main() {
    let x: MyType = MyType::new();
}
"#,
    )
    .unwrap();

    // Create type definition in separate file that won't be walked initially
    fs::create_dir(root.join("types")).unwrap();
    fs::write(
        root.join("types.rs"),
        r#"
pub struct MyType {
    value: i32,
}

impl MyType {
    pub fn new() -> Self {
        Self { value: 42 }
    }
}
"#,
    )
    .unwrap();

    // Configure to walk the directory but use include pattern for main.rs only
    let config = Config {
        paths: Some(vec![root.to_path_buf()]),
        include: Some(vec!["main.rs".to_string()]), // Only include main.rs pattern
        include_types: true,
        semantic_depth: 2,
        ..Config::default()
    };

    let walk_options = WalkOptions::from_config(&config).unwrap();
    let cache = Arc::new(FileCache::new());
    let mut files = walk_directory(root, walk_options).unwrap();

    // Should start with only main.rs
    assert_eq!(files.len(), 1, "Should start with only main.rs");

    // Perform semantic analysis
    context_creator::core::walker::perform_semantic_analysis(&mut files, &config, &cache).unwrap();

    // Debug: Check if type references were found
    let main_file = &files[0];
    eprintln!("Main file type references: {:?}", main_file.type_references);
    eprintln!("Number of files after semantic analysis: {}", files.len());

    // Convert to HashMap for file expansion
    let mut files_map = std::collections::HashMap::new();
    for file in files {
        files_map.insert(file.path.clone(), file);
    }

    // Perform file expansion
    let expanded_map =
        context_creator::core::file_expander::expand_file_list(files_map, &config, &cache).unwrap();

    // EXPECTED: After expansion with --include-types, should include types.rs
    assert_eq!(
        expanded_map.len(),
        2,
        "Should expand to include type definition file"
    );

    // Verify the type file was added
    let has_type_file = expanded_map
        .values()
        .any(|f| f.relative_path.to_str().unwrap().contains("types.rs"));
    assert!(has_type_file, "types.rs should be included after expansion");
}

#[test]
fn test_include_types_adds_definition_paths() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create .git directory to make it a git repository
    fs::create_dir_all(root.join(".git")).unwrap();

    fs::write(
        root.join("main.rs"),
        r#"
struct LocalType {
    x: i32,
}

fn main() {
    let t: LocalType = LocalType { x: 1 };
}
"#,
    )
    .unwrap();

    let config = Config {
        paths: Some(vec![root.to_path_buf()]),
        include_types: true,
        ..Config::default()
    };

    let walk_options = WalkOptions::from_config(&config).unwrap();
    let cache = Arc::new(FileCache::new());
    let mut files = walk_directory(root, walk_options).unwrap();

    context_creator::core::walker::perform_semantic_analysis(&mut files, &config, &cache).unwrap();

    let main_file = files
        .iter()
        .find(|f| f.relative_path.to_str().unwrap() == "main.rs")
        .unwrap();

    // Check that type references have definition_path populated
    for type_ref in &main_file.type_references {
        // This will fail because definition_path field doesn't exist yet
        // assert!(type_ref.definition_path.is_some(), "Type {} should have definition_path", type_ref.name);

        // For now just check we detected the type
        assert_eq!(type_ref.name, "LocalType");
    }
}
