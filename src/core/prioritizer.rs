//! File prioritization based on token limits

use crate::core::cache::FileCache;
use crate::core::digest::DigestOptions;
use crate::core::token::{would_exceed_limit, TokenCounter};
use crate::core::walker::FileInfo;
use anyhow::Result;
use rayon::prelude::*;
use std::sync::Arc;

/// File with pre-computed token count
#[derive(Debug, Clone)]
struct FileWithTokens {
    file: FileInfo,
    token_count: usize,
}

/// Prioritize files based on their importance and token limits
pub fn prioritize_files(
    mut files: Vec<FileInfo>,
    options: &DigestOptions,
    cache: Arc<FileCache>,
) -> Result<Vec<FileInfo>> {
    // If no token limit, return all files sorted by priority
    let max_tokens = match options.max_tokens {
        Some(limit) => limit,
        None => {
            files.sort_by(|a, b| {
                b.priority
                    .partial_cmp(&a.priority)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.relative_path.cmp(&b.relative_path))
            });
            return Ok(files);
        }
    };

    // Create token counter
    let counter = TokenCounter::new()?;

    // Calculate overhead for markdown structure
    let structure_overhead = calculate_structure_overhead(options, &files)?;

    // Phase 1: Count tokens for all files in parallel with proper error handling
    let results: Vec<crate::utils::error::Result<FileWithTokens>> = files
        .into_par_iter()
        .map(|file| {
            // Read file content from cache
            let content = cache.get_or_load(&file.path).map_err(|e| {
                crate::utils::error::CodeDigestError::FileProcessingError {
                    path: file.path.display().to_string(),
                    error: format!("Could not read file: {}", e),
                }
            })?;

            // Count tokens for this file
            let file_tokens = counter
                .count_file_tokens(&content, &file.relative_path.to_string_lossy())
                .map_err(|e| crate::utils::error::CodeDigestError::TokenCountingError {
                    path: file.path.display().to_string(),
                    error: e.to_string(),
                })?;

            Ok(FileWithTokens { file, token_count: file_tokens.total_tokens })
        })
        .collect();

    // Use partition_result to separate successes from errors
    use itertools::Itertools;
    let (files_with_tokens, errors): (Vec<_>, Vec<_>) = results.into_iter().partition_result();

    // Log errors without failing the entire operation
    if !errors.is_empty() {
        eprintln!("Warning: {} files could not be processed for token counting:", errors.len());
        for error in &errors {
            eprintln!("  {}", error);
        }
    }

    // Phase 2: Sort by priority and select files sequentially
    let mut files_with_tokens = files_with_tokens;
    files_with_tokens.sort_by(|a, b| {
        b.file
            .priority
            .partial_cmp(&a.file.priority)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.file.relative_path.cmp(&b.file.relative_path))
    });

    let mut selected_files = Vec::new();
    let mut total_tokens = structure_overhead;

    // Select files until we hit the token limit
    for file_with_tokens in files_with_tokens {
        // Check if adding this file would exceed the limit
        if would_exceed_limit(total_tokens, file_with_tokens.token_count, max_tokens) {
            // Try to find smaller files that might fit
            continue;
        }

        // Add the file
        total_tokens += file_with_tokens.token_count;
        selected_files.push(file_with_tokens.file);
    }

    // Log statistics
    if options.include_stats {
        eprintln!("Token limit: {max_tokens}");
        eprintln!("Structure overhead: {structure_overhead} tokens");
        eprintln!(
            "Selected {} files with approximately {} tokens",
            selected_files.len(),
            total_tokens
        );
    }

    Ok(selected_files)
}

/// Calculate token overhead for markdown structure
fn calculate_structure_overhead(options: &DigestOptions, files: &[FileInfo]) -> Result<usize> {
    let counter = TokenCounter::new()?;
    let mut overhead = 0;

    // Document header
    if !options.doc_header_template.is_empty() {
        let header = options.doc_header_template.replace("{directory}", ".");
        overhead += counter.count_tokens(&format!("{header}\n\n"))?;
    }

    // Statistics section
    if options.include_stats {
        // Estimate statistics section size
        let stats_estimate = format!(
            "## Statistics\n\n- Total files: {}\n- Total size: X bytes\n\n### Files by type:\n",
            files.len()
        );
        overhead += counter.count_tokens(&stats_estimate)?;
        overhead += 200; // Buffer for file type list
    }

    // File tree
    if options.include_tree {
        overhead += counter.count_tokens("## File Structure\n\n```\n")?;
        // Estimate tree size (rough approximation)
        overhead += files.len() * 20; // ~20 tokens per file in tree
        overhead += counter.count_tokens("```\n\n")?;
    }

    // Table of contents
    if options.include_toc {
        overhead += counter.count_tokens("## Table of Contents\n\n")?;
        for file in files {
            let toc_line = format!("- [{}](#anchor)\n", file.relative_path.display());
            overhead += counter.count_tokens(&toc_line)?;
        }
        overhead += counter.count_tokens("\n")?;
    }

    Ok(overhead)
}

/// Group files by directory for better organization
pub fn group_by_directory(files: Vec<FileInfo>) -> Vec<(String, Vec<FileInfo>)> {
    use std::collections::HashMap;

    let mut groups: HashMap<String, Vec<FileInfo>> = HashMap::new();

    for file in files {
        let dir = file
            .relative_path
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| ".".to_string());

        groups.entry(dir).or_default().push(file);
    }

    let mut result: Vec<_> = groups.into_iter().collect();
    result.sort_by(|a, b| a.0.cmp(&b.0));

    // Sort files within each group by priority
    for (_, files) in &mut result {
        files.sort_by(|a, b| {
            b.priority
                .partial_cmp(&a.priority)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.relative_path.cmp(&b.relative_path))
        });
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::file_ext::FileType;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn create_test_cache() -> Arc<FileCache> {
        Arc::new(FileCache::new())
    }

    fn create_test_files(_temp_dir: &TempDir, files: &[FileInfo]) {
        for file in files {
            if let Some(parent) = file.path.parent() {
                fs::create_dir_all(parent).ok();
            }
            fs::write(&file.path, "test content").ok();
        }
    }

    #[test]
    fn test_prioritize_without_limit() {
        let temp_dir = TempDir::new().unwrap();
        let files = vec![
            FileInfo {
                path: temp_dir.path().join("low.txt"),
                relative_path: PathBuf::from("low.txt"),
                size: 100,
                file_type: FileType::Text,
                priority: 0.3,
            },
            FileInfo {
                path: temp_dir.path().join("high.rs"),
                relative_path: PathBuf::from("high.rs"),
                size: 100,
                file_type: FileType::Rust,
                priority: 1.0,
            },
        ];

        create_test_files(&temp_dir, &files);
        let cache = create_test_cache();
        let options = DigestOptions::default();
        let result = prioritize_files(files, &options, cache).unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].relative_path, PathBuf::from("high.rs"));
        assert_eq!(result[1].relative_path, PathBuf::from("low.txt"));
    }

    #[test]
    fn test_group_by_directory() {
        let files = vec![
            FileInfo {
                path: PathBuf::from("src/main.rs"),
                relative_path: PathBuf::from("src/main.rs"),
                size: 100,
                file_type: FileType::Rust,
                priority: 1.0,
            },
            FileInfo {
                path: PathBuf::from("src/lib.rs"),
                relative_path: PathBuf::from("src/lib.rs"),
                size: 100,
                file_type: FileType::Rust,
                priority: 1.0,
            },
            FileInfo {
                path: PathBuf::from("tests/test.rs"),
                relative_path: PathBuf::from("tests/test.rs"),
                size: 100,
                file_type: FileType::Rust,
                priority: 0.8,
            },
        ];

        let groups = group_by_directory(files);

        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].0, "src");
        assert_eq!(groups[0].1.len(), 2);
        assert_eq!(groups[1].0, "tests");
        assert_eq!(groups[1].1.len(), 1);
    }

    #[test]
    fn test_prioritize_algorithm_ordering() {
        let temp_dir = TempDir::new().unwrap();
        let files = vec![
            FileInfo {
                path: temp_dir.path().join("test.rs"),
                relative_path: PathBuf::from("test.rs"),
                size: 500,
                file_type: FileType::Rust,
                priority: 0.8,
            },
            FileInfo {
                path: temp_dir.path().join("main.rs"),
                relative_path: PathBuf::from("main.rs"),
                size: 1000,
                file_type: FileType::Rust,
                priority: 1.5,
            },
            FileInfo {
                path: temp_dir.path().join("lib.rs"),
                relative_path: PathBuf::from("lib.rs"),
                size: 800,
                file_type: FileType::Rust,
                priority: 1.2,
            },
        ];

        create_test_files(&temp_dir, &files);
        let cache = create_test_cache();
        let options = DigestOptions::default();
        let result = prioritize_files(files, &options, cache).unwrap();

        // Should return all files when no limit
        assert_eq!(result.len(), 3);

        // Files should be sorted by priority (highest first)
        assert_eq!(result[0].relative_path, PathBuf::from("main.rs"));
        assert_eq!(result[1].relative_path, PathBuf::from("lib.rs"));
        assert_eq!(result[2].relative_path, PathBuf::from("test.rs"));
    }

    #[test]
    fn test_calculate_structure_overhead() {
        let files = vec![FileInfo {
            path: PathBuf::from("main.rs"),
            relative_path: PathBuf::from("main.rs"),
            size: 1000,
            file_type: FileType::Rust,
            priority: 1.5,
        }];

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

        let overhead = calculate_structure_overhead(&options, &files).unwrap();

        // Should account for headers, tree, stats, TOC
        assert!(overhead > 0);
        assert!(overhead < 10000); // Reasonable upper bound
    }

    #[test]
    fn test_priority_ordering() {
        let mut files = [
            FileInfo {
                path: PathBuf::from("test.rs"),
                relative_path: PathBuf::from("test.rs"),
                size: 500,
                file_type: FileType::Rust,
                priority: 0.8,
            },
            FileInfo {
                path: PathBuf::from("main.rs"),
                relative_path: PathBuf::from("main.rs"),
                size: 1000,
                file_type: FileType::Rust,
                priority: 1.5,
            },
            FileInfo {
                path: PathBuf::from("lib.rs"),
                relative_path: PathBuf::from("lib.rs"),
                size: 800,
                file_type: FileType::Rust,
                priority: 1.2,
            },
        ];

        // Sort by priority (highest first)
        files.sort_by(|a, b| b.priority.partial_cmp(&a.priority).unwrap());

        assert_eq!(files[0].relative_path, PathBuf::from("main.rs"));
        assert_eq!(files[1].relative_path, PathBuf::from("lib.rs"));
        assert_eq!(files[2].relative_path, PathBuf::from("test.rs"));
    }

    #[test]
    fn test_group_by_directory_complex() {
        let files = vec![
            FileInfo {
                path: PathBuf::from("src/core/mod.rs"),
                relative_path: PathBuf::from("src/core/mod.rs"),
                size: 500,
                file_type: FileType::Rust,
                priority: 1.0,
            },
            FileInfo {
                path: PathBuf::from("src/utils/helpers.rs"),
                relative_path: PathBuf::from("src/utils/helpers.rs"),
                size: 300,
                file_type: FileType::Rust,
                priority: 0.9,
            },
            FileInfo {
                path: PathBuf::from("tests/integration.rs"),
                relative_path: PathBuf::from("tests/integration.rs"),
                size: 200,
                file_type: FileType::Rust,
                priority: 0.8,
            },
            FileInfo {
                path: PathBuf::from("main.rs"),
                relative_path: PathBuf::from("main.rs"),
                size: 1000,
                file_type: FileType::Rust,
                priority: 1.5,
            },
        ];

        let grouped = group_by_directory(files);

        // Should have at least 3 groups
        assert!(grouped.len() >= 3);

        // Check that files are correctly grouped by directory
        let has_root_or_main = grouped.iter().any(|(dir, files)| {
            (dir == "." || dir.is_empty())
                && files.iter().any(|f| f.relative_path == PathBuf::from("main.rs"))
        });
        assert!(has_root_or_main);

        let has_src_core = grouped.iter().any(|(dir, files)| {
            dir == "src/core"
                && files.iter().any(|f| f.relative_path == PathBuf::from("src/core/mod.rs"))
        });
        assert!(has_src_core);
    }
}
