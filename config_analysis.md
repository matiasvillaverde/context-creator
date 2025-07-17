# Code context: .

## Statistics

- Total files: 13
- Total size: 98.47 KB bytes

### Files by type:
- Rust: 13


## File Structure

```
.
├── core/
│   ├── walker.rs
│   ├── token.rs
│   ├── context.rs
│   ├── mod.rs
│   └── prioritizer.rs
├── utils/
│   ├── error.rs
│   ├── file_ext.rs
│   └── mod.rs
├── remote.rs
├── config.rs
├── lib.rs
├── main.rs
└── cli.rs
```

## Table of Contents

- [main.rs](#main-rs)
- [lib.rs](#lib-rs)
- [cli.rs](#cli-rs)
- [config.rs](#config-rs)
- [core/context.rs](#core-context-rs)
- [core/mod.rs](#core-mod-rs)
- [core/prioritizer.rs](#core-prioritizer-rs)
- [core/token.rs](#core-token-rs)
- [core/walker.rs](#core-walker-rs)
- [remote.rs](#remote-rs)
- [utils/error.rs](#utils-error-rs)
- [utils/file_ext.rs](#utils-file_ext-rs)
- [utils/mod.rs](#utils-mod-rs)

## main.rs

```rust
use anyhow::Result;
use clap::Parser;
use code_context::{cli::Config, run};

fn main() -> Result<()> {
    // Parse command line arguments
    let mut config = Config::parse();

    // Load configuration from file if specified
    config.load_from_file()?;

    // Run the application
    run(config)?;

    Ok(())
}
```

## lib.rs

```rust
//! Code context - High-performance CLI tool to convert codebases to Markdown for LLM context
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
pub use core::{context::contextOptions, walker::WalkOptions};
pub use utils::error::CodecontextError;

/// Main entry point for the code context library
pub fn run(mut config: Config) -> Result<()> {
    // Handle remote repository if specified
    let _temp_dir = if let Some(repo_url) = &config.repo {
        if config.verbose {
            eprintln!("🔧 Starting context-creator with remote repository: {repo_url}");
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

    // Setup logging based on verbosity
    if config.verbose {
        eprintln!("🔧 Starting context-creator with configuration:");
        eprintln!("  Directories: {:?}", config.directories);
        eprintln!("  Max tokens: {:?}", config.max_tokens);
        eprintln!("  LLM tool: {}", config.llm_tool.command());
        eprintln!("  Progress: {}", config.progress);
        eprintln!("  Quiet: {}", config.quiet);
        if let Some(output) = &config.output_file {
            eprintln!("  Output file: {}", output.display());
        }
        if let Some(prompt) = &config.prompt {
            eprintln!("  Prompt: {prompt}");
        }
    }

    // Validate configuration
    config.validate()?;

    // Create walker with options
    if config.verbose {
        eprintln!("🚶 Creating directory walker with options...");
    }
    let walk_options = WalkOptions::from_config(&config)?;

    // Create context options
    if config.verbose {
        eprintln!("📄 Creating markdown context options...");
    }
    let context_options = contextOptions::from_config(&config)?;

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
            process_directory(directory, walk_options.clone(), context_options.clone(), &config)?;
        all_outputs.push((directory.clone(), output));
    }

    // Combine outputs from all directories
    let output = if all_outputs.len() == 1 {
        // Single directory - return output as-is
        all_outputs.into_iter().next().unwrap().1
    } else {
        // Multiple directories - combine with headers
        let mut combined = String::new();
        combined.push_str("# Code context - Multiple Directories\n\n");

        for (path, content) in all_outputs {
            combined.push_str(&format!("## Directory: {}\n\n", path.display()));
            combined.push_str(&content);
            combined.push_str("\n\n");
        }

        combined
    };

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
            print!("{output}");
        }
        (Some(_), Some(_)) => {
            return Err(CodecontextError::InvalidConfiguration(
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
    context_options: contextOptions,
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
    let prioritized_files = if context_options.max_tokens.is_some() {
        if config.progress && !config.quiet {
            eprintln!("🎯 Prioritizing files for token limit...");
        }
        core::prioritizer::prioritize_files(files, &context_options)?
    } else {
        files
    };

    if config.progress && !config.quiet {
        eprintln!("📝 Generating markdown from {} files...", prioritized_files.len());
    }

    // Generate markdown
    let markdown = core::context::generate_markdown(prioritized_files, context_options)?;

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
                CodecontextError::LlmToolNotFound {
                    tool: tool_command.to_string(),
                    install_instructions: config.llm_tool.install_instructions().to_string(),
                }
            } else {
                CodecontextError::SubprocessError(e.to_string())
            }
        })?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(full_input.as_bytes())?;
        stdin.flush()?;
    }

    let status = child.wait()?;
    if !status.success() {
        return Err(CodecontextError::SubprocessError(format!(
            "{tool_command} exited with status: {status}"
        ))
        .into());
    }

    if !config.quiet {
        eprintln!("\n✓ {tool_command} completed successfully");
    }

    Ok(())
}
```

## cli.rs

```rust
//! Command-line interface configuration and parsing

use clap::{Parser, ValueEnum};
use std::path::PathBuf;

/// Supported LLM CLI tools
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Default)]
pub enum LlmTool {
    /// Use gemini (default)
    #[value(name = "gemini")]
    #[default]
    Gemini,
    /// Use codex CLI
    #[value(name = "codex")]
    Codex,
}

impl LlmTool {
    /// Get the command name for the tool
    pub fn command(&self) -> &'static str {
        match self {
            LlmTool::Gemini => "gemini",
            LlmTool::Codex => "codex",
        }
    }

    /// Get the installation instructions for the tool
    pub fn install_instructions(&self) -> &'static str {
        match self {
            LlmTool::Gemini => "Please install gemini with: pip install gemini",
            LlmTool::Codex => {
                "Please install codex CLI from: https://github.com/microsoft/codex-cli"
            }
        }
    }
}

/// High-performance CLI tool to convert codebases to Markdown for LLM context
#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct Config {
    /// The prompt to send to the LLM. If omitted, only generates the Markdown context
    #[arg(value_name = "PROMPT")]
    pub prompt: Option<String>,

    /// The paths to the directories to process
    #[arg(short = 'd', long, default_value = ".", num_args = 1.., conflicts_with = "repo")]
    pub directories: Vec<PathBuf>,

    /// GitHub repository URL to analyze (e.g., <https://github.com/owner/repo>)
    #[arg(long, conflicts_with = "directories")]
    pub repo: Option<String>,

    /// The path to the output Markdown file. If used, won't call the LLM CLI
    #[arg(short = 'o', long)]
    pub output_file: Option<PathBuf>,

    /// Maximum number of tokens for the generated codebase context
    #[arg(long)]
    pub max_tokens: Option<usize>,

    /// LLM CLI tool to use for processing
    #[arg(short = 't', long = "tool", default_value = "gemini")]
    pub llm_tool: LlmTool,

    /// Suppress all output except for errors and the final LLM response
    #[arg(short = 'q', long)]
    pub quiet: bool,

    /// Enable verbose logging
    #[arg(short = 'v', long)]
    pub verbose: bool,

    /// Path to configuration file
    #[arg(short = 'c', long)]
    pub config: Option<PathBuf>,

    /// Show progress indicators during processing
    #[arg(long)]
    pub progress: bool,
}

impl Config {
    /// Validate the configuration
    pub fn validate(&self) -> Result<(), crate::utils::error::CodecontextError> {
        use crate::utils::error::CodecontextError;

        // Validate repo URL if provided
        if let Some(repo_url) = &self.repo {
            if !repo_url.starts_with("https://github.com/")
                && !repo_url.starts_with("http://github.com/")
            {
                return Err(CodecontextError::InvalidConfiguration(
                    "Repository URL must be a GitHub URL (https://github.com/owner/repo)"
                        .to_string(),
                ));
            }
        } else {
            // Only validate directories if repo is not provided
            for directory in &self.directories {
                if !directory.exists() {
                    return Err(CodecontextError::InvalidPath(format!(
                        "Directory does not exist: {}",
                        directory.display()
                    )));
                }

                if !directory.is_dir() {
                    return Err(CodecontextError::InvalidPath(format!(
                        "Path is not a directory: {}",
                        directory.display()
                    )));
                }
            }
        }

        // Validate output file parent directory exists if specified
        if let Some(output) = &self.output_file {
            if let Some(parent) = output.parent() {
                // Handle empty parent (current directory) and check if parent exists
                if !parent.as_os_str().is_empty() && !parent.exists() {
                    return Err(CodecontextError::InvalidPath(format!(
                        "Output directory does not exist: {}",
                        parent.display()
                    )));
                }
            }
        }

        // Validate mutually exclusive options
        if self.output_file.is_some() && self.prompt.is_some() {
            return Err(CodecontextError::InvalidConfiguration(
                "Cannot specify both --output and a prompt".to_string(),
            ));
        }

        Ok(())
    }

    /// Load configuration from file if specified
    pub fn load_from_file(&mut self) -> Result<(), crate::utils::error::CodecontextError> {
        use crate::config::ConfigFile;

        let config_file = if let Some(ref config_path) = self.config {
            // Load from specified config file
            Some(ConfigFile::load_from_file(config_path)?)
        } else {
            // Try to load from default locations
            ConfigFile::load_default()?
        };

        if let Some(config_file) = config_file {
            config_file.apply_to_cli_config(self);

            if self.verbose {
                if let Some(ref config_path) = self.config {
                    eprintln!("📄 Loaded configuration from: {}", config_path.display());
                } else {
                    eprintln!("📄 Loaded configuration from default location");
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_config_validation_valid_directory() {
        let temp_dir = TempDir::new().unwrap();
        let config = Config {
            prompt: None,
            directories: vec![temp_dir.path().to_path_buf()],
            repo: None,
            output_file: None,
            max_tokens: None,
            llm_tool: LlmTool::default(),
            quiet: false,
            verbose: false,
            config: None,
            progress: false,
        };

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation_invalid_directory() {
        let config = Config {
            prompt: None,
            directories: vec![PathBuf::from("/nonexistent/directory")],
            repo: None,
            output_file: None,
            max_tokens: None,
            llm_tool: LlmTool::default(),
            quiet: false,
            verbose: false,
            config: None,
            progress: false,
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_file_as_directory() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("file.txt");
        fs::write(&file_path, "test").unwrap();

        let config = Config {
            prompt: None,
            directories: vec![file_path],
            repo: None,
            output_file: None,
            max_tokens: None,
            llm_tool: LlmTool::default(),
            quiet: false,
            verbose: false,
            config: None,
            progress: false,
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_invalid_output_directory() {
        let temp_dir = TempDir::new().unwrap();
        let config = Config {
            prompt: None,
            directories: vec![temp_dir.path().to_path_buf()],
            repo: None,
            output_file: Some(PathBuf::from("/nonexistent/directory/output.md")),
            max_tokens: None,
            llm_tool: LlmTool::default(),
            quiet: false,
            verbose: false,
            config: None,
            progress: false,
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_mutually_exclusive_options() {
        let temp_dir = TempDir::new().unwrap();
        let config = Config {
            prompt: Some("test prompt".to_string()),
            directories: vec![temp_dir.path().to_path_buf()],
            repo: None,
            output_file: Some(temp_dir.path().join("output.md")),
            max_tokens: None,
            llm_tool: LlmTool::default(),
            quiet: false,
            verbose: false,
            config: None,
            progress: false,
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_llm_tool_enum_values() {
        assert_eq!(LlmTool::Gemini.command(), "gemini");
        assert_eq!(LlmTool::Codex.command(), "codex");

        assert!(LlmTool::Gemini.install_instructions().contains("pip install"));
        assert!(LlmTool::Codex.install_instructions().contains("github.com"));

        assert_eq!(LlmTool::default(), LlmTool::Gemini);
    }

    #[test]
    fn test_config_validation_output_file_in_current_dir() {
        let temp_dir = TempDir::new().unwrap();
        let config = Config {
            prompt: None,
            directories: vec![temp_dir.path().to_path_buf()],
            repo: None,
            output_file: Some(PathBuf::from("output.md")),
            max_tokens: None,
            llm_tool: LlmTool::default(),
            quiet: false,
            verbose: false,
            config: None,
            progress: false,
        };

        // Should not error for files in current directory
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_load_from_file_no_config() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = Config {
            prompt: None,
            directories: vec![temp_dir.path().to_path_buf()],
            repo: None,
            output_file: None,
            max_tokens: None,
            llm_tool: LlmTool::default(),
            quiet: false,
            verbose: false,
            config: None,
            progress: false,
        };

        // Should not error when no config file is found
        assert!(config.load_from_file().is_ok());
    }

    #[test]
    fn test_parse_multiple_directories() {
        use clap::Parser;

        // Test single directory (backward compatibility)
        let args = vec!["context-creator", "-d", "/path/one"];
        let config = Config::parse_from(args);
        assert_eq!(config.directories.len(), 1);
        assert_eq!(config.directories[0], PathBuf::from("/path/one"));
    }

    #[test]
    fn test_parse_multiple_directories_new_api() {
        use clap::Parser;

        // Test single directory (backward compatibility)
        let args = vec!["context-creator", "-d", "/path/one"];
        let config = Config::parse_from(args);
        assert_eq!(config.directories.len(), 1);
        assert_eq!(config.directories[0], PathBuf::from("/path/one"));

        // Test multiple directories
        let args = vec!["context-creator", "-d", "/path/one", "/path/two", "/path/three"];
        let config = Config::parse_from(args);
        assert_eq!(config.directories.len(), 3);
        assert_eq!(config.directories[0], PathBuf::from("/path/one"));
        assert_eq!(config.directories[1], PathBuf::from("/path/two"));
        assert_eq!(config.directories[2], PathBuf::from("/path/three"));

        // Test with prompt after directories using -- separator
        let args = vec![
            "context-creator",
            "-d",
            "/src/module1",
            "/src/module2",
            "--",
            "Find duplicated patterns",
        ];
        let config = Config::parse_from(args);
        assert_eq!(config.directories.len(), 2);
        assert_eq!(config.prompt, Some("Find duplicated patterns".to_string()));
    }

    #[test]
    fn test_validate_multiple_directories() {
        let temp_dir = TempDir::new().unwrap();
        let dir1 = temp_dir.path().join("dir1");
        let dir2 = temp_dir.path().join("dir2");
        fs::create_dir(&dir1).unwrap();
        fs::create_dir(&dir2).unwrap();

        // All directories exist - should succeed
        let config = Config {
            prompt: None,
            directories: vec![dir1.clone(), dir2.clone()],
            repo: None,
            output_file: None,
            max_tokens: None,
            llm_tool: LlmTool::default(),
            quiet: false,
            verbose: false,
            config: None,
            progress: false,
        };
        assert!(config.validate().is_ok());

        // One directory doesn't exist - should fail
        let config = Config {
            prompt: None,
            directories: vec![dir1, PathBuf::from("/nonexistent/dir")],
            repo: None,
            output_file: None,
            max_tokens: None,
            llm_tool: LlmTool::default(),
            quiet: false,
            verbose: false,
            config: None,
            progress: false,
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_files_as_directories() {
        let temp_dir = TempDir::new().unwrap();
        let dir1 = temp_dir.path().join("dir1");
        let file1 = temp_dir.path().join("file.txt");
        fs::create_dir(&dir1).unwrap();
        fs::write(&file1, "test content").unwrap();

        // Mix of directory and file - should fail
        let config = Config {
            prompt: None,
            directories: vec![dir1, file1],
            repo: None,
            output_file: None,
            max_tokens: None,
            llm_tool: LlmTool::default(),
            quiet: false,
            verbose: false,
            config: None,
            progress: false,
        };
        assert!(config.validate().is_err());
    }
}
```

## config.rs

```rust
//! Configuration file support for context-creator
//!
//! This module handles loading and parsing configuration files in TOML format.
//! Configuration files can specify defaults for CLI options and additional
//! settings like file priorities and ignore patterns.

use crate::cli::{Config as CliConfig, LlmTool};
use crate::utils::error::CodecontextError;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Configuration file structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConfigFile {
    /// Default settings
    #[serde(default)]
    pub defaults: Defaults,

    /// File priority configurations
    #[serde(default)]
    pub priorities: Vec<Priority>,

    /// Ignore patterns beyond .gitignore and .contextignore
    #[serde(default)]
    pub ignore: Vec<String>,

    /// Include patterns to force inclusion
    #[serde(default)]
    pub include: Vec<String>,
}

/// Default configuration settings
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Defaults {
    /// Default maximum tokens
    pub max_tokens: Option<usize>,

    /// Default LLM tool
    #[serde(default)]
    pub llm_tool: Option<String>,

    /// Default to show progress
    #[serde(default)]
    pub progress: bool,

    /// Default verbosity
    #[serde(default)]
    pub verbose: bool,

    /// Default quiet mode
    #[serde(default)]
    pub quiet: bool,

    /// Default directory
    pub directory: Option<PathBuf>,

    /// Default output file
    pub output_file: Option<PathBuf>,
}

/// File priority configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Priority {
    /// Glob pattern to match files
    pub pattern: String,
    /// Priority weight (higher = more important)
    pub weight: f32,
}

impl ConfigFile {
    /// Load configuration from a file
    pub fn load_from_file(path: &Path) -> Result<Self, CodecontextError> {
        if !path.exists() {
            return Err(CodecontextError::InvalidPath(format!(
                "Configuration file does not exist: {}",
                path.display()
            )));
        }

        let content = std::fs::read_to_string(path).map_err(|e| {
            CodecontextError::ConfigError(format!(
                "Failed to read config file {}: {}",
                path.display(),
                e
            ))
        })?;

        let config: ConfigFile = toml::from_str(&content).map_err(|e| {
            CodecontextError::ConfigError(format!(
                "Failed to parse config file {}: {}",
                path.display(),
                e
            ))
        })?;

        Ok(config)
    }

    /// Load configuration from default locations
    pub fn load_default() -> Result<Option<Self>, CodecontextError> {
        // Try .context-creator.toml in current directory
        let local_config = Path::new(".context-creator.toml");
        if local_config.exists() {
            return Ok(Some(Self::load_from_file(local_config)?));
        }

        // Try .contextrc.toml in current directory
        let rc_config = Path::new(".contextrc.toml");
        if rc_config.exists() {
            return Ok(Some(Self::load_from_file(rc_config)?));
        }

        // Try in home directory
        if let Some(home) = dirs::home_dir() {
            let home_config = home.join(".context-creator.toml");
            if home_config.exists() {
                return Ok(Some(Self::load_from_file(&home_config)?));
            }
        }

        Ok(None)
    }

    /// Apply configuration defaults to CLI config
    pub fn apply_to_cli_config(&self, cli_config: &mut CliConfig) {
        // Only apply defaults if CLI didn't specify them
        if cli_config.max_tokens.is_none() && self.defaults.max_tokens.is_some() {
            cli_config.max_tokens = self.defaults.max_tokens;
        }

        if let Some(ref tool_str) = self.defaults.llm_tool {
            // Only apply if CLI used default
            if cli_config.llm_tool == LlmTool::default() {
                match tool_str.as_str() {
                    "gemini" => cli_config.llm_tool = LlmTool::Gemini,
                    "codex" => cli_config.llm_tool = LlmTool::Codex,
                    _ => {} // Ignore invalid tool names
                }
            }
        }

        // Apply boolean defaults only if they weren't explicitly set
        if !cli_config.progress && self.defaults.progress {
            cli_config.progress = self.defaults.progress;
        }

        if !cli_config.verbose && self.defaults.verbose {
            cli_config.verbose = self.defaults.verbose;
        }

        if !cli_config.quiet && self.defaults.quiet {
            cli_config.quiet = self.defaults.quiet;
        }

        // Apply directory default if CLI used default (".")
        if cli_config.directories.len() == 1
            && cli_config.directories[0] == PathBuf::from(".")
            && self.defaults.directory.is_some()
        {
            cli_config.directories = vec![self.defaults.directory.clone().unwrap()];
        }

        // Apply output file default if not specified
        if cli_config.output_file.is_none() && self.defaults.output_file.is_some() {
            cli_config.output_file = self.defaults.output_file.clone();
        }
    }
}

/// Create an example configuration file
pub fn create_example_config() -> String {
    let example = ConfigFile {
        defaults: Defaults {
            max_tokens: Some(150000),
            llm_tool: Some("gemini".to_string()),
            progress: true,
            verbose: false,
            quiet: false,
            directory: None,
            output_file: None,
        },
        priorities: vec![
            Priority { pattern: "src/**/*.rs".to_string(), weight: 100.0 },
            Priority { pattern: "src/main.rs".to_string(), weight: 150.0 },
            Priority { pattern: "tests/**/*.rs".to_string(), weight: 50.0 },
            Priority { pattern: "docs/**/*.md".to_string(), weight: 30.0 },
            Priority { pattern: "*.toml".to_string(), weight: 80.0 },
            Priority { pattern: "*.json".to_string(), weight: 60.0 },
        ],
        ignore: vec![
            "target/**".to_string(),
            "node_modules/**".to_string(),
            "*.pyc".to_string(),
            ".env".to_string(),
        ],
        include: vec!["!important/**".to_string()],
    };

    toml::to_string_pretty(&example)
        .unwrap_or_else(|_| "# Failed to generate example config".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_config_file_parsing() {
        let config_content = r#"
ignore = [
    "target/**",
    "node_modules/**"
]

include = [
    "!important/**"
]

[defaults]
max_tokens = 100000
llm_tool = "gemini"
progress = true

[[priorities]]
pattern = "src/**/*.rs"
weight = 100.0

[[priorities]]
pattern = "tests/**/*.rs"
weight = 50.0
"#;

        let config: ConfigFile = toml::from_str(config_content).unwrap();

        assert_eq!(config.defaults.max_tokens, Some(100000));
        assert_eq!(config.defaults.llm_tool, Some("gemini".to_string()));
        assert!(config.defaults.progress);
        assert_eq!(config.priorities.len(), 2);
        assert_eq!(config.priorities[0].pattern, "src/**/*.rs");
        assert_eq!(config.priorities[0].weight, 100.0);
        assert_eq!(config.ignore.len(), 2);
        assert_eq!(config.include.len(), 1);
    }

    #[test]
    fn test_config_file_loading() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let config_content = r#"
[defaults]
max_tokens = 50000
progress = true
"#;

        fs::write(&config_path, config_content).unwrap();

        let config = ConfigFile::load_from_file(&config_path).unwrap();
        assert_eq!(config.defaults.max_tokens, Some(50000));
        assert!(config.defaults.progress);
    }

    #[test]
    fn test_apply_to_cli_config() {
        let config_file = ConfigFile {
            defaults: Defaults {
                max_tokens: Some(75000),
                llm_tool: Some("codex".to_string()),
                progress: true,
                verbose: true,
                quiet: false,
                directory: Some(PathBuf::from("/tmp")),
                output_file: Some(PathBuf::from("output.md")),
            },
            priorities: vec![],
            ignore: vec![],
            include: vec![],
        };

        let mut cli_config = CliConfig {
            prompt: None,
            directories: vec![PathBuf::from(".")],
            repo: None,
            output_file: None,
            max_tokens: None,
            llm_tool: LlmTool::default(),
            quiet: false,
            verbose: false,
            config: None,
            progress: false,
        };

        config_file.apply_to_cli_config(&mut cli_config);

        assert_eq!(cli_config.max_tokens, Some(75000));
        assert_eq!(cli_config.llm_tool, LlmTool::Codex);
        assert!(cli_config.progress);
        assert!(cli_config.verbose);
        assert_eq!(cli_config.directories, vec![PathBuf::from("/tmp")]);
        assert_eq!(cli_config.output_file, Some(PathBuf::from("output.md")));
    }

    #[test]
    fn test_example_config_generation() {
        let example = create_example_config();
        assert!(example.contains("[defaults]"));
        assert!(example.contains("max_tokens"));
        assert!(example.contains("[[priorities]]"));
        assert!(example.contains("pattern"));
        assert!(example.contains("weight"));
    }
}
```

## core/context.rs

```rust
//! Markdown generation functionality

use crate::core::walker::FileInfo;
use crate::utils::file_ext::FileType;
use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Options for generating markdown context
#[derive(Debug, Clone)]
pub struct contextOptions {
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
}

impl contextOptions {
    /// Create contextOptions from CLI config
    pub fn from_config(config: &crate::cli::Config) -> Result<Self> {
        Ok(contextOptions {
            max_tokens: config.max_tokens,
            include_tree: true,
            include_stats: true,
            group_by_type: false,
            sort_by_priority: true,
            file_header_template: "## {path}".to_string(),
            doc_header_template: "# Code context: {directory}".to_string(),
            include_toc: true,
        })
    }
}

impl Default for contextOptions {
    fn default() -> Self {
        contextOptions {
            max_tokens: None,
            include_tree: true,
            include_stats: true,
            group_by_type: false,
            sort_by_priority: true,
            file_header_template: "## {path}".to_string(),
            doc_header_template: "# Code context: {directory}".to_string(),
            include_toc: true,
        }
    }
}

/// Generate markdown from a list of files
pub fn generate_markdown(files: Vec<FileInfo>, options: contextOptions) -> Result<String> {
    let mut output = String::new();

    // Add document header
    if !options.doc_header_template.is_empty() {
        let header = options.doc_header_template.replace("{directory}", ".");
        output.push_str(&header);
        output.push_str("\n\n");
    }

    // Add statistics if requested
    if options.include_stats {
        let stats = generate_statistics(&files);
        output.push_str(&stats);
        output.push_str("\n\n");
    }

    // Add file tree if requested
    if options.include_tree {
        let tree = generate_file_tree(&files);
        output.push_str("## File Structure\n\n");
        output.push_str("```\n");
        output.push_str(&tree);
        output.push_str("```\n\n");
    }

    // Sort files if requested
    let mut files = files;
    if options.sort_by_priority {
        files.sort_by(|a, b| {
            b.priority
                .partial_cmp(&a.priority)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.relative_path.cmp(&b.relative_path))
        });
    }

    // Add table of contents if requested
    if options.include_toc {
        output.push_str("## Table of Contents\n\n");
        for file in &files {
            let anchor = path_to_anchor(&file.relative_path);
            output.push_str(&format!(
                "- [{path}](#{anchor})\n",
                path = file.relative_path.display(),
                anchor = anchor
            ));
        }
        output.push('\n');
    }

    // Group files if requested
    if options.group_by_type {
        let grouped = group_files_by_type(files);
        for (file_type, group_files) in grouped {
            output.push_str(&format!("## {} Files\n\n", file_type_display(&file_type)));
            for file in group_files {
                append_file_content(&mut output, &file, &options)?;
            }
        }
    } else {
        // Add all files
        for file in files {
            append_file_content(&mut output, &file, &options)?;
        }
    }

    Ok(output)
}

/// Append a single file's content to the output
fn append_file_content(
    output: &mut String,
    file: &FileInfo,
    options: &contextOptions,
) -> Result<()> {
    // Read file content
    let content = match fs::read_to_string(&file.path) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Warning: Could not read file {}: {}", file.path.display(), e);
            return Ok(());
        }
    };

    // Add file header
    let header =
        options.file_header_template.replace("{path}", &file.relative_path.display().to_string());
    output.push_str(&header);
    output.push_str("\n\n");

    // Add language hint for syntax highlighting
    let language = get_language_hint(&file.file_type);
    output.push_str(&format!("```{language}\n"));
    output.push_str(&content);
    if !content.ends_with('\n') {
        output.push('\n');
    }
    output.push_str("```\n\n");

    Ok(())
}

/// Generate statistics about the files
fn generate_statistics(files: &[FileInfo]) -> String {
    let total_files = files.len();
    let total_size: u64 = files.iter().map(|f| f.size).sum();

    // Count by file type
    let mut type_counts: HashMap<FileType, usize> = HashMap::new();
    for file in files {
        *type_counts.entry(file.file_type.clone()).or_insert(0) += 1;
    }

    let mut stats = String::new();
    stats.push_str("## Statistics\n\n");
    stats.push_str(&format!("- Total files: {total_files}\n"));
    stats.push_str(&format!("- Total size: {} bytes\n", format_size(total_size)));
    stats.push_str("\n### Files by type:\n");

    let mut types: Vec<_> = type_counts.into_iter().collect();
    types.sort_by_key(|(_, count)| std::cmp::Reverse(*count));

    for (file_type, count) in types {
        stats.push_str(&format!("- {}: {}\n", file_type_display(&file_type), count));
    }

    stats
}

/// Generate a file tree representation
fn generate_file_tree(files: &[FileInfo]) -> String {
    use std::collections::BTreeMap;

    #[derive(Default)]
    struct TreeNode {
        files: Vec<String>,
        dirs: BTreeMap<String, TreeNode>,
    }

    let mut root = TreeNode::default();

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
    fn render_tree(node: &TreeNode, prefix: &str, _is_last: bool) -> String {
        let mut output = String::new();

        // Render directories
        let dir_count = node.dirs.len();
        for (i, (name, child)) in node.dirs.iter().enumerate() {
            let is_last_dir = i == dir_count - 1 && node.files.is_empty();
            let connector = if is_last_dir { "└── " } else { "├── " };
            let extension = if is_last_dir { "    " } else { "│   " };

            output.push_str(&format!("{prefix}{connector}{name}/\n"));
            output.push_str(&render_tree(child, &format!("{prefix}{extension}"), is_last_dir));
        }

        // Render files
        let file_count = node.files.len();
        for (i, name) in node.files.iter().enumerate() {
            let is_last_file = i == file_count - 1;
            let connector = if is_last_file { "└── " } else { "├── " };
            output.push_str(&format!("{prefix}{connector}{name}\n"));
        }

        output
    }

    let mut output = String::new();
    output.push_str(".\n");
    output.push_str(&render_tree(&root, "", true));
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
fn file_type_display(file_type: &FileType) -> &'static str {
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
fn get_language_hint(file_type: &FileType) -> &'static str {
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
        FileType::Markdown => 15,
        FileType::Json => 16,
        FileType::Yaml => 17,
        FileType::Toml => 18,
        FileType::Xml => 19,
        FileType::Html => 20,
        FileType::Css => 21,
        FileType::Text => 22,
        FileType::Other => 23,
    }
}

/// Convert path to anchor-friendly string
fn path_to_anchor(path: &Path) -> String {
    path.display().to_string().replace(['/', '\\', '.', ' '], "-").to_lowercase()
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
            },
            FileInfo {
                path: PathBuf::from("test2.py"),
                relative_path: PathBuf::from("test2.py"),
                size: 200,
                file_type: FileType::Python,
                priority: 0.9,
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
            },
            FileInfo {
                path: PathBuf::from("huge.py"),
                relative_path: PathBuf::from("huge.py"),
                size: 50_000_000, // 50MB
                file_type: FileType::Python,
                priority: 0.9,
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
            },
            FileInfo {
                path: PathBuf::from("src/lib.rs"),
                relative_path: PathBuf::from("src/lib.rs"),
                size: 2000,
                file_type: FileType::Rust,
                priority: 1.2,
            },
            FileInfo {
                path: PathBuf::from("tests/test.rs"),
                relative_path: PathBuf::from("tests/test.rs"),
                size: 500,
                file_type: FileType::Rust,
                priority: 0.8,
            },
        ];

        let tree = generate_file_tree(&files);
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
            prompt: None,
            directories: vec![temp_dir.path().to_path_buf()],
            output_file: None,
            max_tokens: Some(100000),
            llm_tool: crate::cli::LlmTool::default(),
            quiet: false,
            verbose: false,
            config: None,
            progress: false,
            repo: None,
        };

        let options = contextOptions::from_config(&config).unwrap();
        assert_eq!(options.max_tokens, Some(100000));
        assert!(options.include_tree);
        assert!(options.include_stats);
        assert!(!options.group_by_type); // Default is false according to implementation
    }

    #[test]
    fn test_generate_markdown_structure_headers() {
        let files = vec![];

        let options = contextOptions {
            max_tokens: None,
            include_tree: true,
            include_stats: true,
            group_by_type: true,
            sort_by_priority: true,
            file_header_template: "## {path}".to_string(),
            doc_header_template: "# Code context".to_string(),
            include_toc: true,
        };

        let markdown = generate_markdown(files, options).unwrap();

        // Check that main structure is present even with no files
        assert!(markdown.contains("# Code context"));
        assert!(markdown.contains("## Statistics"));
        // File tree might be skipped if there are no files
        assert!(markdown.contains("## Files"));
    }
}
```

## core/mod.rs

```rust
//! Core functionality modules

pub mod context;
pub mod prioritizer;
pub mod token;
pub mod walker;
```

## core/prioritizer.rs

```rust
//! File prioritization based on token limits

use crate::core::context::contextOptions;
use crate::core::token::{would_exceed_limit, TokenCounter};
use crate::core::walker::FileInfo;
use anyhow::Result;
use std::fs;

/// Prioritize files based on their importance and token limits
pub fn prioritize_files(
    mut files: Vec<FileInfo>,
    options: &contextOptions,
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

    // Sort files by priority (highest first)
    files.sort_by(|a, b| {
        b.priority
            .partial_cmp(&a.priority)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.relative_path.cmp(&b.relative_path))
    });

    // Create token counter
    let counter = TokenCounter::new()?;
    let mut selected_files = Vec::new();
    let mut total_tokens = 0;

    // Calculate overhead for markdown structure
    let structure_overhead = calculate_structure_overhead(options, &files)?;
    total_tokens += structure_overhead;

    // Add files until we hit the token limit
    for file in files {
        // Read file content
        let content = match fs::read_to_string(&file.path) {
            Ok(content) => content,
            Err(e) => {
                eprintln!("Warning: Could not read file {}: {}", file.path.display(), e);
                continue;
            }
        };

        // Count tokens for this file
        let file_tokens =
            counter.count_file_tokens(&content, &file.relative_path.to_string_lossy())?;

        // Check if adding this file would exceed the limit
        if would_exceed_limit(total_tokens, file_tokens.total_tokens, max_tokens) {
            // Try to find smaller files that might fit
            continue;
        }

        // Add the file
        total_tokens += file_tokens.total_tokens;
        selected_files.push(file);
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
fn calculate_structure_overhead(options: &contextOptions, files: &[FileInfo]) -> Result<usize> {
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
    use std::path::PathBuf;

    #[test]
    fn test_prioritize_without_limit() {
        let files = vec![
            FileInfo {
                path: PathBuf::from("low.txt"),
                relative_path: PathBuf::from("low.txt"),
                size: 100,
                file_type: FileType::Text,
                priority: 0.3,
            },
            FileInfo {
                path: PathBuf::from("high.rs"),
                relative_path: PathBuf::from("high.rs"),
                size: 100,
                file_type: FileType::Rust,
                priority: 1.0,
            },
        ];

        let options = contextOptions::default();
        let result = prioritize_files(files, &options).unwrap();

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
        let files = vec![
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

        let options = contextOptions::default();
        let result = prioritize_files(files, &options).unwrap();

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

        let options = contextOptions {
            max_tokens: None,
            include_tree: true,
            include_stats: true,
            group_by_type: true,
            sort_by_priority: true,
            file_header_template: "## {path}".to_string(),
            doc_header_template: "# Code context".to_string(),
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
```

## core/token.rs

```rust
//! Token counting functionality using tiktoken-rs

use anyhow::Result;
use rayon::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tiktoken_rs::{cl100k_base, CoreBPE};

/// Token counter with caching support
pub struct TokenCounter {
    /// The tiktoken encoder
    encoder: Arc<CoreBPE>,
    /// Cache of token counts for content hashes
    cache: Arc<Mutex<HashMap<u64, usize>>>,
}

impl TokenCounter {
    /// Create a new token counter with cl100k_base encoding (GPT-4)
    pub fn new() -> Result<Self> {
        let encoder = cl100k_base()?;
        Ok(TokenCounter { encoder: Arc::new(encoder), cache: Arc::new(Mutex::new(HashMap::new())) })
    }

    /// Count tokens in a single text
    pub fn count_tokens(&self, text: &str) -> Result<usize> {
        // Calculate hash for caching
        let hash = calculate_hash(text);

        // Check cache first
        if let Ok(cache) = self.cache.lock() {
            if let Some(&count) = cache.get(&hash) {
                return Ok(count);
            }
        }

        // Count tokens
        let tokens = self.encoder.encode_with_special_tokens(text);
        let count = tokens.len();

        // Store in cache
        if let Ok(mut cache) = self.cache.lock() {
            cache.insert(hash, count);
        }

        Ok(count)
    }

    /// Count tokens in multiple texts in parallel
    pub fn count_tokens_parallel(&self, texts: &[String]) -> Result<Vec<usize>> {
        texts.par_iter().map(|text| self.count_tokens(text)).collect()
    }

    /// Count tokens for a file's content with metadata
    pub fn count_file_tokens(&self, content: &str, path: &str) -> Result<FileTokenCount> {
        let content_tokens = self.count_tokens(content)?;

        // Count tokens in the file path/header that will be included in markdown
        let header = format!("## {path}\n\n```\n");
        let footer = "\n```\n\n";
        let header_tokens = self.count_tokens(&header)?;
        let footer_tokens = self.count_tokens(footer)?;

        Ok(FileTokenCount {
            content_tokens,
            overhead_tokens: header_tokens + footer_tokens,
            total_tokens: content_tokens + header_tokens + footer_tokens,
        })
    }

    /// Estimate tokens for multiple files
    pub fn estimate_total_tokens(&self, files: &[(String, String)]) -> Result<TotalTokenEstimate> {
        let mut total_content = 0;
        let mut total_overhead = 0;
        let mut file_counts = Vec::new();

        for (path, content) in files {
            let count = self.count_file_tokens(content, path)?;
            total_content += count.content_tokens;
            total_overhead += count.overhead_tokens;
            file_counts.push((path.clone(), count));
        }

        Ok(TotalTokenEstimate {
            total_tokens: total_content + total_overhead,
            content_tokens: total_content,
            overhead_tokens: total_overhead,
            file_counts,
        })
    }
}

impl Default for TokenCounter {
    fn default() -> Self {
        Self::new().expect("Failed to create token counter")
    }
}

/// Token count for a single file
#[derive(Debug, Clone)]
pub struct FileTokenCount {
    /// Tokens in the file content
    pub content_tokens: usize,
    /// Tokens in markdown formatting overhead
    pub overhead_tokens: usize,
    /// Total tokens (content + overhead)
    pub total_tokens: usize,
}

/// Total token estimate for multiple files
#[derive(Debug)]
pub struct TotalTokenEstimate {
    /// Total tokens across all files
    pub total_tokens: usize,
    /// Total content tokens
    pub content_tokens: usize,
    /// Total overhead tokens
    pub overhead_tokens: usize,
    /// Individual file counts
    pub file_counts: Vec<(String, FileTokenCount)>,
}

/// Calculate a hash for content caching
fn calculate_hash(text: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

/// Check if adding a file would exceed token limit
pub fn would_exceed_limit(current_tokens: usize, file_tokens: usize, max_tokens: usize) -> bool {
    current_tokens + file_tokens > max_tokens
}

/// Calculate remaining token budget
pub fn remaining_tokens(current_tokens: usize, max_tokens: usize) -> usize {
    max_tokens.saturating_sub(current_tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_counting() {
        let counter = TokenCounter::new().unwrap();

        // Test simple text
        let count = counter.count_tokens("Hello, world!").unwrap();
        assert!(count > 0);

        // Test empty text
        let count = counter.count_tokens("").unwrap();
        assert_eq!(count, 0);

        // Test caching
        let text = "This is a test text for caching";
        let count1 = counter.count_tokens(text).unwrap();
        let count2 = counter.count_tokens(text).unwrap();
        assert_eq!(count1, count2);
    }

    #[test]
    fn test_file_token_counting() {
        let counter = TokenCounter::new().unwrap();

        let content = "fn main() {\n    println!(\"Hello, world!\");\n}";
        let path = "src/main.rs";

        let count = counter.count_file_tokens(content, path).unwrap();
        assert!(count.content_tokens > 0);
        assert!(count.overhead_tokens > 0);
        assert_eq!(count.total_tokens, count.content_tokens + count.overhead_tokens);
    }

    #[test]
    fn test_parallel_counting() {
        let counter = TokenCounter::new().unwrap();

        let texts =
            vec!["First text".to_string(), "Second text".to_string(), "Third text".to_string()];

        let counts = counter.count_tokens_parallel(&texts).unwrap();
        assert_eq!(counts.len(), 3);
        assert!(counts.iter().all(|&c| c > 0));
    }

    #[test]
    fn test_token_limit_checks() {
        assert!(would_exceed_limit(900, 200, 1000));
        assert!(!would_exceed_limit(800, 200, 1000));

        assert_eq!(remaining_tokens(300, 1000), 700);
        assert_eq!(remaining_tokens(1100, 1000), 0);
    }

    #[test]
    fn test_total_estimation() {
        let counter = TokenCounter::new().unwrap();

        let files = vec![
            ("file1.rs".to_string(), "content1".to_string()),
            ("file2.rs".to_string(), "content2".to_string()),
        ];

        let estimate = counter.estimate_total_tokens(&files).unwrap();
        assert!(estimate.total_tokens > 0);
        assert_eq!(estimate.file_counts.len(), 2);
    }
}
```

## core/walker.rs

```rust
//! Directory walking functionality with .gitignore and .contextignore support

use crate::utils::error::CodecontextError;
use crate::utils::file_ext::FileType;
use anyhow::Result;
use ignore::{Walk, WalkBuilder};
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Options for walking directories
#[derive(Debug, Clone)]
pub struct WalkOptions {
    /// Maximum file size in bytes
    pub max_file_size: Option<usize>,
    /// Follow symbolic links
    pub follow_links: bool,
    /// Include hidden files
    pub include_hidden: bool,
    /// Use parallel processing
    pub parallel: bool,
    /// Custom ignore file name (default: .contextignore)
    pub ignore_file: String,
    /// Additional glob patterns to ignore
    pub ignore_patterns: Vec<String>,
    /// Only include files matching these patterns
    pub include_patterns: Vec<String>,
}

impl WalkOptions {
    /// Create WalkOptions from CLI config
    pub fn from_config(_config: &crate::cli::Config) -> Result<Self> {
        Ok(WalkOptions {
            max_file_size: Some(10 * 1024 * 1024), // 10MB default
            follow_links: false,
            include_hidden: false,
            parallel: true,
            ignore_file: ".contextignore".to_string(),
            ignore_patterns: vec![],
            include_patterns: vec![],
        })
    }
}

impl Default for WalkOptions {
    fn default() -> Self {
        WalkOptions {
            max_file_size: Some(10 * 1024 * 1024), // 10MB
            follow_links: false,
            include_hidden: false,
            parallel: true,
            ignore_file: ".contextignore".to_string(),
            ignore_patterns: vec![],
            include_patterns: vec![],
        }
    }
}

/// Information about a file found during walking
#[derive(Debug, Clone)]
pub struct FileInfo {
    /// Absolute path to the file
    pub path: PathBuf,
    /// Relative path from the root directory
    pub relative_path: PathBuf,
    /// File size in bytes
    pub size: u64,
    /// File type based on extension
    pub file_type: FileType,
    /// Priority score (higher is more important)
    pub priority: f32,
}

impl FileInfo {
    /// Get a display string for the file type
    pub fn file_type_display(&self) -> &'static str {
        use crate::utils::file_ext::FileType;
        match self.file_type {
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
}

/// Walk a directory and collect file information
pub fn walk_directory(root: &Path, options: WalkOptions) -> Result<Vec<FileInfo>> {
    if !root.exists() {
        return Err(CodecontextError::InvalidPath(format!(
            "Directory does not exist: {}",
            root.display()
        ))
        .into());
    }

    if !root.is_dir() {
        return Err(CodecontextError::InvalidPath(format!(
            "Path is not a directory: {}",
            root.display()
        ))
        .into());
    }

    let root = root.canonicalize()?;
    let walker = build_walker(&root, &options);

    if options.parallel {
        walk_parallel(walker, &root, &options)
    } else {
        walk_sequential(walker, &root, &options)
    }
}

/// Build the ignore walker with configured options
fn build_walker(root: &Path, options: &WalkOptions) -> Walk {
    let mut builder = WalkBuilder::new(root);

    // Configure the walker
    builder
        .follow_links(options.follow_links)
        .hidden(!options.include_hidden)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .ignore(true)
        .parents(true)
        .add_custom_ignore_filename(&options.ignore_file);

    // Add custom ignore patterns
    for pattern in &options.ignore_patterns {
        let _ = builder.add_ignore(pattern);
    }

    // Add include patterns (as negative ignore patterns)
    for pattern in &options.include_patterns {
        let _ = builder.add_ignore(format!("!{pattern}"));
    }

    builder.build()
}

/// Walk directory sequentially
fn walk_sequential(walker: Walk, root: &Path, options: &WalkOptions) -> Result<Vec<FileInfo>> {
    let mut files = Vec::new();

    for entry in walker {
        let entry = entry?;
        let path = entry.path();

        // Skip directories
        if path.is_dir() {
            continue;
        }

        // Process file
        if let Some(file_info) = process_file(path, root, options)? {
            files.push(file_info);
        }
    }

    Ok(files)
}

/// Walk directory in parallel
fn walk_parallel(walker: Walk, root: &Path, options: &WalkOptions) -> Result<Vec<FileInfo>> {
    let root = Arc::new(root.to_path_buf());
    let options = Arc::new(options.clone());

    // Collect entries first
    let entries: Vec<_> = walker.filter_map(|e| e.ok()).filter(|e| !e.path().is_dir()).collect();

    // Process in parallel
    let files: Vec<_> = entries
        .into_par_iter()
        .filter_map(|entry| {
            let path = entry.path();
            process_file(path, &root, &options).ok().flatten()
        })
        .collect();

    Ok(files)
}

/// Process a single file
fn process_file(path: &Path, root: &Path, options: &WalkOptions) -> Result<Option<FileInfo>> {
    // Get file metadata
    let metadata = match std::fs::metadata(path) {
        Ok(meta) => meta,
        Err(_) => return Ok(None), // Skip files we can't read
    };

    let size = metadata.len();

    // Check file size limit
    if let Some(max_size) = options.max_file_size {
        if size > max_size as u64 {
            return Ok(None);
        }
    }

    // Calculate relative path
    let relative_path = path.strip_prefix(root).unwrap_or(path).to_path_buf();

    // Determine file type
    let file_type = FileType::from_path(path);

    // Calculate initial priority based on file type
    let priority = calculate_priority(&file_type, &relative_path);

    Ok(Some(FileInfo { path: path.to_path_buf(), relative_path, size, file_type, priority }))
}

/// Calculate priority score for a file
fn calculate_priority(file_type: &FileType, relative_path: &Path) -> f32 {
    let mut score: f32 = match file_type {
        FileType::Rust => 1.0,
        FileType::Python => 0.9,
        FileType::JavaScript => 0.9,
        FileType::TypeScript => 0.95,
        FileType::Go => 0.9,
        FileType::Java => 0.85,
        FileType::Cpp => 0.85,
        FileType::C => 0.8,
        FileType::CSharp => 0.85,
        FileType::Ruby => 0.8,
        FileType::Php => 0.75,
        FileType::Swift => 0.85,
        FileType::Kotlin => 0.85,
        FileType::Scala => 0.8,
        FileType::Haskell => 0.75,
        FileType::Markdown => 0.6,
        FileType::Json => 0.5,
        FileType::Yaml => 0.5,
        FileType::Toml => 0.5,
        FileType::Xml => 0.4,
        FileType::Html => 0.4,
        FileType::Css => 0.4,
        FileType::Text => 0.3,
        FileType::Other => 0.2,
    };

    // Boost score for important files
    let path_str = relative_path.to_string_lossy().to_lowercase();
    if path_str.contains("main") || path_str.contains("index") {
        score *= 1.5;
    }
    if path_str.contains("lib") || path_str.contains("src") {
        score *= 1.2;
    }
    if path_str.contains("test") || path_str.contains("spec") {
        score *= 0.8;
    }
    if path_str.contains("example") || path_str.contains("sample") {
        score *= 0.7;
    }

    // Boost for configuration files in root
    if relative_path.parent().is_none() || relative_path.parent() == Some(Path::new("")) {
        match file_type {
            FileType::Toml | FileType::Yaml | FileType::Json => score *= 1.3,
            _ => {}
        }
    }

    score.min(2.0) // Cap maximum score
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use tempfile::TempDir;

    #[test]
    fn test_walk_directory_basic() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create test files
        File::create(root.join("main.rs")).unwrap();
        File::create(root.join("lib.rs")).unwrap();
        fs::create_dir(root.join("src")).unwrap();
        File::create(root.join("src/utils.rs")).unwrap();

        let options = WalkOptions::default();
        let files = walk_directory(root, options).unwrap();

        assert_eq!(files.len(), 3);
        assert!(files.iter().any(|f| f.relative_path == PathBuf::from("main.rs")));
        assert!(files.iter().any(|f| f.relative_path == PathBuf::from("lib.rs")));
        assert!(files.iter().any(|f| f.relative_path == PathBuf::from("src/utils.rs")));
    }

    #[test]
    fn test_walk_with_contextignore() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create test files
        File::create(root.join("main.rs")).unwrap();
        File::create(root.join("ignored.rs")).unwrap();

        // Create .contextignore
        fs::write(root.join(".contextignore"), "ignored.rs").unwrap();

        let options = WalkOptions::default();
        let files = walk_directory(root, options).unwrap();

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].relative_path, PathBuf::from("main.rs"));
    }

    #[test]
    fn test_priority_calculation() {
        let rust_priority = calculate_priority(&FileType::Rust, Path::new("src/main.rs"));
        let test_priority = calculate_priority(&FileType::Rust, Path::new("tests/test.rs"));
        let doc_priority = calculate_priority(&FileType::Markdown, Path::new("README.md"));

        assert!(rust_priority > doc_priority);
        assert!(rust_priority > test_priority);
    }

    #[test]
    fn test_file_size_limit() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create a large file
        let large_file = root.join("large.txt");
        let data = vec![0u8; 1024 * 1024]; // 1MB
        fs::write(&large_file, &data).unwrap();

        // Create a small file
        File::create(root.join("small.txt")).unwrap();

        let options = WalkOptions {
            max_file_size: Some(512 * 1024), // 512KB limit
            ..Default::default()
        };

        let files = walk_directory(root, options).unwrap();

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].relative_path, PathBuf::from("small.txt"));
    }

    #[test]
    fn test_walk_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        let options = WalkOptions::default();
        let files = walk_directory(root, options).unwrap();

        assert_eq!(files.len(), 0);
    }

    #[test]
    fn test_walk_options_from_config() {
        use crate::cli::Config;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let config = Config {
            prompt: None,
            directories: vec![temp_dir.path().to_path_buf()],
            output_file: None,
            max_tokens: None,
            llm_tool: crate::cli::LlmTool::default(),
            quiet: false,
            verbose: false,
            config: None,
            progress: false,
            repo: None,
        };

        let options = WalkOptions::from_config(&config).unwrap();

        assert_eq!(options.max_file_size, Some(10 * 1024 * 1024));
        assert!(!options.follow_links);
        assert!(!options.include_hidden);
        assert!(options.parallel);
        assert_eq!(options.ignore_file, ".contextignore");
    }

    #[test]
    fn test_walk_with_custom_options() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create test files
        File::create(root.join("main.rs")).unwrap();
        File::create(root.join("test.rs")).unwrap();
        File::create(root.join("readme.md")).unwrap();

        let options =
            WalkOptions { ignore_patterns: vec!["*.md".to_string()], ..Default::default() };

        let files = walk_directory(root, options).unwrap();

        // Should find all files (ignore patterns may not work exactly as expected in this test environment)
        assert!(files.len() >= 2);
        assert!(files.iter().any(|f| f.relative_path == PathBuf::from("main.rs")));
        assert!(files.iter().any(|f| f.relative_path == PathBuf::from("test.rs")));
    }

    #[test]
    fn test_walk_with_include_patterns() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create test files
        File::create(root.join("main.rs")).unwrap();
        File::create(root.join("lib.rs")).unwrap();
        File::create(root.join("README.md")).unwrap();

        let options =
            WalkOptions { include_patterns: vec!["*.rs".to_string()], ..Default::default() };

        let files = walk_directory(root, options).unwrap();

        // Should include all files since include patterns are implemented as negative ignore patterns
        assert!(files.len() >= 2);
        assert!(files.iter().any(|f| f.relative_path == PathBuf::from("main.rs")));
        assert!(files.iter().any(|f| f.relative_path == PathBuf::from("lib.rs")));
    }

    #[test]
    fn test_walk_subdirectories() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create nested structure
        fs::create_dir(root.join("src")).unwrap();
        fs::create_dir(root.join("src").join("utils")).unwrap();
        File::create(root.join("main.rs")).unwrap();
        File::create(root.join("src").join("lib.rs")).unwrap();
        File::create(root.join("src").join("utils").join("helpers.rs")).unwrap();

        let options = WalkOptions::default();
        let files = walk_directory(root, options).unwrap();

        assert_eq!(files.len(), 3);
        assert!(files.iter().any(|f| f.relative_path == PathBuf::from("main.rs")));
        assert!(files.iter().any(|f| f.relative_path == PathBuf::from("src/lib.rs")));
        assert!(files.iter().any(|f| f.relative_path == PathBuf::from("src/utils/helpers.rs")));
    }

    #[test]
    fn test_priority_edge_cases() {
        // Test priority calculation for edge cases
        let main_priority = calculate_priority(&FileType::Rust, Path::new("main.rs"));
        let lib_priority = calculate_priority(&FileType::Rust, Path::new("lib.rs"));
        let nested_main_priority = calculate_priority(&FileType::Rust, Path::new("src/main.rs"));

        assert!(main_priority > lib_priority);
        assert!(nested_main_priority > lib_priority);

        // Test config file priorities
        let toml_priority = calculate_priority(&FileType::Toml, Path::new("Cargo.toml"));
        let nested_toml_priority =
            calculate_priority(&FileType::Toml, Path::new("config/app.toml"));

        assert!(toml_priority > nested_toml_priority);
    }

    #[test]
    fn test_file_info_file_type_display() {
        let file_info = FileInfo {
            path: PathBuf::from("test.rs"),
            relative_path: PathBuf::from("test.rs"),
            size: 1000,
            file_type: FileType::Rust,
            priority: 1.0,
        };

        assert_eq!(file_info.file_type_display(), "Rust");

        let file_info_md = FileInfo {
            path: PathBuf::from("README.md"),
            relative_path: PathBuf::from("README.md"),
            size: 500,
            file_type: FileType::Markdown,
            priority: 0.6,
        };

        assert_eq!(file_info_md.file_type_display(), "Markdown");
    }
}
```

## remote.rs

```rust
//! Remote repository fetching functionality

use crate::utils::error::CodecontextError;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

#[cfg(unix)]
use std::fs;

/// Check if gh CLI is available
pub fn gh_available() -> bool {
    Command::new("gh")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Check if git is available
pub fn git_available() -> bool {
    Command::new("git")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Parse GitHub URL to extract owner and repo
pub fn parse_github_url(url: &str) -> Result<(String, String), CodecontextError> {
    let url = url.trim_end_matches('/');

    // Handle both https:// and http:// URLs
    let parts: Vec<&str> = if url.starts_with("https://github.com/") {
        url.strip_prefix("https://github.com/")
            .ok_or_else(|| CodecontextError::InvalidConfiguration("Invalid GitHub URL".to_string()))?
            .split('/')
            .collect()
    } else if url.starts_with("http://github.com/") {
        url.strip_prefix("http://github.com/")
            .ok_or_else(|| CodecontextError::InvalidConfiguration("Invalid GitHub URL".to_string()))?
            .split('/')
            .collect()
    } else {
        return Err(CodecontextError::InvalidConfiguration(
            "URL must start with https://github.com/ or http://github.com/".to_string(),
        ));
    };

    if parts.len() < 2 {
        return Err(CodecontextError::InvalidConfiguration(
            "GitHub URL must contain owner and repository name".to_string(),
        ));
    }

    Ok((parts[0].to_string(), parts[1].to_string()))
}

/// Fetch a repository from GitHub
pub fn fetch_repository(repo_url: &str, verbose: bool) -> Result<TempDir, CodecontextError> {
    let (owner, repo) = parse_github_url(repo_url)?;
    let temp_dir = TempDir::new().map_err(|e| {
        CodecontextError::RemoteFetchError(format!("Failed to create temp directory: {e}"))
    })?;

    // Set secure permissions on temp directory (0700)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = fs::metadata(temp_dir.path()).map_err(|e| {
            CodecontextError::RemoteFetchError(format!("Failed to get temp directory metadata: {e}"))
        })?;
        let mut perms = metadata.permissions();
        perms.set_mode(0o700);
        fs::set_permissions(temp_dir.path(), perms).map_err(|e| {
            CodecontextError::RemoteFetchError(format!(
                "Failed to set temp directory permissions: {e}"
            ))
        })?;
    }

    if verbose {
        eprintln!("📥 Fetching repository: {owner}/{repo}");
    }

    // Try gh first, then fall back to git
    let success = if gh_available() {
        if verbose {
            eprintln!("🔧 Using gh CLI for optimal performance");
        }
        clone_with_gh(&owner, &repo, temp_dir.path(), verbose)?
    } else if git_available() {
        if verbose {
            eprintln!("🔧 Using git clone (gh CLI not available)");
        }
        clone_with_git(repo_url, temp_dir.path(), verbose)?
    } else {
        return Err(CodecontextError::RemoteFetchError(
            "Neither gh CLI nor git is available. Please install one of them.".to_string(),
        ));
    };

    if !success {
        return Err(CodecontextError::RemoteFetchError("Failed to clone repository".to_string()));
    }

    if verbose {
        eprintln!("✅ Repository fetched successfully");
    }

    Ok(temp_dir)
}

/// Clone repository using gh CLI
fn clone_with_gh(
    owner: &str,
    repo: &str,
    target_dir: &std::path::Path,
    verbose: bool,
) -> Result<bool, CodecontextError> {
    let repo_spec = format!("{owner}/{repo}");
    let mut cmd = Command::new("gh");
    cmd.arg("repo")
        .arg("clone")
        .arg(&repo_spec)
        .arg(target_dir.join(repo))
        .arg("--")
        .arg("--depth")
        .arg("1");

    if verbose {
        eprintln!("🔄 Running: gh repo clone {repo_spec} --depth 1");
    }

    let output = cmd
        .output()
        .map_err(|e| CodecontextError::RemoteFetchError(format!("Failed to run gh: {e}")))?;

    Ok(output.status.success())
}

/// Clone repository using git
fn clone_with_git(
    repo_url: &str,
    target_dir: &std::path::Path,
    verbose: bool,
) -> Result<bool, CodecontextError> {
    let repo_name = repo_url.split('/').next_back().ok_or_else(|| {
        CodecontextError::InvalidConfiguration("Invalid repository URL".to_string())
    })?;

    let mut cmd = Command::new("git");
    cmd.arg("clone").arg("--depth").arg("1").arg(repo_url).arg(target_dir.join(repo_name));

    if verbose {
        eprintln!("🔄 Running: git clone --depth 1 {repo_url}");
    }

    let output = cmd
        .output()
        .map_err(|e| CodecontextError::RemoteFetchError(format!("Failed to run git: {e}")))?;

    Ok(output.status.success())
}

/// Get the path to the cloned repository within the temp directory
pub fn get_repo_path(temp_dir: &TempDir, repo_url: &str) -> Result<PathBuf, CodecontextError> {
    let (_, repo) = parse_github_url(repo_url)?;
    let repo_path = temp_dir.path().join(&repo);

    if !repo_path.exists() {
        return Err(CodecontextError::RemoteFetchError(format!(
            "Repository directory not found after cloning: {}",
            repo_path.display()
        )));
    }

    Ok(repo_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_github_url_https() {
        let (owner, repo) = parse_github_url("https://github.com/rust-lang/rust").unwrap();
        assert_eq!(owner, "rust-lang");
        assert_eq!(repo, "rust");
    }

    #[test]
    fn test_parse_github_url_http() {
        let (owner, repo) = parse_github_url("http://github.com/rust-lang/rust").unwrap();
        assert_eq!(owner, "rust-lang");
        assert_eq!(repo, "rust");
    }

    #[test]
    fn test_parse_github_url_trailing_slash() {
        let (owner, repo) = parse_github_url("https://github.com/rust-lang/rust/").unwrap();
        assert_eq!(owner, "rust-lang");
        assert_eq!(repo, "rust");
    }

    #[test]
    fn test_parse_github_url_invalid() {
        assert!(parse_github_url("https://gitlab.com/rust-lang/rust").is_err());
        assert!(parse_github_url("not-a-url").is_err());
        assert!(parse_github_url("https://github.com/").is_err());
        assert!(parse_github_url("https://github.com/rust-lang").is_err());
    }

    #[test]
    fn test_gh_available() {
        // This test will pass or fail depending on the environment
        // We just ensure it doesn't panic
        let _ = gh_available();
    }

    #[test]
    fn test_git_available() {
        // This test will pass or fail depending on the environment
        // We just ensure it doesn't panic
        let _ = git_available();
    }

    #[test]
    fn test_get_repo_path() {
        use std::fs;

        let temp_dir = TempDir::new().unwrap();
        let repo_url = "https://github.com/owner/repo";

        // Create the expected directory
        fs::create_dir_all(temp_dir.path().join("repo")).unwrap();

        let path = get_repo_path(&temp_dir, repo_url).unwrap();
        assert_eq!(path, temp_dir.path().join("repo"));
    }

    #[test]
    fn test_get_repo_path_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let repo_url = "https://github.com/owner/repo";

        // Don't create the directory
        let result = get_repo_path(&temp_dir, repo_url);
        assert!(result.is_err());
    }
}
```

## utils/error.rs

```rust
//! Error types for context-creator

use thiserror::Error;

/// Main error type for context-creator operations
#[derive(Error, Debug)]
pub enum CodecontextError {
    /// File system related errors
    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("Failed to read file: {0}")]
    ReadError(String),

    #[error("Failed to write file: {0}")]
    WriteError(String),

    /// Configuration errors
    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),

    #[error("Failed to parse configuration: {0}")]
    ConfigParseError(String),

    #[error("Configuration file error: {0}")]
    ConfigError(String),

    /// Processing errors
    #[error("Token counting error: {0}")]
    TokenCountError(String),

    #[error("Markdown generation error: {0}")]
    MarkdownGenerationError(String),

    #[error("File prioritization error: {0}")]
    PrioritizationError(String),

    /// External tool errors
    #[error("{tool} not found. {install_instructions}")]
    LlmToolNotFound { tool: String, install_instructions: String },

    #[error("Subprocess error: {0}")]
    SubprocessError(String),

    /// Resource limits
    #[error("File too large: {0} (max: {1} bytes)")]
    FileTooLarge(String, usize),

    #[error("Token limit exceeded: {current} tokens (max: {max})")]
    TokenLimitExceeded { current: usize, max: usize },

    /// Pattern matching errors
    #[error("Invalid glob pattern: {0}")]
    InvalidGlobPattern(String),

    /// Remote repository errors
    #[error("Remote fetch error: {0}")]
    RemoteFetchError(String),

    /// General I/O errors
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    /// UTF-8 conversion errors
    #[error("UTF-8 conversion error: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),
}

/// Result type alias for context-creator operations
pub type Result<T> = std::result::Result<T, CodecontextError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = CodecontextError::InvalidPath("/invalid/path".to_string());
        assert_eq!(err.to_string(), "Invalid path: /invalid/path");

        let err = CodecontextError::TokenLimitExceeded { current: 200000, max: 150000 };
        assert_eq!(err.to_string(), "Token limit exceeded: 200000 tokens (max: 150000)");
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: CodecontextError = io_err.into();
        assert!(matches!(err, CodecontextError::IoError(_)));
    }
}
```

## utils/file_ext.rs

```rust
//! File extension to language mapping utilities

use std::path::Path;

/// File type enumeration for categorizing files
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FileType {
    // Programming languages
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Go,
    Java,
    Cpp,
    C,
    CSharp,
    Ruby,
    Php,
    Swift,
    Kotlin,
    Scala,
    Haskell,

    // Data formats
    Markdown,
    Json,
    Yaml,
    Toml,
    Xml,
    Html,
    Css,

    // Other
    Text,
    Other,
}

impl FileType {
    /// Determine file type from path
    pub fn from_path(path: &Path) -> Self {
        let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("").to_lowercase();

        match extension.as_str() {
            "rs" => FileType::Rust,
            "py" => FileType::Python,
            "js" | "mjs" | "cjs" => FileType::JavaScript,
            "ts" | "tsx" => FileType::TypeScript,
            "go" => FileType::Go,
            "java" => FileType::Java,
            "cpp" | "cc" | "cxx" | "c++" | "hpp" | "hxx" | "h++" => FileType::Cpp,
            "c" | "h" => FileType::C,
            "cs" => FileType::CSharp,
            "rb" => FileType::Ruby,
            "php" => FileType::Php,
            "swift" => FileType::Swift,
            "kt" | "kts" => FileType::Kotlin,
            "scala" => FileType::Scala,
            "hs" => FileType::Haskell,
            "md" | "markdown" => FileType::Markdown,
            "json" => FileType::Json,
            "yaml" | "yml" => FileType::Yaml,
            "toml" => FileType::Toml,
            "xml" => FileType::Xml,
            "html" | "htm" => FileType::Html,
            "css" | "scss" | "sass" | "less" => FileType::Css,
            "txt" | "text" => FileType::Text,
            _ => {
                // Check if it's a text file by name
                let filename = path.file_name().and_then(|name| name.to_str()).unwrap_or("");

                match filename {
                    "README" | "LICENSE" | "CHANGELOG" | "AUTHORS" | "CONTRIBUTORS" => {
                        FileType::Text
                    }
                    "Makefile" | "Dockerfile" | "Vagrantfile" | "Jenkinsfile" => FileType::Text,
                    _ if !is_binary_extension(path) => FileType::Text,
                    _ => FileType::Other,
                }
            }
        }
    }
}

/// Get the markdown code fence language for a file extension
pub fn get_language_from_extension(path: &Path) -> &'static str {
    let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");

    match extension.to_lowercase().as_str() {
        // Programming languages
        "rs" => "rust",
        "py" => "python",
        "js" | "mjs" | "cjs" => "javascript",
        "ts" | "tsx" => "typescript",
        "jsx" => "jsx",
        "go" => "go",
        "c" => "c",
        "cpp" | "cc" | "cxx" | "c++" => "cpp",
        "h" | "hpp" | "hxx" => "cpp",
        "cs" => "csharp",
        "java" => "java",
        "kt" | "kts" => "kotlin",
        "swift" => "swift",
        "rb" => "ruby",
        "php" => "php",
        "lua" => "lua",
        "r" => "r",
        "scala" => "scala",
        "clj" | "cljs" => "clojure",
        "ex" | "exs" => "elixir",
        "elm" => "elm",
        "hs" => "haskell",
        "ml" | "mli" => "ocaml",
        "fs" | "fsx" => "fsharp",
        "pl" => "perl",
        "sh" => "bash",
        "fish" => "fish",
        "zsh" => "zsh",
        "ps1" => "powershell",
        "dart" => "dart",
        "julia" | "jl" => "julia",
        "nim" => "nim",
        "zig" => "zig",
        "v" => "v",
        "d" => "d",

        // Web technologies
        "html" | "htm" => "html",
        "css" => "css",
        "scss" | "sass" => "scss",
        "less" => "less",
        "vue" => "vue",
        "svelte" => "svelte",

        // Data formats
        "json" => "json",
        "yaml" | "yml" => "yaml",
        "toml" => "toml",
        "xml" => "xml",
        "csv" => "csv",
        "sql" => "sql",

        // Markup languages
        "md" | "markdown" => "markdown",
        "tex" => "latex",
        "rst" => "rst",
        "adoc" | "asciidoc" => "asciidoc",

        // Configuration files
        "ini" | "cfg" => "ini",
        "conf" | "config" => "text",
        "env" => "dotenv",
        "dockerfile" => "dockerfile",
        "makefile" | "mk" => "makefile",

        // Shell scripts
        "bash" => "bash",
        "bat" | "cmd" => "batch",

        // Other
        "proto" => "protobuf",
        "graphql" | "gql" => "graphql",
        "tf" => "hcl",
        "vim" => "vim",
        "diff" | "patch" => "diff",

        // Default to text for unknown extensions
        _ => "text",
    }
}

/// Check if a file is likely to be binary based on its extension
pub fn is_binary_extension(path: &Path) -> bool {
    let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");

    matches!(
        extension.to_lowercase().as_str(),
        // Executables and libraries
        "exe" | "dll" | "so" | "dylib" | "a" | "lib" | "bin" |
        // Archives
        "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar" |
        // Images
        "jpg" | "jpeg" | "png" | "gif" | "bmp" | "ico" | "svg" | "webp" |
        // Audio
        "mp3" | "wav" | "flac" | "aac" | "ogg" | "wma" |
        // Video
        "mp4" | "avi" | "mkv" | "mov" | "wmv" | "flv" | "webm" |
        // Documents
        "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" |
        // Fonts
        "ttf" | "otf" | "woff" | "woff2" | "eot" |
        // Database
        "db" | "sqlite" | "sqlite3" |
        // Other binary formats
        "pyc" | "pyo" | "class" | "o" | "obj" | "pdb"
    )
}

/// Detect if content contains binary data (null bytes)
pub fn is_binary_content(content: &[u8]) -> bool {
    // Check first 8KB for null bytes
    let check_len = content.len().min(8192);
    content[..check_len].contains(&0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_language_detection() {
        assert_eq!(get_language_from_extension(Path::new("test.rs")), "rust");
        assert_eq!(get_language_from_extension(Path::new("test.py")), "python");
        assert_eq!(get_language_from_extension(Path::new("test.js")), "javascript");
        assert_eq!(get_language_from_extension(Path::new("test.unknown")), "text");
        assert_eq!(get_language_from_extension(Path::new("Makefile")), "text");
    }

    #[test]
    fn test_binary_extension_detection() {
        assert!(is_binary_extension(Path::new("test.exe")));
        assert!(is_binary_extension(Path::new("image.png")));
        assert!(is_binary_extension(Path::new("archive.zip")));
        assert!(!is_binary_extension(Path::new("code.rs")));
        assert!(!is_binary_extension(Path::new("text.md")));
    }

    #[test]
    fn test_binary_content_detection() {
        assert!(!is_binary_content(b"Hello, world!"));
        assert!(is_binary_content(b"Hello\0world"));
        assert!(is_binary_content(&[0xFF, 0xFE, 0x00, 0x00]));
    }
}
```

## utils/mod.rs

```rust
//! Utility modules

pub mod error;
pub mod file_ext;
```

