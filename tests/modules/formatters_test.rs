//! Tests for the formatters module

use context_creator::cli::OutputFormat;
use context_creator::core::cache::FileCache;
use context_creator::core::context_builder::ContextOptions;
use context_creator::core::walker::FileInfo;
use context_creator::formatters::{create_formatter, DigestData};
use context_creator::utils::file_ext::FileType;
use std::path::PathBuf;
use std::sync::Arc;

#[test]
fn test_create_formatter_returns_correct_type() {
    let markdown_formatter = create_formatter(OutputFormat::Markdown);
    assert!(markdown_formatter.format_name() == "markdown");

    let xml_formatter = create_formatter(OutputFormat::Xml);
    assert!(xml_formatter.format_name() == "xml");

    let plain_formatter = create_formatter(OutputFormat::Plain);
    assert!(plain_formatter.format_name() == "plain");

    let paths_formatter = create_formatter(OutputFormat::Paths);
    assert!(paths_formatter.format_name() == "paths");
}

#[test]
fn test_digest_data_creation() {
    let files = vec![FileInfo {
        path: PathBuf::from("test.rs"),
        relative_path: PathBuf::from("test.rs"),
        size: 100,
        file_type: FileType::Rust,
        priority: 1.0,
        imports: vec![],
        imported_by: vec![],
        function_calls: vec![],
        type_references: vec![],
        exported_functions: vec![],
    }];

    let options = ContextOptions::default();
    let cache = Arc::new(FileCache::new());

    let digest_data = DigestData {
        files: &files,
        options: &options,
        cache: &cache,
        base_directory: ".",
    };

    assert_eq!(digest_data.files.len(), 1);
    assert_eq!(digest_data.base_directory, ".");
}

#[test]
fn test_formatter_trait_methods() {
    // Test that each formatter properly implements the required methods
    let mut formatter = create_formatter(OutputFormat::Markdown);
    let files = vec![];
    let options = ContextOptions::default();
    let cache = Arc::new(FileCache::new());

    let data = DigestData {
        files: &files,
        options: &options,
        cache: &cache,
        base_directory: ".",
    };

    // These should not panic
    assert!(formatter.render_header(&data).is_ok());
    assert!(formatter.render_statistics(&data).is_ok());
    assert!(formatter.render_file_tree(&data).is_ok());
    assert!(formatter.render_toc(&data).is_ok());

    // Can't call finalize on trait object directly
    assert!(formatter.format_name() == "markdown");
}

#[test]
fn test_paths_formatter_only_outputs_paths() {
    use context_creator::formatters::paths::PathsFormatter;
    use context_creator::formatters::DigestFormatter;

    let mut formatter = PathsFormatter::new();

    let files = vec![
        FileInfo {
            path: PathBuf::from("/full/path/to/file1.rs"),
            relative_path: PathBuf::from("file1.rs"),
            size: 100,
            file_type: FileType::Rust,
            priority: 1.0,
            imports: vec![],
            imported_by: vec![],
            function_calls: vec![],
            type_references: vec![],
            exported_functions: vec![],
        },
        FileInfo {
            path: PathBuf::from("/full/path/to/file2.rs"),
            relative_path: PathBuf::from("file2.rs"),
            size: 200,
            file_type: FileType::Rust,
            priority: 0.9,
            imports: vec![],
            imported_by: vec![],
            function_calls: vec![],
            type_references: vec![],
            exported_functions: vec![],
        },
    ];

    let options = ContextOptions::default();
    let cache = Arc::new(FileCache::new());

    let data = DigestData {
        files: &files,
        options: &options,
        cache: &cache,
        base_directory: ".",
    };

    // PathsFormatter should ignore most methods
    let _ = formatter.render_header(&data);
    let _ = formatter.render_statistics(&data);
    let _ = formatter.render_file_tree(&data);
    let _ = formatter.render_toc(&data);

    // Only render files
    for file in &files {
        let _ = formatter.render_file_details(file, &data);
    }

    let output = Box::new(formatter).finalize();

    // Should only contain file paths, one per line
    assert!(output.contains("file1.rs"));
    assert!(output.contains("file2.rs"));
    assert!(!output.contains("100")); // No file size
    assert!(!output.contains("Rust")); // No file type
    assert!(!output.contains("#")); // No markdown headers
    assert!(!output.contains("```")); // No code blocks
}
