//! Context creation functionality for LLM consumption

use crate::cli::OutputFormat;
use crate::core::cache::FileCache;
use crate::core::walker::FileInfo;
use crate::formatters::{create_formatter, DigestData};
use crate::utils::file_ext::FileType;
use crate::utils::git::get_file_git_context;
use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::warn;

/// Options for generating context for LLM consumption
#[derive(Debug, Clone)]
pub struct ContextOptions {
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
    /// Enable enhanced context with file metadata
    pub enhanced_context: bool,
    /// Include git commit history in file headers
    pub git_context: bool,
}

impl ContextOptions {
    /// Create ContextOptions from CLI config
    pub fn from_config(config: &crate::cli::Config) -> Result<Self> {
        Ok(ContextOptions {
            max_tokens: config.get_effective_context_tokens(),
            include_tree: true,
            include_stats: true,
            group_by_type: false,
            sort_by_priority: true,
            file_header_template: "## {path}".to_string(),
            doc_header_template: "# Code Context: {directory}".to_string(),
            include_toc: true,
            enhanced_context: config.enhanced_context,
            git_context: config.git_context,
        })
    }
}

impl Default for ContextOptions {
    fn default() -> Self {
        ContextOptions {
            max_tokens: None,
            include_tree: true,
            include_stats: true,
            group_by_type: false,
            sort_by_priority: true,
            file_header_template: "## {path}".to_string(),
            doc_header_template: "# Code Context: {directory}".to_string(),
            include_toc: true,
            enhanced_context: false,
            git_context: false,
        }
    }
}

/// Estimate the total size of the markdown output
fn estimate_output_size(files: &[FileInfo], options: &ContextOptions, cache: &FileCache) -> usize {
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
    options: ContextOptions,
    cache: Arc<FileCache>,
) -> Result<String> {
    let mut output = create_output_buffer(&files, &options, &cache);

    add_document_header(&mut output, &options);
    add_statistics_section(&mut output, &files, &options);
    add_file_tree_section(&mut output, &files, &options);

    let sorted_files = sort_files_by_priority(files, &options);
    add_table_of_contents(&mut output, &sorted_files, &options);
    add_file_contents(&mut output, sorted_files, &options, &cache)?;

    Ok(output)
}

// Helper functions - each 10 lines or less

fn create_output_buffer(
    files: &[FileInfo],
    options: &ContextOptions,
    cache: &Arc<FileCache>,
) -> String {
    let estimated_size = estimate_output_size(files, options, cache);
    String::with_capacity(estimated_size)
}

fn add_document_header(output: &mut String, options: &ContextOptions) {
    if !options.doc_header_template.is_empty() {
        let header = options.doc_header_template.replace("{directory}", ".");
        output.push_str(&header);
        output.push_str("\n\n");
    }
}

fn add_statistics_section(output: &mut String, files: &[FileInfo], options: &ContextOptions) {
    if options.include_stats {
        let stats = generate_statistics(files);
        output.push_str(&stats);
        output.push_str("\n\n");
    }
}

fn add_file_tree_section(output: &mut String, files: &[FileInfo], options: &ContextOptions) {
    if options.include_tree {
        let tree = generate_file_tree(files, options);
        output.push_str("## File Structure\n\n");
        output.push_str("```\n");
        output.push_str(&tree);
        output.push_str("```\n\n");
    }
}

fn sort_files_by_priority(mut files: Vec<FileInfo>, options: &ContextOptions) -> Vec<FileInfo> {
    if options.sort_by_priority {
        files.sort_by(|a, b| {
            b.priority
                .partial_cmp(&a.priority)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.relative_path.cmp(&b.relative_path))
        });
    }
    files
}

fn add_table_of_contents(output: &mut String, files: &[FileInfo], options: &ContextOptions) {
    if options.include_toc {
        output.push_str("## Table of Contents\n\n");
        for file in files {
            add_toc_entry(output, file);
        }
        output.push('\n');
    }
}

fn add_toc_entry(output: &mut String, file: &FileInfo) {
    let anchor = path_to_anchor(&file.relative_path);
    output.push_str(&format!(
        "- [{path}](#{anchor})\n",
        path = file.relative_path.display(),
        anchor = anchor
    ));
}

fn add_file_contents(
    output: &mut String,
    files: Vec<FileInfo>,
    options: &ContextOptions,
    cache: &Arc<FileCache>,
) -> Result<()> {
    if options.group_by_type {
        add_grouped_files(output, files, options, cache)
    } else {
        add_ungrouped_files(output, files, options, cache)
    }
}

fn add_grouped_files(
    output: &mut String,
    files: Vec<FileInfo>,
    options: &ContextOptions,
    cache: &Arc<FileCache>,
) -> Result<()> {
    let grouped = group_files_by_type(files);
    for (file_type, group_files) in grouped {
        output.push_str(&format!("## {} Files\n\n", file_type_display(&file_type)));
        for file in group_files {
            append_file_content(output, &file, options, cache)?;
        }
    }
    Ok(())
}

fn add_ungrouped_files(
    output: &mut String,
    files: Vec<FileInfo>,
    options: &ContextOptions,
    cache: &Arc<FileCache>,
) -> Result<()> {
    for file in files {
        append_file_content(output, &file, options, cache)?;
    }
    Ok(())
}

/// Generate digest using the appropriate formatter
pub fn generate_digest(
    files: Vec<FileInfo>,
    options: ContextOptions,
    cache: Arc<FileCache>,
    output_format: OutputFormat,
    base_directory: &str,
) -> Result<String> {
    // Create formatter based on output format
    let mut formatter = create_formatter(output_format);

    // Create digest data
    let data = DigestData {
        files: &files,
        options: &options,
        cache: &cache,
        base_directory,
    };

    // Render all sections
    formatter.render_header(&data)?;
    formatter.render_statistics(&data)?;
    formatter.render_file_tree(&data)?;
    formatter.render_toc(&data)?;

    // Render file details
    for file in &files {
        formatter.render_file_details(file, &data)?;
    }

    // Finalize and return
    Ok(formatter.finalize())
}

/// Append a single file's content to the output
fn append_file_content(
    output: &mut String,
    file: &FileInfo,
    options: &ContextOptions,
    cache: &FileCache,
) -> Result<()> {
    let content = load_file_content(file, cache)?;
    add_file_header(output, file, options);
    add_semantic_info(output, file);
    add_file_body(output, &content, &file.file_type);
    Ok(())
}

fn load_file_content(file: &FileInfo, cache: &FileCache) -> Result<String> {
    match cache.get_or_load(&file.path) {
        Ok(content) => Ok(content.to_string()),
        Err(e) => {
            warn!("Could not read file {}: {}", file.path.display(), e);
            Ok(String::new())
        }
    }
}

fn add_file_header(output: &mut String, file: &FileInfo, options: &ContextOptions) {
    let path_with_metadata = format_path_with_metadata(file, options);
    let header = options
        .file_header_template
        .replace("{path}", &path_with_metadata);
    output.push_str(&header);
    output.push('\n');

    // Add git context if enabled
    if options.git_context {
        // Find the repository root from the file path
        let repo_root = file.path.parent().unwrap_or(Path::new("."));
        if let Some(git_context) = get_file_git_context(repo_root, &file.path) {
            if !git_context.recent_commits.is_empty() {
                output.push('\n');
                output.push_str("Git history:\n");
                for (i, commit) in git_context.recent_commits.iter().enumerate().take(3) {
                    if i > 0 {
                        output.push('\n');
                    }
                    output.push_str(&format!(
                        "  - {} by {}",
                        commit.message.trim(),
                        commit.author
                    ));
                }
                output.push('\n');
            }
        }
    }

    output.push('\n');
}

pub fn format_path_with_metadata(file: &FileInfo, options: &ContextOptions) -> String {
    if options.enhanced_context {
        format!(
            "{} ({}, {})",
            file.relative_path.display(),
            format_size(file.size),
            file_type_display(&file.file_type)
        )
    } else {
        file.relative_path.display().to_string()
    }
}

fn add_semantic_info(output: &mut String, file: &FileInfo) {
    add_imports_info(output, &file.imports);
    add_imported_by_info(output, &file.imported_by);
    add_function_calls_info(output, &file.function_calls);
    add_type_references_info(output, &file.type_references);
}

fn add_imports_info(output: &mut String, imports: &[PathBuf]) {
    if !imports.is_empty() {
        output.push_str("Imports: ");
        let names = format_import_names(imports);
        output.push_str(&format!("{}\n\n", names.join(", ")));
    }
}

pub fn format_import_names(imports: &[PathBuf]) -> Vec<String> {
    imports.iter().map(|p| format_import_name(p)).collect()
}

fn format_import_name(path: &Path) -> String {
    let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

    if filename == "__init__.py" {
        get_python_module_name(path)
    } else {
        strip_common_extensions(filename).to_string()
    }
}

fn get_python_module_name(path: &Path) -> String {
    path.parent()
        .and_then(|parent| parent.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string()
}

fn strip_common_extensions(filename: &str) -> &str {
    filename
        .strip_suffix(".py")
        .or_else(|| filename.strip_suffix(".rs"))
        .or_else(|| filename.strip_suffix(".js"))
        .or_else(|| filename.strip_suffix(".ts"))
        .unwrap_or(filename)
}

fn add_imported_by_info(output: &mut String, imported_by: &[PathBuf]) {
    if !imported_by.is_empty() {
        output.push_str("Imported by: ");
        let names = format_imported_by_names(imported_by);
        output.push_str(&format!("{}\n\n", names.join(", ")));
    }
}

pub fn format_imported_by_names(imported_by: &[PathBuf]) -> Vec<String> {
    imported_by
        .iter()
        .map(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_else(|| p.to_str().unwrap_or("unknown"))
                .to_string()
        })
        .collect()
}

fn add_function_calls_info(
    output: &mut String,
    calls: &[crate::core::semantic::analyzer::FunctionCall],
) {
    if !calls.is_empty() {
        output.push_str("Function calls: ");
        let names = format_function_call_names(calls);
        output.push_str(&format!("{}\n\n", names.join(", ")));
    }
}

fn format_function_call_names(
    calls: &[crate::core::semantic::analyzer::FunctionCall],
) -> Vec<String> {
    calls
        .iter()
        .map(|fc| {
            if let Some(module) = &fc.module {
                format!("{}.{}", module, fc.name)
            } else {
                fc.name.clone()
            }
        })
        .collect()
}

fn add_type_references_info(
    output: &mut String,
    refs: &[crate::core::semantic::analyzer::TypeReference],
) {
    if !refs.is_empty() {
        output.push_str("Type references: ");
        let names: Vec<String> = refs
            .iter()
            .map(|tr| {
                if let Some(module) = &tr.module {
                    // Check if module already ends with the type name to avoid duplication
                    if module.ends_with(&format!("::{}", tr.name)) {
                        module.clone()
                    } else {
                        format!("{}.{}", module, tr.name)
                    }
                } else {
                    tr.name.clone()
                }
            })
            .collect();
        output.push_str(&format!("{}\n\n", names.join(", ")));
    }
}

fn add_file_body(output: &mut String, content: &str, file_type: &FileType) {
    let language = get_language_hint(file_type);
    output.push_str(&format!("```{language}\n"));
    output.push_str(content);
    if !content.ends_with('\n') {
        output.push('\n');
    }
    output.push_str("```\n\n");
}

/// Generate statistics about the files
pub fn generate_statistics(files: &[FileInfo]) -> String {
    let mut stats = create_stats_buffer(files.len());
    add_stats_header(&mut stats);
    add_file_count(&mut stats, files.len());
    add_total_size(&mut stats, calculate_total_size(files));
    add_file_type_breakdown(&mut stats, count_file_types(files));
    stats
}

fn create_stats_buffer(file_count: usize) -> String {
    String::with_capacity(500 + file_count * 50)
}

fn add_stats_header(stats: &mut String) {
    stats.push_str("## Statistics\n\n");
}

fn add_file_count(stats: &mut String, count: usize) {
    stats.push_str(&format!("- Total files: {count}\n"));
}

fn add_total_size(stats: &mut String, size: u64) {
    stats.push_str(&format!("- Total size: {} bytes\n", format_size(size)));
}

fn calculate_total_size(files: &[FileInfo]) -> u64 {
    files.iter().map(|f| f.size).sum()
}

fn count_file_types(files: &[FileInfo]) -> Vec<(FileType, usize)> {
    let mut type_counts: HashMap<FileType, usize> = HashMap::new();
    for file in files {
        *type_counts.entry(file.file_type.clone()).or_insert(0) += 1;
    }
    let mut types: Vec<_> = type_counts.into_iter().collect();
    types.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
    types
}

fn add_file_type_breakdown(stats: &mut String, types: Vec<(FileType, usize)>) {
    stats.push_str("\n### Files by type:\n");
    for (file_type, count) in types {
        stats.push_str(&format!("- {}: {}\n", file_type_display(&file_type), count));
    }
}

/// Generate a file tree representation
pub fn generate_file_tree(files: &[FileInfo], options: &ContextOptions) -> String {
    use std::collections::{BTreeMap, HashMap};

    #[derive(Default)]
    struct TreeNode {
        files: Vec<String>,
        dirs: BTreeMap<String, TreeNode>,
    }

    let mut root = TreeNode::default();

    // Create a lookup map from relative path to FileInfo for metadata
    let file_lookup: HashMap<String, &FileInfo> = files
        .iter()
        .map(|f| (f.relative_path.to_string_lossy().to_string(), f))
        .collect();

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
    #[allow(clippy::too_many_arguments)]
    fn render_tree(
        node: &TreeNode,
        prefix: &str,
        _is_last: bool,
        current_path: &str,
        file_lookup: &HashMap<String, &FileInfo>,
        options: &ContextOptions,
    ) -> String {
        // Pre-allocate with estimated size
        let estimated_size = (node.dirs.len() + node.files.len()) * 100;
        let mut output = String::with_capacity(estimated_size);

        // Render directories
        let dir_count = node.dirs.len();
        for (i, (name, child)) in node.dirs.iter().enumerate() {
            let is_last_dir = i == dir_count - 1 && node.files.is_empty();
            let connector = if is_last_dir {
                "└── "
            } else {
                "├── "
            };
            let extension = if is_last_dir { "    " } else { "│   " };

            output.push_str(&format!("{prefix}{connector}{name}/\n"));
            let child_path = if current_path.is_empty() {
                name.clone()
            } else {
                format!("{current_path}/{name}")
            };
            output.push_str(&render_tree(
                child,
                &format!("{prefix}{extension}"),
                is_last_dir,
                &child_path,
                file_lookup,
                options,
            ));
        }

        // Render files
        let file_count = node.files.len();
        for (i, name) in node.files.iter().enumerate() {
            let is_last_file = i == file_count - 1;
            let connector = if is_last_file {
                "└── "
            } else {
                "├── "
            };

            let file_path = if current_path.is_empty() {
                name.clone()
            } else {
                format!("{current_path}/{name}")
            };

            // Include metadata if enhanced context is enabled
            let display_name = if options.enhanced_context {
                if let Some(file_info) = file_lookup.get(&file_path) {
                    format!(
                        "{} ({}, {})",
                        name,
                        format_size(file_info.size),
                        file_type_display(&file_info.file_type)
                    )
                } else {
                    name.clone()
                }
            } else {
                name.clone()
            };

            output.push_str(&format!("{prefix}{connector}{display_name}\n"));
        }

        output
    }

    // Pre-allocate output string
    let mut output = String::with_capacity(files.len() * 100 + 10);
    output.push_str(".\n");
    output.push_str(&render_tree(&root, "", true, "", &file_lookup, options));
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
pub fn file_type_display(file_type: &FileType) -> &'static str {
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
        FileType::Dart => "Dart",
        FileType::Lua => "Lua",
        FileType::R => "R",
        FileType::Julia => "Julia",
        FileType::Elixir => "Elixir",
        FileType::Elm => "Elm",
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
pub fn get_language_hint(file_type: &FileType) -> &'static str {
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
        FileType::Dart => "dart",
        FileType::Lua => "lua",
        FileType::R => "r",
        FileType::Julia => "julia",
        FileType::Elixir => "elixir",
        FileType::Elm => "elm",
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
        FileType::Dart => 15,
        FileType::Lua => 16,
        FileType::R => 17,
        FileType::Julia => 18,
        FileType::Elixir => 19,
        FileType::Elm => 20,
        FileType::Markdown => 21,
        FileType::Json => 22,
        FileType::Yaml => 23,
        FileType::Toml => 24,
        FileType::Xml => 25,
        FileType::Html => 26,
        FileType::Css => 27,
        FileType::Text => 28,
        FileType::Other => 29,
    }
}

/// Convert path to anchor-friendly string
pub fn path_to_anchor(path: &Path) -> String {
    path.display()
        .to_string()
        .replace(['/', '\\', '.', ' '], "-")
        .to_lowercase()
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
                imports: Vec::new(),
                imported_by: Vec::new(),
                function_calls: Vec::new(),
                type_references: Vec::new(),
                exported_functions: Vec::new(),
            },
            FileInfo {
                path: PathBuf::from("test2.py"),
                relative_path: PathBuf::from("test2.py"),
                size: 200,
                file_type: FileType::Python,
                priority: 0.9,
                imports: Vec::new(),
                imported_by: Vec::new(),
                function_calls: Vec::new(),
                type_references: Vec::new(),
                exported_functions: Vec::new(),
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
                imports: Vec::new(),
                imported_by: Vec::new(),
                function_calls: Vec::new(),
                type_references: Vec::new(),
                exported_functions: Vec::new(),
            },
            FileInfo {
                path: PathBuf::from("huge.py"),
                relative_path: PathBuf::from("huge.py"),
                size: 50_000_000, // 50MB
                file_type: FileType::Python,
                priority: 0.9,
                imports: Vec::new(),
                imported_by: Vec::new(),
                function_calls: Vec::new(),
                type_references: Vec::new(),
                exported_functions: Vec::new(),
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
                imports: Vec::new(),
                imported_by: Vec::new(),
                function_calls: Vec::new(),
                type_references: Vec::new(),
                exported_functions: Vec::new(),
            },
            FileInfo {
                path: PathBuf::from("src/lib.rs"),
                relative_path: PathBuf::from("src/lib.rs"),
                size: 2000,
                file_type: FileType::Rust,
                priority: 1.2,
                imports: Vec::new(),
                imported_by: Vec::new(),
                function_calls: Vec::new(),
                type_references: Vec::new(),
                exported_functions: Vec::new(),
            },
            FileInfo {
                path: PathBuf::from("tests/test.rs"),
                relative_path: PathBuf::from("tests/test.rs"),
                size: 500,
                file_type: FileType::Rust,
                priority: 0.8,
                imports: Vec::new(),
                imported_by: Vec::new(),
                function_calls: Vec::new(),
                type_references: Vec::new(),
                exported_functions: Vec::new(),
            },
        ];

        let options = ContextOptions::default();
        let tree = generate_file_tree(&files, &options);
        assert!(tree.contains("src/"));
        assert!(tree.contains("tests/"));
        assert!(tree.contains("main.rs"));
        assert!(tree.contains("lib.rs"));
        assert!(tree.contains("test.rs"));
    }

    #[test]
    fn test_context_options_from_config() {
        use crate::cli::Config;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let config = Config {
            paths: Some(vec![temp_dir.path().to_path_buf()]),
            max_tokens: Some(100000),
            ..Config::default()
        };

        let options = ContextOptions::from_config(&config).unwrap();
        assert_eq!(options.max_tokens, Some(100000));
        assert!(options.include_tree);
        assert!(options.include_stats);
        assert!(!options.group_by_type); // Default is false according to implementation
    }

    #[test]
    fn test_generate_markdown_structure_headers() {
        let files = vec![];

        let options = ContextOptions {
            max_tokens: None,
            include_tree: true,
            include_stats: true,
            group_by_type: true,
            sort_by_priority: true,
            file_header_template: "## {path}".to_string(),
            doc_header_template: "# Code Context".to_string(),
            include_toc: true,
            enhanced_context: false,
            git_context: false,
        };

        let cache = create_test_cache();
        let markdown = generate_markdown(files, options, cache).unwrap();

        // Check that main structure is present even with no files
        assert!(markdown.contains("# Code Context"));
        assert!(markdown.contains("## Statistics"));
    }

    #[test]
    fn test_enhanced_tree_generation_with_metadata() {
        use crate::core::walker::FileInfo;
        use crate::utils::file_ext::FileType;
        use std::path::PathBuf;

        let files = vec![
            FileInfo {
                path: PathBuf::from("src/main.rs"),
                relative_path: PathBuf::from("src/main.rs"),
                size: 145,
                file_type: FileType::Rust,
                priority: 1.5,
                imports: Vec::new(),
                imported_by: Vec::new(),
                function_calls: Vec::new(),
                type_references: Vec::new(),
                exported_functions: Vec::new(),
            },
            FileInfo {
                path: PathBuf::from("src/lib.rs"),
                relative_path: PathBuf::from("src/lib.rs"),
                size: 89,
                file_type: FileType::Rust,
                priority: 1.2,
                imports: Vec::new(),
                imported_by: Vec::new(),
                function_calls: Vec::new(),
                type_references: Vec::new(),
                exported_functions: Vec::new(),
            },
        ];

        let options = ContextOptions {
            max_tokens: None,
            include_tree: true,
            include_stats: true,
            group_by_type: false,
            sort_by_priority: true,
            file_header_template: "## {path}".to_string(),
            doc_header_template: "# Code Context".to_string(),
            include_toc: true,
            enhanced_context: true,
            git_context: false,
        };

        let cache = create_test_cache();
        let markdown = generate_markdown(files, options, cache).unwrap();

        // Should include file sizes and types in tree
        assert!(markdown.contains("main.rs (145 B, Rust)"));
        assert!(markdown.contains("lib.rs (89 B, Rust)"));
    }

    #[test]
    fn test_enhanced_file_headers_with_metadata() {
        use crate::core::walker::FileInfo;
        use crate::utils::file_ext::FileType;
        use std::path::PathBuf;

        let files = vec![FileInfo {
            path: PathBuf::from("src/main.rs"),
            relative_path: PathBuf::from("src/main.rs"),
            size: 145,
            file_type: FileType::Rust,
            priority: 1.5,
            imports: Vec::new(),
            imported_by: Vec::new(),
            function_calls: Vec::new(),
            type_references: Vec::new(),
            exported_functions: Vec::new(),
        }];

        let options = ContextOptions {
            max_tokens: None,
            include_tree: true,
            include_stats: true,
            group_by_type: false,
            sort_by_priority: true,
            file_header_template: "## {path}".to_string(),
            doc_header_template: "# Code Context".to_string(),
            include_toc: true,
            enhanced_context: true,
            git_context: false,
        };

        let cache = create_test_cache();
        let markdown = generate_markdown(files, options, cache).unwrap();

        // Should include metadata in file headers
        assert!(markdown.contains("## src/main.rs (145 B, Rust)"));
    }

    #[test]
    fn test_basic_mode_unchanged() {
        use crate::core::walker::FileInfo;
        use crate::utils::file_ext::FileType;
        use std::path::PathBuf;

        let files = vec![FileInfo {
            path: PathBuf::from("src/main.rs"),
            relative_path: PathBuf::from("src/main.rs"),
            size: 145,
            file_type: FileType::Rust,
            priority: 1.5,
            imports: Vec::new(),
            imported_by: Vec::new(),
            function_calls: Vec::new(),
            type_references: Vec::new(),
            exported_functions: Vec::new(),
        }];

        let options = ContextOptions {
            max_tokens: None,
            include_tree: true,
            include_stats: true,
            group_by_type: false,
            sort_by_priority: true,
            file_header_template: "## {path}".to_string(),
            doc_header_template: "# Code Context".to_string(),
            include_toc: true,
            enhanced_context: false,
            git_context: false,
        };

        let cache = create_test_cache();
        let markdown = generate_markdown(files, options, cache).unwrap();

        // Should NOT include metadata - backward compatibility
        assert!(markdown.contains("## src/main.rs"));
        assert!(!markdown.contains("## src/main.rs (145 B, Rust)"));
        assert!(markdown.contains("main.rs") && !markdown.contains("main.rs (145 B, Rust)"));
    }
}
