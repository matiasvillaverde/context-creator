//! Git diff command implementation

use crate::cli::{Commands, Config};
use crate::core::{
    cache::FileCache,
    context_builder::ContextOptions,
    file_expander, prioritizer,
    project_analyzer::ProjectAnalysis,
    walker::{walk_directory, FileInfo, WalkOptions},
};
use crate::utils::file_ext::{get_language_from_extension, FileType};
use crate::utils::git;
use anyhow::{anyhow, Result};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
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

    let repo_root = git::get_repository_root(&working_dir).map_err(|_| {
        anyhow!("Not a git repository. Please run this command from within a git repository.")
    })?;

    info!("Analyzing git diff between {} and {}", from, to);

    // Get the list of changed files
    let changed_files = match git::get_changed_files(&repo_root, &from, &to) {
        Ok(files) => files,
        Err(e) => {
            return Err(anyhow!("Failed to get changed files: {}", e));
        }
    };

    info!("Found {} changed files", changed_files.len());

    // Get diff statistics for summary
    let stats = git::get_diff_stats(&repo_root, &from, &to)?;

    // Create a cache for file operations
    let cache = Arc::new(FileCache::new());

    // Create context options
    let context_options = ContextOptions::from_config(&config)?;
    let walk_options = WalkOptions::from_config(&config)?;

    // Filter to only include changed files that exist and are readable
    let mut valid_files = Vec::new();
    for file in &changed_files {
        if file.exists() && file.is_file() {
            valid_files.push(file.clone());
        } else {
            debug!("Skipping non-existent or non-file: {:?}", file);
        }
    }

    if !changed_files.is_empty() && valid_files.is_empty() {
        println!("No valid files to process in the diff.");
        return Ok(());
    }

    let changed_file_infos = collect_changed_file_infos(&repo_root, &valid_files, &walk_options)?;
    let changed_file_keys = path_key_set(changed_file_infos.iter().map(|file| file.path.as_path()));

    let mut initial_files_map = HashMap::new();
    for file in changed_file_infos.clone() {
        initial_files_map.insert(file.path.clone(), file);
    }

    let expanded_files_map =
        if config.trace_imports || config.include_callers || config.include_types {
            info!("Performing semantic analysis on changed files");
            if config.include_callers {
                let project_analysis =
                    ProjectAnalysis::analyze_project(&repo_root, &walk_options, &config, &cache)?;

                let mut analyzed_initial_files = HashMap::new();
                for file in initial_files_map.into_values() {
                    if let Some(analyzed_file) = project_analysis.get_file(&file.path) {
                        analyzed_initial_files
                            .insert(analyzed_file.path.clone(), analyzed_file.clone());
                    } else {
                        analyzed_initial_files.insert(file.path.clone(), file);
                    }
                }

                file_expander::expand_file_list_with_context(
                    analyzed_initial_files,
                    &config,
                    &cache,
                    &walk_options,
                    &project_analysis.file_map,
                )?
            } else {
                file_expander::expand_file_list(initial_files_map, &config, &cache, &walk_options)?
            }
        } else {
            initial_files_map
        };

    let expanded_files = expanded_files_map.into_values().collect();
    let files_to_process =
        prioritizer::prioritize_files(expanded_files, &context_options, cache.clone())?;

    if let Some(max_tokens) = context_options.max_tokens {
        debug!("Token limit enabled: {}", max_tokens);
        debug!(
            "Selected {} files after prioritization",
            files_to_process.len()
        );
    };

    // Generate the diff markdown
    let markdown = generate_diff_markdown(DiffMarkdownParams {
        from: &from,
        to: &to,
        stats: &stats,
        repo_root: &repo_root,
        changed_files: &changed_file_infos,
        changed_file_keys: &changed_file_keys,
        files: &files_to_process,
        cache,
    })?;

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
    repo_root: &'a Path,
    changed_files: &'a [FileInfo],
    changed_file_keys: &'a HashSet<PathBuf>,
    files: &'a [FileInfo],
    cache: Arc<FileCache>,
}

/// Generate markdown content for the diff
fn generate_diff_markdown(params: DiffMarkdownParams) -> Result<String> {
    let mut markdown = String::new();
    let mut changed_files: Vec<&FileInfo> = params.changed_files.iter().collect();
    changed_files.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));

    let mut files: Vec<&FileInfo> = params.files.iter().collect();
    files.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));

    // Header
    markdown.push_str(&format!(
        "# Git Diff Analysis: {} → {}\n\n",
        params.from, params.to
    ));
    markdown.push_str(&format!("From: {}\n", params.from));
    markdown.push_str(&format!("To: {}\n\n", params.to));

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
    if changed_files.is_empty() {
        markdown.push_str("No files changed.\n");
    } else {
        for file in changed_files {
            markdown.push_str(&format!("- `{}`\n", file.relative_path.display()));
        }
    }
    markdown.push('\n');

    let semantic_files: Vec<&FileInfo> = files
        .iter()
        .copied()
        .filter(|file| !params.changed_file_keys.contains(&path_key(&file.path)))
        .collect();

    if !semantic_files.is_empty() {
        markdown.push_str("## Semantic Context Files\n\n");
        for file in semantic_files {
            markdown.push_str(&format!("- `{}`\n", file.relative_path.display()));
        }
        markdown.push('\n');
    }

    // File contents
    markdown.push_str("## File Contents\n\n");

    for file in files {
        let relative_path = if file.relative_path.as_os_str().is_empty() {
            file.path
                .strip_prefix(params.repo_root)
                .unwrap_or(&file.path)
        } else {
            &file.relative_path
        };

        markdown.push_str(&format!("### {}\n\n", relative_path.display()));

        let language = get_language_from_extension(&file.path);

        // Read file content
        match params.cache.get_or_load(&file.path) {
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

fn collect_changed_file_infos(
    repo_root: &Path,
    changed_files: &[PathBuf],
    walk_options: &WalkOptions,
) -> Result<Vec<FileInfo>> {
    if changed_files.is_empty() {
        return Ok(Vec::new());
    }

    let wanted = path_key_set(changed_files.iter().map(PathBuf::as_path));
    let mut found = HashSet::new();
    let mut file_infos = Vec::new();

    for file in walk_directory(repo_root, walk_options.clone())? {
        let key = path_key(&file.path);
        if wanted.contains(&key) {
            found.insert(key);
            file_infos.push(file);
        }
    }

    for path in changed_files {
        let key = path_key(path);
        if !found.contains(&key) {
            file_infos.push(file_info_for_changed_path(repo_root, path)?);
        }
    }

    Ok(file_infos)
}

fn file_info_for_changed_path(repo_root: &Path, path: &Path) -> Result<FileInfo> {
    let metadata = std::fs::metadata(path)?;
    let relative_path = path.strip_prefix(repo_root).unwrap_or(path).to_path_buf();
    let file_type = FileType::from_path(path);

    Ok(FileInfo {
        path: path.to_path_buf(),
        relative_path,
        size: metadata.len(),
        file_type,
        priority: 1.0,
        imports: Vec::new(),
        imported_by: Vec::new(),
        function_calls: Vec::new(),
        type_references: Vec::new(),
        exported_functions: Vec::new(),
    })
}

fn path_key_set<'a>(paths: impl Iterator<Item = &'a Path>) -> HashSet<PathBuf> {
    paths.map(path_key).collect()
}

fn path_key(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

/// Simple token estimation (rough approximation)
fn estimate_token_count(text: &str) -> usize {
    // Very rough approximation: 1 token ≈ 4 characters
    text.len() / 4
}
