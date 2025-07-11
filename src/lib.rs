//! Code Digest - High-performance CLI tool to convert codebases to Markdown for LLM context
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

pub use cli::Config;
pub use core::{digest::DigestOptions, walker::WalkOptions};
pub use utils::error::CodeDigestError;

/// Main entry point for the code digest library
pub fn run(mut config: Config) -> Result<()> {
    // Handle remote repository if specified
    let _temp_dir = if let Some(repo_url) = &config.repo {
        if config.verbose {
            eprintln!("🔧 Starting code-digest with remote repository: {repo_url}");
        }

        // Fetch the repository
        let temp_dir = crate::remote::fetch_repository(repo_url, config.verbose)?;
        let repo_path = crate::remote::get_repo_path(&temp_dir, repo_url)?;

        // Update config to use the cloned repository
        config.directories = vec![repo_path];

        Some(temp_dir) // Keep temp_dir alive until end of function
    } else {
        None
    };

    // Update config with resolved directories first
    config.directories = config.get_directories();

    // Setup logging based on verbosity
    if config.verbose {
        eprintln!("🔧 Starting code-digest with configuration:");
        eprintln!("  Directories: {:?}", config.directories);
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

    // Update config with resolved directories
    config.directories = config.get_directories();

    // Validate configuration
    config.validate()?;

    // Create walker with options
    if config.verbose {
        eprintln!("🚶 Creating directory walker with options...");
    }
    let walk_options = WalkOptions::from_config(&config)?;

    // Create digest options
    if config.verbose {
        eprintln!("📄 Creating markdown digest options...");
    }
    let digest_options = DigestOptions::from_config(&config)?;

    // Process all directories
    let mut all_outputs = Vec::new();

    for (index, directory) in config.directories.iter().enumerate() {
        if config.progress && !config.quiet && config.directories.len() > 1 {
            eprintln!(
                "📂 Processing directory {} of {}: {}",
                index + 1,
                config.directories.len(),
                directory.display()
            );
        }

        let output =
            process_directory(directory, walk_options.clone(), digest_options.clone(), &config)?;
        all_outputs.push((directory.clone(), output));
    }

    // Combine outputs from all directories
    let output = if all_outputs.len() == 1 {
        // Single directory - return output as-is
        all_outputs.into_iter().next().unwrap().1
    } else {
        // Multiple directories - combine with headers
        let mut combined = String::new();
        combined.push_str("# Code Digest - Multiple Directories\n\n");

        for (path, content) in all_outputs {
            combined.push_str(&format!("## Directory: {}\n\n", path.display()));
            combined.push_str(&content);
            combined.push_str("\n\n");
        }

        combined
    };

    // Handle output based on configuration
    let resolved_prompt = config.get_prompt();
    match (config.output_file.as_ref(), resolved_prompt.as_ref()) {
        (Some(file), None) => {
            // Write to file
            std::fs::write(file, output)?;
            if !config.quiet {
                println!(" Written to {}", file.display());
            }
        }
        (None, Some(prompt)) => {
            // Send to LLM CLI with prompt
            if config.progress && !config.quiet {
                eprintln!("🤖 Sending context to {}...", config.llm_tool.command());
            }
            execute_with_llm(prompt, &output, &config)?;
        }
        (None, None) => {
            // Print to stdout
            print!("{output}");
        }
        (Some(_), Some(_)) => {
            return Err(CodeDigestError::InvalidConfiguration(
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
    digest_options: DigestOptions,
    config: &Config,
) -> Result<String> {
    // Walk the directory
    if config.progress && !config.quiet {
        eprintln!("🔍 Scanning directory: {}", path.display());
    }
    let files = core::walker::walk_directory(path, walk_options)?;

    if config.progress && !config.quiet {
        eprintln!("📁 Found {} files", files.len());
    }

    if config.verbose {
        eprintln!("📋 File list:");
        for file in &files {
            eprintln!("  {} ({})", file.relative_path.display(), file.file_type_display());
        }
    }

    // Prioritize files if needed
    let prioritized_files = if digest_options.max_tokens.is_some() {
        if config.progress && !config.quiet {
            eprintln!("🎯 Prioritizing files for token limit...");
        }
        core::prioritizer::prioritize_files(files, &digest_options)?
    } else {
        files
    };

    if config.progress && !config.quiet {
        eprintln!("📝 Generating markdown from {} files...", prioritized_files.len());
    }

    // Generate markdown
    let markdown = core::digest::generate_markdown(prioritized_files, digest_options)?;

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
                CodeDigestError::LlmToolNotFound {
                    tool: tool_command.to_string(),
                    install_instructions: config.llm_tool.install_instructions().to_string(),
                }
            } else {
                CodeDigestError::SubprocessError(e.to_string())
            }
        })?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(full_input.as_bytes())?;
        stdin.flush()?;
    }

    let status = child.wait()?;
    if !status.success() {
        return Err(CodeDigestError::SubprocessError(format!(
            "{tool_command} exited with status: {status}"
        ))
        .into());
    }

    if !config.quiet {
        eprintln!("\n✓ {tool_command} completed successfully");
    }

    Ok(())
}
