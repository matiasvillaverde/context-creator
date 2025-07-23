//! Search command implementation

use crate::{
    cli::{Commands, Config},
    core::search::{find_files_with_matches, SearchConfig},
};
use anyhow::Result;
use std::path::PathBuf;

/// Run the search command
pub fn run_search(mut config: Config) -> Result<()> {
    if let Some(Commands::Search {
        pattern,
        no_semantic,
        paths,
    }) = &config.command
    {
        // Clone values we need before modifying config
        let search_pattern = pattern.clone();
        let search_paths = paths.clone().unwrap_or_else(|| vec![PathBuf::from(".")]);
        let disable_semantic = *no_semantic;

        // Enable semantic flags automatically unless --no-semantic is specified
        if !disable_semantic {
            config.trace_imports = true;
            config.include_callers = true;
            config.include_types = true;
        }

        // Perform search across all paths
        let mut all_matches = Vec::new();
        for path in &search_paths {
            let search_config = SearchConfig {
                pattern: &search_pattern,
                path,
                case_insensitive: true,
                include_globs: &config.get_include_patterns(),
                exclude_globs: &config.get_ignore_patterns(),
            };

            let matches = find_files_with_matches(&search_config)?;
            all_matches.extend(matches);
        }

        if all_matches.is_empty() {
            // Generate empty context output
            let empty_files = Vec::new();
            let context_options = ContextOptions::from_config(&config)?;
            let cache = Arc::new(crate::core::cache::FileCache::new());

            let output = if config.output_format == crate::cli::OutputFormat::Markdown {
                crate::core::context_builder::generate_markdown(
                    empty_files,
                    context_options,
                    cache,
                )?
            } else {
                crate::core::context_builder::generate_digest(
                    empty_files,
                    context_options,
                    cache,
                    config.output_format,
                    &search_paths[0].display().to_string(),
                )?
            };

            // Handle output based on configuration
            match (config.output_file.as_ref(), config.copy) {
                (Some(file), false) => {
                    std::fs::write(file, output)?;
                    if !config.quiet {
                        println!("✓ Written to {}", file.display());
                    }
                }
                (None, true) => {
                    crate::copy_to_clipboard(&output)?;
                    if !config.quiet {
                        println!("✓ Copied to clipboard");
                    }
                }
                (None, false) => {
                    print!("{output}");
                }
                _ => unreachable!("Invalid output configuration"),
            }

            return Ok(());
        }

        // Create a temporary directory list file to pass specific files
        // We'll use the existing pipeline but with a custom file filter
        use crate::core::walker::{FileInfo, WalkOptions};
        use crate::ContextOptions;
        use std::sync::Arc;

        // Convert matched files to FileInfo
        let mut files = Vec::new();
        for path in &all_matches {
            if let Ok(metadata) = std::fs::metadata(path) {
                let file_info = FileInfo {
                    path: path.clone(),
                    relative_path: path
                        .strip_prefix(&search_paths[0])
                        .unwrap_or(path)
                        .to_path_buf(),
                    size: metadata.len(),
                    file_type: crate::utils::file_ext::FileType::from_path(path),
                    priority: 10.0, // High priority for direct search matches
                    imports: vec![],
                    imported_by: vec![],
                    function_calls: vec![],
                    type_references: vec![],
                    exported_functions: vec![],
                };
                files.push(file_info);
            }
        }

        // Clear the command to avoid recursion
        config.command = None;

        // Now process the filtered files through the semantic pipeline
        let cache = Arc::new(crate::core::cache::FileCache::new());
        let walk_options = WalkOptions::from_config(&config)?;
        let context_options = ContextOptions::from_config(&config)?;

        // If semantic analysis is enabled, expand the file list
        if config.trace_imports || config.include_callers || config.include_types {
            if config.progress && !config.quiet {
                println!("Analyzing semantic dependencies...");
            }

            // Perform semantic analysis on the matched files
            let project_analysis = crate::core::project_analyzer::ProjectAnalysis::analyze_project(
                &search_paths[0],
                &walk_options,
                &config,
                &cache,
            )?;

            // Create initial set from our search results
            let mut initial_files_map = std::collections::HashMap::new();
            for file in files {
                if let Some(analyzed_file) = project_analysis.get_file(&file.path) {
                    initial_files_map.insert(file.path.clone(), analyzed_file.clone());
                } else {
                    initial_files_map.insert(file.path.clone(), file);
                }
            }

            // Expand file list based on semantic relationships
            let files_map = crate::core::file_expander::expand_file_list_with_context(
                initial_files_map,
                &config,
                &cache,
                &walk_options,
                &project_analysis.file_map,
            )?;

            // Convert back to Vec<FileInfo>
            files = files_map.into_values().collect();
        }

        // Prioritize files if needed
        let prioritized_files = if context_options.max_tokens.is_some() {
            crate::core::prioritizer::prioritize_files(files, &context_options, cache.clone())?
        } else {
            files
        };

        // Generate output
        let output = if config.output_format == crate::cli::OutputFormat::Markdown {
            crate::core::context_builder::generate_markdown(
                prioritized_files,
                context_options,
                cache,
            )?
        } else {
            crate::core::context_builder::generate_digest(
                prioritized_files,
                context_options,
                cache,
                config.output_format,
                &search_paths[0].display().to_string(),
            )?
        };

        // Handle output based on configuration
        match (config.output_file.as_ref(), config.copy) {
            (Some(file), false) => {
                std::fs::write(file, output)?;
                if !config.quiet {
                    println!("✓ Written to {}", file.display());
                }
            }
            (None, true) => {
                crate::copy_to_clipboard(&output)?;
                if !config.quiet {
                    println!("✓ Copied to clipboard");
                }
            }
            (None, false) => {
                print!("{output}");
            }
            _ => unreachable!("Invalid output configuration"),
        }

        Ok(())
    } else {
        unreachable!("run_search called without search command")
    }
}
