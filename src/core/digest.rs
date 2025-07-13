//! Markdown generation functionality

use crate::core::cache::FileCache;
use crate::core::walker::FileInfo;
use crate::utils::file_ext::FileType;
use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

/// Options for generating markdown digest
#[derive(Debug, Clone)]
pub struct DigestOptions {
    /// Maximum tokens allowed in the output
    pub max_tokens: Option<usize>,
    /// Include file tree in output
    pub include_tree: bool,
    /// Include token count statistics
    pub include_stats: bool,
    /// Group files by type
    pub group_by_type: bool,
    /// Sort files by priority
    pub sort_by_priority: bool,
    /// Template for file headers
    pub file_header_template: String,
    /// Template for the document header
    pub doc_header_template: String,
    /// Include table of contents
    pub include_toc: bool,
}

impl DigestOptions {
    /// Create DigestOptions from CLI config
    pub fn from_config(config: &crate::cli::Config) -> Result<Self> {
        Ok(DigestOptions {
            max_tokens: config.max_tokens,
            include_tree: true,
            include_stats: true,
            group_by_type: false,
            sort_by_priority: true,
            file_header_template: "## {path}".to_string(),
            doc_header_template: "# Code Digest: {directory}".to_string(),
            include_toc: true,
        })
    }
}

impl Default for DigestOptions {
    fn default() -> Self {
        DigestOptions {
            max_tokens: None,
            include_tree: true,
            include_stats: true,
            group_by_type: false,
            sort_by_priority: true,
            file_header_template: "## {path}".to_string(),
            doc_header_template: "# Code Digest: {directory}".to_string(),
            include_toc: true,
        }
    }
}

/// Estimate the total size of the markdown output
fn estimate_output_size(files: &[FileInfo], options: &DigestOptions, cache: &FileCache) -> usize {
    let mut size = 0;

    // Document header
    if !options.doc_header_template.is_empty() {
        size += options.doc_header_template.len() + 50; // Extra for replacements and newlines
    }

    // Statistics section
    if options.include_stats {
        size += 500; // Estimated size for stats
        size += files.len() * 50; // For file type listing
    }

    // File tree
    if options.include_tree {
        size += 100; // Headers
        size += files.len() * 100; // Estimated per-file in tree
    }

    // Table of contents
    if options.include_toc {
        size += 50; // Header
        size += files.len() * 100; // Per-file TOC entry
    }

    // File contents
    for file in files {
        // Header template
        size +=
            options.file_header_template.len() + file.relative_path.to_string_lossy().len() + 20;

        // File content + code fence
        if let Ok(content) = cache.get_or_load(&file.path) {
            size += content.len() + 20; // Content + fence markers
        } else {
            size += file.size as usize; // Fallback to file size
        }
    }

    // Add 20% buffer for formatting and unexpected overhead
    size + (size / 5)
}

/// Generate markdown from a list of files
pub fn generate_markdown(
    files: Vec<FileInfo>,
    options: DigestOptions,
    cache: Arc<FileCache>,
) -> Result<String> {
    // Pre-allocate string with estimated capacity
    let estimated_size = estimate_output_size(&files, &options, &cache);
    let mut output = String::with_capacity(estimated_size);

    // Add document header
    if !options.doc_header_template.is_empty() {
        let header = options.doc_header_template.replace("{directory}", ".");
        output.push_str(&header);
        output.push_str("\n\n");
    }

    // Add statistics if requested
    if options.include_stats {
        let stats = generate_statistics(&files);
        output.push_str(&stats);
        output.push_str("\n\n");
    }

    // Add file tree if requested
    if options.include_tree {
        let tree = generate_file_tree(&files);
        output.push_str("## File Structure\n\n");
        output.push_str("```\n");
        output.push_str(&tree);
        output.push_str("```\n\n");
    }

    // Sort files if requested
    let mut files = files;
    if options.sort_by_priority {
        files.sort_by(|a, b| {
            b.priority
                .partial_cmp(&a.priority)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.relative_path.cmp(&b.relative_path))
        });
    }

    // Add table of contents if requested
    if options.include_toc {
        output.push_str("## Table of Contents\n\n");
        for file in &files {
            let anchor = path_to_anchor(&file.relative_path);
            output.push_str(&format!(
                "- [{path}](#{anchor})\n",
                path = file.relative_path.display(),
                anchor = anchor
            ));
        }
        output.push('\n');
    }

    // Group files if requested
    if options.group_by_type {
        let grouped = group_files_by_type(files);
        for (file_type, group_files) in grouped {
            output.push_str(&format!("## {} Files\n\n", file_type_display(&file_type)));
            for file in group_files {
                append_file_content(&mut output, &file, &options, &cache)?;
            }
        }
    } else {
        // Add all files
        for file in files {
            append_file_content(&mut output, &file, &options, &cache)?;
        }
    }

    Ok(output)
}

/// Append a single file's content to the output
fn append_file_content(
    output: &mut String,
    file: &FileInfo,
    options: &DigestOptions,
    cache: &FileCache,
) -> Result<()> {
    // Read file content from cache
    let content = match cache.get_or_load(&file.path) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Warning: Could not read file {}: {}", file.path.display(), e);
            return Ok(());
        }
    };

    // Add file header
    let header =
        options.file_header_template.replace("{path}", &file.relative_path.display().to_string());
    output.push_str(&header);
    output.push_str("\n\n");

    // Add language hint for syntax highlighting
    let language = get_language_hint(&file.file_type);
    output.push_str(&format!("```{language}\n"));
    output.push_str(&content);
    if !content.ends_with('\n') {
        output.push('\n');
    }
    output.push_str("```\n\n");

    Ok(())
}

/// Generate statistics about the files
fn generate_statistics(files: &[FileInfo]) -> String {
    let total_files = files.len();
    let total_size: u64 = files.iter().map(|f| f.size).sum();

    // Count by file type
    let mut type_counts: HashMap<FileType, usize> = HashMap::new();
    for file in files {
        *type_counts.entry(file.file_type.clone()).or_insert(0) += 1;
    }

    // Pre-allocate with estimated capacity
    let mut stats = String::with_capacity(500 + type_counts.len() * 50);
    stats.push_str("## Statistics\n\n");
    stats.push_str(&format!("- Total files: {total_files}\n"));
    stats.push_str(&format!("- Total size: {} bytes\n", format_size(total_size)));
    stats.push_str("\n### Files by type:\n");

    let mut types: Vec<_> = type_counts.into_iter().collect();
    types.sort_by_key(|(_, count)| std::cmp::Reverse(*count));

    for (file_type, count) in types {
        stats.push_str(&format!("- {}: {}\n", file_type_display(&file_type), count));
    }

    stats
}

/// Generate a file tree representation
fn generate_file_tree(files: &[FileInfo]) -> String {
    use std::collections::BTreeMap;

    #[derive(Default)]
    struct TreeNode {
        files: Vec<String>,
        dirs: BTreeMap<String, TreeNode>,
    }

    let mut root = TreeNode::default();

    // Build tree structure
    for file in files {
        let parts: Vec<_> = file
            .relative_path
            .components()
            .map(|c| c.as_os_str().to_string_lossy().to_string())
            .collect();

        let mut current = &mut root;
        for (i, part) in parts.iter().enumerate() {
            if i == parts.len() - 1 {
                // File
                current.files.push(part.clone());
            } else {
                // Directory
                current = current.dirs.entry(part.clone()).or_default();
            }
        }
    }

    // Render tree
    fn render_tree(node: &TreeNode, prefix: &str, _is_last: bool) -> String {
        // Pre-allocate with estimated size
        let estimated_size = (node.dirs.len() + node.files.len()) * 100;
        let mut output = String::with_capacity(estimated_size);

        // Render directories
        let dir_count = node.dirs.len();
        for (i, (name, child)) in node.dirs.iter().enumerate() {
            let is_last_dir = i == dir_count - 1 && node.files.is_empty();
            let connector = if is_last_dir { "└── " } else { "├── " };
            let extension = if is_last_dir { "    " } else { "│   " };

            output.push_str(&format!("{prefix}{connector}{name}/\n"));
            output.push_str(&render_tree(child, &format!("{prefix}{extension}"), is_last_dir));
        }

        // Render files
        let file_count = node.files.len();
        for (i, name) in node.files.iter().enumerate() {
            let is_last_file = i == file_count - 1;
            let connector = if is_last_file { "└── " } else { "├── " };
            output.push_str(&format!("{prefix}{connector}{name}\n"));
        }

        output
    }

    // Pre-allocate output string
    let mut output = String::with_capacity(files.len() * 100 + 10);
    output.push_str(".\n");
    output.push_str(&render_tree(&root, "", true));
    output
}

/// Group files by their type
fn group_files_by_type(files: Vec<FileInfo>) -> Vec<(FileType, Vec<FileInfo>)> {
    let mut groups: HashMap<FileType, Vec<FileInfo>> = HashMap::new();

    for file in files {
        groups.entry(file.file_type.clone()).or_default().push(file);
    }

    let mut result: Vec<_> = groups.into_iter().collect();
    result.sort_by_key(|(file_type, _)| file_type_priority(file_type));
    result
}

/// Get display name for file type
fn file_type_display(file_type: &FileType) -> &'static str {
    match file_type {
        FileType::Rust => "Rust",
        FileType::Python => "Python",
        FileType::JavaScript => "JavaScript",
        FileType::TypeScript => "TypeScript",
        FileType::Go => "Go",
        FileType::Java => "Java",
        FileType::Cpp => "C++",
        FileType::C => "C",
        FileType::CSharp => "C#",
        FileType::Ruby => "Ruby",
        FileType::Php => "PHP",
        FileType::Swift => "Swift",
        FileType::Kotlin => "Kotlin",
        FileType::Scala => "Scala",
        FileType::Haskell => "Haskell",
        FileType::Markdown => "Markdown",
        FileType::Json => "JSON",
        FileType::Yaml => "YAML",
        FileType::Toml => "TOML",
        FileType::Xml => "XML",
        FileType::Html => "HTML",
        FileType::Css => "CSS",
        FileType::Text => "Text",
        FileType::Other => "Other",
    }
}

/// Get language hint for syntax highlighting
fn get_language_hint(file_type: &FileType) -> &'static str {
    match file_type {
        FileType::Rust => "rust",
        FileType::Python => "python",
        FileType::JavaScript => "javascript",
        FileType::TypeScript => "typescript",
        FileType::Go => "go",
        FileType::Java => "java",
        FileType::Cpp => "cpp",
        FileType::C => "c",
        FileType::CSharp => "csharp",
        FileType::Ruby => "ruby",
        FileType::Php => "php",
        FileType::Swift => "swift",
        FileType::Kotlin => "kotlin",
        FileType::Scala => "scala",
        FileType::Haskell => "haskell",
        FileType::Markdown => "markdown",
        FileType::Json => "json",
        FileType::Yaml => "yaml",
        FileType::Toml => "toml",
        FileType::Xml => "xml",
        FileType::Html => "html",
        FileType::Css => "css",
        FileType::Text => "text",
        FileType::Other => "",
    }
}

/// Get priority for file type ordering
fn file_type_priority(file_type: &FileType) -> u8 {
    match file_type {
        FileType::Rust => 1,
        FileType::Python => 2,
        FileType::JavaScript => 3,
        FileType::TypeScript => 3,
        FileType::Go => 4,
        FileType::Java => 5,
        FileType::Cpp => 6,
        FileType::C => 7,
        FileType::CSharp => 8,
        FileType::Ruby => 9,
        FileType::Php => 10,
        FileType::Swift => 11,
        FileType::Kotlin => 12,
        FileType::Scala => 13,
        FileType::Haskell => 14,
        FileType::Markdown => 15,
        FileType::Json => 16,
        FileType::Yaml => 17,
        FileType::Toml => 18,
        FileType::Xml => 19,
        FileType::Html => 20,
        FileType::Css => 21,
        FileType::Text => 22,
        FileType::Other => 23,
    }
}

/// Convert path to anchor-friendly string
fn path_to_anchor(path: &Path) -> String {
    path.display().to_string().replace(['/', '\\', '.', ' '], "-").to_lowercase()
}

/// Format file size in human-readable format
fn format_size(size: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut size = size as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", size as u64, UNITS[unit_index])
    } else {
        format!("{:.2} {}", size, UNITS[unit_index])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn create_test_cache() -> Arc<FileCache> {
        Arc::new(FileCache::new())
    }

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(1024), "1.00 KB");
        assert_eq!(format_size(1536), "1.50 KB");
        assert_eq!(format_size(1048576), "1.00 MB");
    }

    #[test]
    fn test_path_to_anchor() {
        assert_eq!(path_to_anchor(Path::new("src/main.rs")), "src-main-rs");
        assert_eq!(path_to_anchor(Path::new("test file.txt")), "test-file-txt");
    }

    #[test]
    fn test_file_type_display() {
        assert_eq!(file_type_display(&FileType::Rust), "Rust");
        assert_eq!(file_type_display(&FileType::Python), "Python");
    }

    #[test]
    fn test_generate_statistics() {
        let files = vec![
            FileInfo {
                path: PathBuf::from("test1.rs"),
                relative_path: PathBuf::from("test1.rs"),
                size: 100,
                file_type: FileType::Rust,
                priority: 1.0,
            },
            FileInfo {
                path: PathBuf::from("test2.py"),
                relative_path: PathBuf::from("test2.py"),
                size: 200,
                file_type: FileType::Python,
                priority: 0.9,
            },
        ];

        let stats = generate_statistics(&files);
        assert!(stats.contains("Total files: 2"));
        assert!(stats.contains("Total size: 300 B"));
        assert!(stats.contains("Rust: 1"));
        assert!(stats.contains("Python: 1"));
    }

    #[test]
    fn test_generate_statistics_empty() {
        let files = vec![];
        let stats = generate_statistics(&files);
        assert!(stats.contains("Total files: 0"));
        assert!(stats.contains("Total size: 0 B"));
    }

    #[test]
    fn test_generate_statistics_large_files() {
        let files = vec![
            FileInfo {
                path: PathBuf::from("large.rs"),
                relative_path: PathBuf::from("large.rs"),
                size: 2_000_000, // 2MB
                file_type: FileType::Rust,
                priority: 1.0,
            },
            FileInfo {
                path: PathBuf::from("huge.py"),
                relative_path: PathBuf::from("huge.py"),
                size: 50_000_000, // 50MB
                file_type: FileType::Python,
                priority: 0.9,
            },
        ];

        let stats = generate_statistics(&files);
        assert!(stats.contains("Total files: 2"));
        assert!(stats.contains("MB bytes")); // Just check that it's in MB
        assert!(stats.contains("Python: 1"));
        assert!(stats.contains("Rust: 1"));
    }

    #[test]
    fn test_generate_file_tree_with_grouping() {
        let files = vec![
            FileInfo {
                path: PathBuf::from("src/main.rs"),
                relative_path: PathBuf::from("src/main.rs"),
                size: 1000,
                file_type: FileType::Rust,
                priority: 1.5,
            },
            FileInfo {
                path: PathBuf::from("src/lib.rs"),
                relative_path: PathBuf::from("src/lib.rs"),
                size: 2000,
                file_type: FileType::Rust,
                priority: 1.2,
            },
            FileInfo {
                path: PathBuf::from("tests/test.rs"),
                relative_path: PathBuf::from("tests/test.rs"),
                size: 500,
                file_type: FileType::Rust,
                priority: 0.8,
            },
        ];

        let tree = generate_file_tree(&files);
        assert!(tree.contains("src/"));
        assert!(tree.contains("tests/"));
        assert!(tree.contains("main.rs"));
        assert!(tree.contains("lib.rs"));
        assert!(tree.contains("test.rs"));
    }

    #[test]
    fn test_digest_options_from_config() {
        use crate::cli::Config;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let config = Config {
            prompt: None,
            prompt_flag: None,
            directories: vec![temp_dir.path().to_path_buf()],
            directories_positional: vec![],
            output_file: None,
            max_tokens: Some(100000),
            llm_tool: crate::cli::LlmTool::default(),
            quiet: false,
            verbose: false,
            config: None,
            progress: false,
            repo: None,
            read_stdin: false,
            copy: false,
            custom_priorities: vec![],
        };

        let options = DigestOptions::from_config(&config).unwrap();
        assert_eq!(options.max_tokens, Some(100000));
        assert!(options.include_tree);
        assert!(options.include_stats);
        assert!(!options.group_by_type); // Default is false according to implementation
    }

    #[test]
    fn test_generate_markdown_structure_headers() {
        let files = vec![];

        let options = DigestOptions {
            max_tokens: None,
            include_tree: true,
            include_stats: true,
            group_by_type: true,
            sort_by_priority: true,
            file_header_template: "## {path}".to_string(),
            doc_header_template: "# Code Digest".to_string(),
            include_toc: true,
        };

        let cache = create_test_cache();
        let markdown = generate_markdown(files, options, cache).unwrap();

        // Check that main structure is present even with no files
        assert!(markdown.contains("# Code Digest"));
        assert!(markdown.contains("## Statistics"));
    }
}
