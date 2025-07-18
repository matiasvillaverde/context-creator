//! Integration test for --include-types using the full processing flow

use context_creator::{run, Config};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_include_types_full_flow() {
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

    // Create type definition in separate file
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

    // Create output file path
    let output_file = root.join("output.md");

    // Create config
    let config = Config {
        paths: Some(vec![root.to_path_buf()]),
        include: Some(vec!["main.rs".to_string()]),
        output_file: Some(output_file.clone()),
        include_types: true,
        semantic_depth: 2,
        progress: false,
        quiet: true,
        ..Default::default()
    };

    // Run the full flow
    run(config).unwrap();

    // Read the output
    let output = fs::read_to_string(&output_file).unwrap();

    // Check that both files are included
    assert!(output.contains("main.rs"), "Output should contain main.rs");
    assert!(
        output.contains("types.rs"),
        "Output should contain types.rs - type definition file should be included"
    );

    // Check that MyType definition is present
    assert!(
        output.contains("struct MyType"),
        "Type definition should be present in output"
    );
}
