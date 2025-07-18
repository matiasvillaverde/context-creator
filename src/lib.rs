//! Context Creator - High-performance CLI tool to convert codebases to Markdown for LLM context
//!
//! This library provides the core functionality for traversing directories,
//! processing files, and generating formatted Markdown output suitable for
//! large language model consumption.

pub mod cli;
pub mod config;
pub mod core;
pub mod remote;
pub mod utils;

use anyhow::Result;
use std::path::Path;
use std::sync::Arc;

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
    let _temp_dir = if let Some(repo_url) = &config.repo {
        if config.verbose {
            eprintln!("🔧 Starting context-creator with remote repository: {repo_url}");
        }

        // Fetch the repository
        let temp_dir = crate::remote::fetch_repository(repo_url, config.verbose)?;
        let repo_path = crate::remote::get_repo_path(&temp_dir, repo_url)?;

        // Update config to use the cloned repository
        config.paths = Some(vec![repo_path]);

        Some(temp_dir) // Keep temp_dir alive until end of function
    } else {
        None
    };

    // No need to update config since get_directories() handles resolution

    // Setup logging based on verbosity
    if config.verbose {
        eprintln!("🔧 Starting context-creator with configuration:");
        eprintln!("  Directories: {:?}", config.get_directories());
        eprintln!("  Max tokens: {:?}", config.max_tokens);
        eprintln!("  LLM tool: {}", config.llm_tool.command());
        eprintln!("  Progress: {}", config.progress);
        eprintln!("  Quiet: {}", config.quiet);
        if let Some(output) = &config.output_file {
            eprintln!("  Output file: {}", output.display());
        }
        if let Some(prompt) = config.get_prompt() {
            eprintln!("  Prompt: {prompt}");
        }
    }

    // Configuration was already validated above

    // Create walker with options
    if config.verbose {
        eprintln!("🚶 Creating directory walker with options...");
    }
    let walk_options = WalkOptions::from_config(&config)?;

    // Create context options
    if config.verbose {
        eprintln!("📄 Creating context generation options...");
    }
    let context_options = ContextOptions::from_config(&config)?;

    // Create shared file cache
    if config.verbose {
        eprintln!("💾 Creating file cache for I/O optimization...");
    }
    let cache = Arc::new(FileCache::new());

    // Process all directories
    let mut all_outputs = Vec::new();

    let directories = config.get_directories();
    for (index, directory) in directories.iter().enumerate() {
        if config.progress && !config.quiet && directories.len() > 1 {
            eprintln!(
                "📂 Processing directory {} of {}: {}",
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
                eprintln!("🤖 Sending context to {}...", config.llm_tool.command());
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
                eprintln!("🤖 Sending context to {}...", config.llm_tool.command());
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
        eprintln!("🔍 Scanning directory: {}", path.display());
    }
    let mut files = core::walker::walk_directory(path, walk_options.clone())?;

    if config.progress && !config.quiet {
        eprintln!("📁 Found {} files", files.len());
    }

    // Perform semantic analysis if requested
    if config.trace_imports || config.include_callers || config.include_types {
        if config.progress && !config.quiet {
            eprintln!("🔗 Analyzing semantic dependencies...");
        }
        core::walker::perform_semantic_analysis(&mut files, config, &cache)?;

        if config.progress && !config.quiet {
            let import_count: usize = files.iter().map(|f| f.imports.len()).sum();
            eprintln!("✅ Found {import_count} import relationships");
        }

        // Expand file list based on semantic relationships
        if config.progress && !config.quiet {
            eprintln!("📂 Expanding file list based on semantic relationships...");
        }

        // Convert Vec<FileInfo> to HashMap for expansion
        let mut files_map = std::collections::HashMap::new();
        for file in files {
            files_map.insert(file.path.clone(), file);
        }

        // Expand the file list
        files_map =
            core::file_expander::expand_file_list(files_map, config, &cache, &walk_options)?;

        // Convert back to Vec<FileInfo>
        files = files_map.into_values().collect();

        if config.progress && !config.quiet {
            eprintln!("📊 Expanded to {} files", files.len());
        }
    }

    if config.verbose {
        eprintln!("📋 File list:");
        for file in &files {
            eprintln!(
                "  {} ({})",
                file.relative_path.display(),
                file.file_type_display()
            );
        }
    }

    // Prioritize files if needed
    let prioritized_files = if context_options.max_tokens.is_some() {
        if config.progress && !config.quiet {
            eprintln!("🎯 Prioritizing files for token limit...");
        }
        core::prioritizer::prioritize_files(files, &context_options, cache.clone())?
    } else {
        files
    };

    if config.progress && !config.quiet {
        eprintln!(
            "📝 Generating markdown from {} files...",
            prioritized_files.len()
        );
    }

    // Generate markdown
    let markdown =
        core::context_builder::generate_markdown(prioritized_files, context_options, cache)?;

    if config.progress && !config.quiet {
        eprintln!("✅ Markdown generation complete");
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
        eprintln!("\n✓ {tool_command} completed successfully");
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
