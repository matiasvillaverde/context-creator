//! Context Creator - High-performance CLI tool to convert codebases to Markdown for LLM context
//!
//! This library provides the core functionality for traversing directories,
//! processing files, and generating formatted Markdown output suitable for
//! large language model consumption.

pub mod cli;
pub mod config;
pub mod core;
pub mod logging;
pub mod remote;
pub mod utils;

use anyhow::Result;
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, info};

pub use cli::Config;
pub use core::{cache::FileCache, context_builder::ContextOptions, walker::WalkOptions};
pub use utils::error::ContextCreatorError;

/// Main entry point for the context creator library
pub fn run(mut config: Config) -> Result<()> {
    // Load configuration from file first
    config.load_from_file()?;

    // Validate configuration BEFORE processing
    // This ensures mutual exclusivity checks work correctly
    config.validate()?;

    // Handle remote repository if specified
    let _temp_dir = if let Some(repo_url) = &config.remote {
        if config.verbose > 0 {
            debug!(
                "Starting context-creator with remote repository: {}",
                repo_url
            );
        }

        // Fetch the repository
        let temp_dir = crate::remote::fetch_repository(repo_url, config.verbose > 0)?;
        let repo_path = crate::remote::get_repo_path(&temp_dir, repo_url)?;

        // Update config to use the cloned repository
        config.paths = Some(vec![repo_path]);

        Some(temp_dir) // Keep temp_dir alive until end of function
    } else {
        None
    };

    // No need to update config since get_directories() handles resolution

    // Setup logging based on verbosity
    if config.verbose > 0 {
        debug!("Starting context-creator with configuration:");
        debug!("  Directories: {:?}", config.get_directories());
        debug!("  Max tokens: {:?}", config.max_tokens);
        debug!("  LLM tool: {}", config.llm_tool.command());
        debug!("  Progress: {}", config.progress);
        debug!("  Quiet: {}", config.quiet);
        if let Some(output) = &config.output_file {
            debug!("  Output file: {}", output.display());
        }
        if let Some(prompt) = config.get_prompt() {
            debug!("  Prompt: {}", prompt);
        }
    }

    // Configuration was already validated above

    // Create walker with options
    if config.verbose > 0 {
        debug!("Creating directory walker with options...");
    }
    let walk_options = WalkOptions::from_config(&config)?;

    // Create context options
    if config.verbose > 0 {
        debug!("Creating context generation options...");
    }
    let context_options = ContextOptions::from_config(&config)?;

    // Create shared file cache
    if config.verbose > 0 {
        debug!("Creating file cache for I/O optimization...");
    }
    let cache = Arc::new(FileCache::new());

    // Process all directories
    let mut all_outputs = Vec::new();

    let directories = config.get_directories();
    for (index, directory) in directories.iter().enumerate() {
        if config.progress && !config.quiet && directories.len() > 1 {
            info!(
                "Processing directory {} of {}: {}",
                index + 1,
                directories.len(),
                directory.display()
            );
        }

        let output = process_directory(
            directory,
            walk_options.clone(),
            context_options.clone(),
            cache.clone(),
            &config,
        )?;
        all_outputs.push((directory.clone(), output));
    }

    // Combine outputs from all directories
    let output = if all_outputs.len() == 1 {
        // Single directory - return output as-is
        all_outputs.into_iter().next().unwrap().1
    } else {
        // Multiple directories - combine with headers
        let mut combined = String::new();
        combined.push_str("# Code Context - Multiple Directories\n\n");

        for (path, content) in all_outputs {
            combined.push_str(&format!("## Directory: {}\n\n", path.display()));
            combined.push_str(&content);
            combined.push_str("\n\n");
        }

        combined
    };

    // Handle output based on configuration
    let resolved_prompt = config.get_prompt();
    match (
        config.output_file.as_ref(),
        resolved_prompt.as_ref(),
        config.copy,
    ) {
        (Some(file), None, false) => {
            // Write to file
            std::fs::write(file, output)?;
            if !config.quiet {
                println!(" Written to {}", file.display());
            }
        }
        (None, Some(prompt), false) => {
            // Send to LLM CLI with prompt
            if config.progress && !config.quiet {
                info!("Sending context to {}...", config.llm_tool.command());
            }
            execute_with_llm(prompt, &output, &config)?;
        }
        (None, Some(prompt), true) => {
            // Copy to clipboard then send to LLM
            copy_to_clipboard(&output)?;
            if !config.quiet {
                println!("✓ Copied to clipboard");
            }
            if config.progress && !config.quiet {
                info!("Sending context to {}...", config.llm_tool.command());
            }
            execute_with_llm(prompt, &output, &config)?;
        }
        (None, None, true) => {
            // Copy to clipboard
            copy_to_clipboard(&output)?;
            if !config.quiet {
                println!("✓ Copied to clipboard");
            }
        }
        (None, None, false) => {
            // Print to stdout
            print!("{output}");
        }
        (Some(_), _, true) => {
            // This should have been caught by validation
            return Err(ContextCreatorError::InvalidConfiguration(
                "Cannot specify both --copy and --output".to_string(),
            )
            .into());
        }
        (Some(_), Some(_), _) => {
            return Err(ContextCreatorError::InvalidConfiguration(
                "Cannot specify both output file and prompt".to_string(),
            )
            .into());
        }
    }

    Ok(())
}

/// Process a directory and generate markdown output
fn process_directory(
    path: &Path,
    walk_options: WalkOptions,
    context_options: ContextOptions,
    cache: Arc<FileCache>,
    config: &Config,
) -> Result<String> {
    // Walk the directory
    if config.progress && !config.quiet {
        info!("Scanning directory: {}", path.display());
    }
    let mut files = core::walker::walk_directory(path, walk_options.clone())?;

    if config.progress && !config.quiet {
        info!("Found {} files", files.len());
    }

    // Perform semantic analysis if requested
    if config.trace_imports || config.include_callers || config.include_types {
        if config.progress && !config.quiet {
            info!("Analyzing semantic dependencies...");
        }

        // Perform single-pass project analysis
        let project_analysis = core::project_analyzer::ProjectAnalysis::analyze_project(
            path,
            &walk_options,
            config,
            &cache,
        )?;

        // Create initial set from our filtered files
        let mut initial_files_map = std::collections::HashMap::new();
        for file in files {
            if let Some(analyzed_file) = project_analysis.get_file(&file.path) {
                initial_files_map.insert(file.path.clone(), analyzed_file.clone());
            } else {
                initial_files_map.insert(file.path.clone(), file);
            }
        }

        // Expand file list based on semantic relationships, using the full project context
        if config.progress && !config.quiet {
            info!("Expanding file list based on semantic relationships...");
        }

        // Pass the project analysis file map as context for expansion
        let files_map = core::file_expander::expand_file_list_with_context(
            initial_files_map,
            config,
            &cache,
            &walk_options,
            &project_analysis.file_map,
        )?;

        // Convert back to Vec<FileInfo>
        files = files_map.into_values().collect();

        // Clean up imported_by fields to only include files in our final set
        let final_paths: std::collections::HashSet<_> =
            files.iter().map(|f| f.path.clone()).collect();
        for file in &mut files {
            file.imported_by.retain(|path| final_paths.contains(path));
        }

        if config.progress && !config.quiet {
            info!("Expanded to {} files", files.len());
        }
    }

    if config.verbose > 0 {
        debug!("File list:");
        for file in &files {
            debug!(
                "  {} ({})",
                file.relative_path.display(),
                file.file_type_display()
            );
        }
    }

    // Prioritize files if needed
    let prioritized_files = if context_options.max_tokens.is_some() {
        if config.progress && !config.quiet {
            info!("Prioritizing files for token limit...");
        }
        core::prioritizer::prioritize_files(files, &context_options, cache.clone())?
    } else {
        files
    };

    if config.progress && !config.quiet {
        info!(
            "Generating markdown from {} files...",
            prioritized_files.len()
        );
    }

    // Generate markdown
    let markdown =
        core::context_builder::generate_markdown(prioritized_files, context_options, cache)?;

    if config.progress && !config.quiet {
        info!("Markdown generation complete");
    }

    Ok(markdown)
}

/// Execute LLM CLI with the generated context
fn execute_with_llm(prompt: &str, context: &str, config: &Config) -> Result<()> {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let full_input = format!("{prompt}\n\n{context}");
    let tool_command = config.llm_tool.command();

    let mut child = Command::new(tool_command)
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                ContextCreatorError::LlmToolNotFound {
                    tool: tool_command.to_string(),
                    install_instructions: config.llm_tool.install_instructions().to_string(),
                }
            } else {
                ContextCreatorError::SubprocessError(e.to_string())
            }
        })?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(full_input.as_bytes())?;
        stdin.flush()?;
    }

    let status = child.wait()?;
    if !status.success() {
        return Err(ContextCreatorError::SubprocessError(format!(
            "{tool_command} exited with status: {status}"
        ))
        .into());
    }

    if !config.quiet {
        info!("{} completed successfully", tool_command);
    }

    Ok(())
}

/// Copy content to system clipboard
fn copy_to_clipboard(content: &str) -> Result<()> {
    use arboard::Clipboard;

    let mut clipboard = Clipboard::new().map_err(|e| {
        ContextCreatorError::ClipboardError(format!("Failed to access clipboard: {e}"))
    })?;

    clipboard.set_text(content).map_err(|e| {
        ContextCreatorError::ClipboardError(format!("Failed to copy to clipboard: {e}"))
    })?;

    Ok(())
}
