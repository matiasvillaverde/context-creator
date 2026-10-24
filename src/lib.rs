//! Context Creator - High-performance CLI tool to convert codebases to Markdown for LLM context
//!
//! This library provides the core functionality for traversing directories,
//! processing files, and generating formatted Markdown output suitable for
//! large language model consumption.

pub mod cli;
pub mod commands;
pub mod config;
pub mod core;
pub mod formatters;
pub mod logging;
pub mod mcp_server;
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

    // Handle commands if present
    match &config.command {
        Some(cli::Commands::Search { .. }) => return commands::run_search(config),
        Some(cli::Commands::Diff { .. }) => return commands::run_diff(config),
        Some(cli::Commands::Telemetry { .. }) => return commands::run_telemetry(config),
        Some(cli::Commands::Examples) => {
            println!("{}", cli::USAGE_EXAMPLES);
            return Ok(());
        }
        None => {} // Continue with normal processing
    }

    let generation_start = std::time::Instant::now();

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
            debug!("  Prompt length: {} bytes", prompt.len());
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
    let cache = Arc::new(FileCache::with_secret_policy(config.get_secret_policy()));

    let scan_start = std::time::Instant::now();

    // Process all directories
    let mut all_files = Vec::new();
    let mut visited = std::collections::HashSet::new();

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

        let files = collect_directory_files(
            directory,
            walk_options.clone(),
            context_options.clone(),
            cache.clone(),
            &config,
        )?;
        let label = directory
            .canonicalize()
            .ok()
            .and_then(|path| path.file_name().map(|name| name.to_os_string()))
            .unwrap_or_else(|| "root".into());
        for mut file in files {
            if !visited.insert(file.path.clone()) {
                continue;
            }
            if directories.len() > 1 {
                file.relative_path =
                    std::path::PathBuf::from(format!("{}-{}", index + 1, label.to_string_lossy()))
                        .join(&file.relative_path);
            }
            all_files.push(file);
        }
    }

    let scan_ms = scan_start.elapsed().as_secs_f64() * 1000.0;
    let discovered = all_files.len();
    let selection_start = std::time::Instant::now();
    // Select once across all roots, then count the actual final serialization.
    let mut selection_options = context_options.clone();
    if matches!(
        config.output_format,
        cli::OutputFormat::Map | cli::OutputFormat::Paths
    ) {
        selection_options.max_tokens = None;
    }
    let selected =
        core::prioritizer::prioritize_files(all_files, &selection_options, cache.clone())?;
    let selection_ms = selection_start.elapsed().as_secs_f64() * 1000.0;
    if config.progress && !config.quiet {
        info!(
            "Generating {} context from {} files...",
            format!("{:?}", config.output_format).to_lowercase(),
            selected.len()
        );
    }
    let base = if directories.len() == 1 {
        directories[0].display().to_string()
    } else {
        "multiple directories".to_string()
    };
    let mut context_options = context_options;
    if directories.len() > 1 && config.output_format == cli::OutputFormat::Markdown {
        context_options.doc_header_template = format!(
            "# Code Context - Multiple Directories\n\n{}",
            directories
                .iter()
                .map(|directory| format!("## Directory: {}\n", directory.display()))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
    let render_start = std::time::Instant::now();
    let rendered = core::context_builder::generate_digest_with_report(
        selected,
        context_options,
        cache,
        config.output_format,
        &base,
    )?;
    core::budget::check_prompt_budget(&config, &rendered.text)?;
    let rendering_ms = render_start.elapsed().as_secs_f64() * 1000.0;
    let native_start = std::time::Instant::now();
    let native = core::native_tokens::count_if_requested(&config, &rendered.text)?;
    let native_counting = native_start.elapsed().as_secs_f64() * 1000.0;
    if let Some(path) = &config.report {
        let timings = core::report::Timings {
            scan_and_analysis: scan_ms,
            selection: selection_ms,
            rendering: rendering_ms,
            native_counting,
            accounting: 0.0,
            generation_total: generation_start.elapsed().as_secs_f64() * 1000.0,
        };
        let accounting_start = std::time::Instant::now();
        let mut report =
            core::report::GenerationReport::new(&config, &rendered, discovered, native, timings)?;
        report.timings_ms.accounting = accounting_start.elapsed().as_secs_f64() * 1000.0;
        report.timings_ms.generation_total = generation_start.elapsed().as_secs_f64() * 1000.0;
        std::fs::write(path, serde_json::to_vec_pretty(&report)?)?;
    } else if let Some(native) = native {
        if !config.quiet {
            eprintln!(
                "Native {} input tokens: {} (supplied user text)",
                native.provider, native.input_tokens
            );
        }
    }
    let output = rendered.text;

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
                println!("✓ Written to {}", file.display());
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
pub(crate) fn collect_directory_files(
    path: &Path,
    walk_options: WalkOptions,
    _context_options: ContextOptions,
    cache: Arc<FileCache>,
    config: &Config,
) -> Result<Vec<core::walker::FileInfo>> {
    // Semantic expansion uses the configured seed root. Each explicit input
    // directory may belong to a different repository with its own boundary.
    let mut directory_config = config.clone();
    directory_config.paths = Some(vec![path.to_path_buf()]);
    directory_config.directories = None;
    let config = &directory_config;

    // Walk the directory
    if config.progress && !config.quiet {
        info!("Scanning directory: {}", path.display());
    }
    let mut files = core::walker::walk_directory(path, walk_options.clone())?;
    files.retain(|file| !core::report::is_artifact(config, &file.path));
    if config.output_format == cli::OutputFormat::Map && files.len() > 10_000 {
        anyhow::bail!(
            "Repository map exceeds 10000 files; narrow the input path or include patterns"
        );
    }
    if config.get_secret_policy() != core::secrets::SecretPolicy::Off {
        for file in &files {
            cache.get_or_load(&file.path)?;
        }
    }

    if config.progress && !config.quiet {
        info!("Found {} files", files.len());
    }

    let initial_paths: std::collections::HashSet<_> =
        if config.trace_imports || config.include_types || config.include_callers {
            files
                .iter()
                .filter_map(|file| file.path.canonicalize().ok())
                .collect()
        } else {
            std::collections::HashSet::new()
        };
    // Perform semantic analysis if requested
    if config.trace_imports || config.include_callers || config.include_types {
        if config.progress && !config.quiet {
            info!("Analyzing semantic dependencies...");
        }

        // Caller discovery needs a project-wide index. Import and type expansion can
        // stay on-demand, which keeps single-file runs on real projects responsive.
        let mut initial_files_map = std::collections::HashMap::new();
        let files_map = if config.include_callers {
            let project_analysis = core::project_analyzer::ProjectAnalysis::analyze_project(
                path,
                &walk_options,
                config,
                &cache,
            )?;

            for file in files {
                if let Some(analyzed_file) = project_analysis.get_file(&file.path) {
                    initial_files_map.insert(file.path.clone(), analyzed_file.clone());
                } else {
                    initial_files_map.insert(file.path.clone(), file);
                }
            }

            core::file_expander::expand_file_list_with_context(
                initial_files_map,
                config,
                &cache,
                &walk_options,
                &project_analysis.file_map,
            )?
        } else {
            for file in files {
                initial_files_map.insert(file.path.clone(), file);
            }

            core::file_expander::expand_file_list(initial_files_map, config, &cache, &walk_options)?
        };

        // Expand file list based on semantic relationships.
        if config.progress && !config.quiet {
            info!("Expanding file list based on semantic relationships...");
        }

        // Convert back to Vec<FileInfo>
        files = files_map.into_values().collect();

        // Clean up imported_by fields to only include files in our final set
        let final_paths: std::collections::HashSet<_> =
            files.iter().map(|f| f.path.clone()).collect();
        for file in &mut files {
            file.imported_by.retain(|path| final_paths.contains(path));
        }

        if config.progress && !config.quiet {
            let import_count: usize = files.iter().map(|file| file.imports.len()).sum();
            info!("Found {} import relationships in project", import_count);
            info!("Expanded to {} files", files.len());
        }
    }

    if config.output_format == cli::OutputFormat::Map
        && !config.trace_imports
        && !config.include_callers
        && !config.include_types
    {
        let mut map_config = config.clone();
        map_config.trace_imports = true;
        let root = if path.is_file() {
            path.parent().unwrap_or(path)
        } else {
            path
        };
        let root = root.canonicalize()?;
        files =
            core::project_analyzer::ProjectAnalysis::from_files(files, &root, &map_config, &cache)?
                .all_files;
    }

    if config.trace_imports || config.include_types || config.include_callers {
        let root = core::file_expander::detect_project_root(path);
        let mut visible = Vec::with_capacity(files.len());
        for file in files {
            let canonical = file.path.canonicalize()?;
            if initial_paths.contains(&canonical)
                || !core::walker::repository_path_is_ignored(&root, &canonical)?
            {
                visible.push(file);
            }
        }
        files = visible;
    }
    files.retain(|file| !core::report::is_artifact(config, &file.path));
    if config.get_secret_policy() != core::secrets::SecretPolicy::Off {
        for file in &files {
            cache.get_or_load(&file.path)?;
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

    Ok(files)
}

/// Execute LLM CLI with the generated context
fn execute_with_llm(prompt: &str, context: &str, config: &Config) -> Result<()> {
    use std::io::Write;
    use std::process::Stdio;

    // Use the new prepare_command method
    let (mut command, combined_input) = config.llm_tool.prepare_command(config)?;

    // Determine what to send to stdin based on tool requirements
    let stdin_data = if combined_input {
        format!("{prompt}\n\n{context}") // Gemini, Codex, Ollama
    } else {
        context.to_string() // Claude (prompt passed via -p flag)
    };

    let tool_command = config.llm_tool.command();

    let mut child = command
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
        stdin.write_all(stdin_data.as_bytes())?;
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
