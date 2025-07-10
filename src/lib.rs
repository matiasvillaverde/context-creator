//! Code Digest - High-performance CLI tool to convert codebases to Markdown for LLM context
//!
//! This library provides the core functionality for traversing directories,
//! processing files, and generating formatted Markdown output suitable for
//! large language model consumption.

pub mod cli;
pub mod core;
pub mod utils;

use anyhow::Result;
use std::path::Path;

pub use cli::Config;
pub use core::{digest::DigestOptions, walker::WalkOptions};
pub use utils::error::CodeDigestError;

/// Main entry point for the code digest library
pub fn run(config: Config) -> Result<()> {
    // Validate configuration
    config.validate()?;

    // Create walker with options
    let walk_options = WalkOptions::from_config(&config)?;

    // Create digest options
    let digest_options = DigestOptions::from_config(&config)?;

    // Process the directory
    let output = process_directory(&config.directory, walk_options, digest_options, &config)?;

    // Handle output based on configuration
    match (config.output_file.as_ref(), config.prompt.as_ref()) {
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
            print!("{}", output);
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

    let full_input = format!("{}\n\n{}", prompt, context);
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
            "{} exited with status: {}",
            tool_command, status
        ))
        .into());
    }

    if !config.quiet {
        eprintln!("\n✓ {} completed successfully", tool_command);
    }

    Ok(())
}
