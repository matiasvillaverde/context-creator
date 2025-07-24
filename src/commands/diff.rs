//! Git diff command implementation

use crate::cli::{Commands, Config};
use crate::core::{cache::FileCache, context_builder::ContextOptions};
use crate::utils::git;
use anyhow::{anyhow, Result};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{debug, info};

/// Run the git diff command
pub fn run_diff(config: Config) -> Result<()> {
    let (from, to) = match &config.command {
        Some(Commands::Diff { from, to }) => (from.clone(), to.clone()),
        _ => return Err(anyhow!("Invalid command for diff execution")),
    };

    // Determine the working directory (current directory by default)
    let working_dir = std::env::current_dir()?;

    // Check if we're in a git repository
    if !git::is_git_repository(&working_dir) {
        return Err(anyhow!(
            "Not in a git repository. Please run this command from within a git repository."
        ));
    }

    info!("Analyzing git diff between {} and {}", from, to);

    // Get the list of changed files
    let changed_files = match git::get_changed_files(&working_dir, &from, &to) {
        Ok(files) => files,
        Err(e) => {
            return Err(anyhow!("Failed to get changed files: {}", e));
        }
    };

    if changed_files.is_empty() {
        println!("No files changed between {from} and {to}");
        return Ok(());
    }

    info!("Found {} changed files", changed_files.len());

    // Get diff statistics for summary
    let stats = git::get_diff_stats(&working_dir, &from, &to)?;

    // Create a cache for file operations
    let cache = Arc::new(FileCache::new());

    // Create context options
    let context_options = ContextOptions::from_config(&config)?;

    // Filter to only include changed files that exist and are readable
    let mut valid_files = Vec::new();
    for file in changed_files {
        if file.exists() && file.is_file() {
            valid_files.push(file);
        } else {
            debug!("Skipping non-existent or non-file: {:?}", file);
        }
    }

    if valid_files.is_empty() {
        println!("No valid files to process in the diff.");
        return Ok(());
    }

    // Apply basic token limits if needed (simplified for now)
    let files_to_process = if let Some(max_tokens) = context_options.max_tokens {
        debug!("Token limit enabled: {}", max_tokens);
        // For now, just limit the number of files. A proper implementation would
        // estimate token usage per file and prioritize accordingly.
        let max_files = (max_tokens / 1000).max(1).min(valid_files.len());
        valid_files.into_iter().take(max_files).collect()
    } else {
        valid_files
    };

    // Generate the diff markdown
    let mut markdown = generate_diff_markdown(DiffMarkdownParams {
        from: &from,
        to: &to,
        stats: &stats,
        files: &files_to_process,
        cache,
    })?;

    // Handle semantic analysis if requested
    if config.trace_imports || config.include_callers || config.include_types {
        info!("Performing semantic analysis on changed files");
        // For now, add a placeholder - full semantic integration would require more work
        markdown.push_str("\n\n## Semantic Analysis\n\n");
        markdown.push_str("*Semantic analysis integration is in development*\n");
    }

    // Output the result
    if let Some(output_file) = &config.output_file {
        std::fs::write(output_file, &markdown)?;
        info!("Diff analysis written to: {:?}", output_file);
    } else {
        print!("{markdown}");
    }

    Ok(())
}

/// Parameters for generating diff markdown
struct DiffMarkdownParams<'a> {
    from: &'a str,
    to: &'a str,
    stats: &'a git::DiffStats,
    files: &'a [PathBuf],
    cache: Arc<FileCache>,
}

/// Generate markdown content for the diff
fn generate_diff_markdown(params: DiffMarkdownParams) -> Result<String> {
    let mut markdown = String::new();

    // Header
    markdown.push_str(&format!(
        "# Git Diff Analysis: {} → {}\n\n",
        params.from, params.to
    ));

    // Statistics
    markdown.push_str("## Diff Statistics\n\n");
    markdown.push_str(&format!(
        "- **Files changed**: {}\n",
        params.stats.files_changed
    ));
    markdown.push_str(&format!("- **Lines added**: {}\n", params.stats.insertions));
    markdown.push_str(&format!(
        "- **Lines removed**: {}\n",
        params.stats.deletions
    ));
    markdown.push('\n');

    // Changed files summary
    markdown.push_str("## Changed Files\n\n");
    for file in params.files {
        let relative_path = file.strip_prefix(std::env::current_dir()?).unwrap_or(file);
        markdown.push_str(&format!("- `{}`\n", relative_path.display()));
    }
    markdown.push('\n');

    // File contents
    markdown.push_str("## File Contents\n\n");

    for file in params.files {
        let relative_path = file.strip_prefix(std::env::current_dir()?).unwrap_or(file);

        markdown.push_str(&format!("### {}\n\n", relative_path.display()));

        // Determine file extension for syntax highlighting
        let extension = file.extension().and_then(|ext| ext.to_str()).unwrap_or("");

        let language = match extension {
            "rs" => "rust",
            "py" => "python",
            "js" => "javascript",
            "ts" => "typescript",
            "go" => "go",
            "java" => "java",
            "cpp" | "cc" | "cxx" => "cpp",
            "c" => "c",
            "h" | "hpp" => "c",
            "sh" => "bash",
            "yml" | "yaml" => "yaml",
            "json" => "json",
            "toml" => "toml",
            "md" => "markdown",
            _ => "",
        };

        // Read file content
        match params.cache.get_or_load(file) {
            Ok(content) => {
                markdown.push_str(&format!("```{language}\n{content}\n```\n\n"));
            }
            Err(e) => {
                markdown.push_str(&format!("*Error reading file: {e}*\n\n"));
            }
        }
    }

    // Context statistics
    let total_tokens = estimate_token_count(&markdown);
    markdown.push_str("## Context Statistics\n\n");
    markdown.push_str(&format!("- **Files processed**: {}\n", params.files.len()));
    markdown.push_str(&format!("- **Estimated tokens**: {total_tokens}\n"));

    Ok(markdown)
}

/// Simple token estimation (rough approximation)
fn estimate_token_count(text: &str) -> usize {
    // Very rough approximation: 1 token ≈ 4 characters
    text.len() / 4
}
