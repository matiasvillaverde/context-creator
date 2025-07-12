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
    /// Directories to process (positional)
    #[arg(value_name = "DIRECTORIES", num_args = 0..)]
    pub directories_positional: Vec<PathBuf>,

    /// The prompt to send to the LLM. If omitted, only generates the Markdown context
    #[arg(value_name = "PROMPT", last = true, conflicts_with = "prompt_flag")]
    pub prompt: Option<String>,

    /// The prompt to send to the LLM (use when prompt contains spaces)
    #[arg(short = 'p', long = "prompt", conflicts_with = "prompt")]
    pub prompt_flag: Option<String>,

    /// The paths to the directories to process
    #[arg(short = 'd', long, num_args = 1.., conflicts_with = "repo")]
    pub directories: Vec<PathBuf>,

    /// GitHub repository URL to analyze (e.g., <https://github.com/owner/repo>)
    #[arg(long, conflicts_with = "directories")]
    pub repo: Option<String>,

    /// Read prompt from stdin
    #[arg(long = "stdin")]
    pub read_stdin: bool,

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

    /// Copy output to system clipboard instead of stdout
    #[arg(short = 'C', long)]
    pub copy: bool,
}

impl Config {
    /// Validate the configuration
    pub fn validate(&self) -> Result<(), crate::utils::error::CodeDigestError> {
        use crate::utils::error::CodeDigestError;

        // Validate repo URL if provided
        if let Some(repo_url) = &self.repo {
            if !repo_url.starts_with("https://github.com/")
                && !repo_url.starts_with("http://github.com/")
            {
                return Err(CodeDigestError::InvalidConfiguration(
                    "Repository URL must be a GitHub URL (https://github.com/owner/repo)"
                        .to_string(),
                ));
            }
        } else {
            // Only validate directories if repo is not provided
            for directory in &self.directories {
                if !directory.exists() {
                    return Err(CodeDigestError::InvalidPath(format!(
                        "Directory does not exist: {}",
                        directory.display()
                    )));
                }

                if !directory.is_dir() {
                    return Err(CodeDigestError::InvalidPath(format!(
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
                    return Err(CodeDigestError::InvalidPath(format!(
                        "Output directory does not exist: {}",
                        parent.display()
                    )));
                }
            }
        }

        // Validate mutually exclusive options
        if self.output_file.is_some() && self.get_prompt().is_some() {
            return Err(CodeDigestError::InvalidConfiguration(
                "Cannot specify both --output and a prompt".to_string(),
            ));
        }

        // Validate copy and output mutual exclusivity
        if self.copy && self.output_file.is_some() {
            return Err(CodeDigestError::InvalidConfiguration(
                "Cannot specify both --copy and --output".to_string(),
            ));
        }

        Ok(())
    }

    /// Load configuration from file if specified
    pub fn load_from_file(&mut self) -> Result<(), crate::utils::error::CodeDigestError> {
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

    /// Get the actual prompt from various sources
    pub fn get_prompt(&self) -> Option<String> {
        // First check explicit prompt flag
        if let Some(prompt) = &self.prompt_flag {
            return Some(prompt.clone());
        }

        // Then check the prompt positional argument
        if let Some(prompt) = &self.prompt {
            // Don't return empty strings as prompts
            if !prompt.trim().is_empty() {
                return Some(prompt.clone());
            }
        }

        // For backward compatibility: check if the last directory argument is actually a prompt
        // This should only apply when using old syntax: -d dir1 "prompt text"
        // Not when using new multi-directory syntax: -d dir1 dir2
        if self.directories.len() > 1 && self.prompt.is_none() && self.prompt_flag.is_none() {
            let last = self.directories.last().unwrap();
            // Only treat as prompt if it doesn't exist AND doesn't look like a path
            if !last.exists() {
                let path_str = last.to_string_lossy();
                // Check if it looks like a path (contains separators or common path patterns)
                if !path_str.contains('/')
                    && !path_str.contains('\\')
                    && !path_str.starts_with('.')
                    && !path_str.contains("project")
                    && !path_str.contains("_dir")
                    && !path_str.contains("tmp")
                {
                    return Some(path_str.to_string());
                }
            }
        }

        None
    }

    /// Get all directories from both sources
    pub fn get_directories(&self) -> Vec<PathBuf> {
        let mut dirs = self.directories.clone();

        // For backward compatibility: remove last directory if it's being used as a prompt
        if dirs.len() > 1 && self.prompt.is_none() && self.prompt_flag.is_none() {
            let last = dirs.last().unwrap();
            if !last.exists() {
                let path_str = last.to_string_lossy();
                // Check if it looks like a path (contains separators or common path patterns)
                if !path_str.contains('/')
                    && !path_str.contains('\\')
                    && !path_str.starts_with('.')
                    && !path_str.contains("project")
                    && !path_str.contains("_dir")
                    && !path_str.contains("tmp")
                {
                    dirs.pop();
                }
            }
        }

        // Add positional directories
        dirs.extend(self.directories_positional.clone());

        // If no directories specified, use current directory
        if dirs.is_empty() {
            vec![PathBuf::from(".")]
        } else {
            dirs
        }
    }

    /// Check if we should read from stdin
    pub fn should_read_stdin(&self) -> bool {
        use std::io::IsTerminal;

        // Explicitly requested stdin
        if self.read_stdin {
            return true;
        }

        // If stdin is not a terminal (i.e., it's piped) and no prompt is provided
        if !std::io::stdin().is_terminal() && self.get_prompt().is_none() {
            return true;
        }

        false
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
            prompt_flag: None,
            directories: vec![temp_dir.path().to_path_buf()],
            directories_positional: vec![],
            repo: None,
            read_stdin: false,
            output_file: None,
            max_tokens: None,
            llm_tool: LlmTool::default(),
            quiet: false,
            verbose: false,
            config: None,
            progress: false,
            copy: false,
        };

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation_invalid_directory() {
        let config = Config {
            prompt: None,
            prompt_flag: None,
            directories: vec![PathBuf::from("/nonexistent/directory")],
            directories_positional: vec![],
            repo: None,
            read_stdin: false,
            output_file: None,
            max_tokens: None,
            llm_tool: LlmTool::default(),
            quiet: false,
            verbose: false,
            config: None,
            progress: false,
            copy: false,
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
            prompt_flag: None,
            directories: vec![file_path],
            directories_positional: vec![],
            repo: None,
            read_stdin: false,
            output_file: None,
            max_tokens: None,
            llm_tool: LlmTool::default(),
            quiet: false,
            verbose: false,
            config: None,
            progress: false,
            copy: false,
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_invalid_output_directory() {
        let temp_dir = TempDir::new().unwrap();
        let config = Config {
            prompt: None,
            prompt_flag: None,
            directories: vec![temp_dir.path().to_path_buf()],
            directories_positional: vec![],
            repo: None,
            read_stdin: false,
            output_file: Some(PathBuf::from("/nonexistent/directory/output.md")),
            max_tokens: None,
            llm_tool: LlmTool::default(),
            quiet: false,
            verbose: false,
            config: None,
            progress: false,
            copy: false,
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_mutually_exclusive_options() {
        let temp_dir = TempDir::new().unwrap();
        let config = Config {
            prompt: Some("test prompt".to_string()),
            prompt_flag: None,
            directories: vec![temp_dir.path().to_path_buf()],
            directories_positional: vec![],
            repo: None,
            read_stdin: false,
            output_file: Some(temp_dir.path().join("output.md")),
            max_tokens: None,
            llm_tool: LlmTool::default(),
            quiet: false,
            verbose: false,
            config: None,
            progress: false,
            copy: false,
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
            prompt_flag: None,
            directories: vec![temp_dir.path().to_path_buf()],
            directories_positional: vec![],
            repo: None,
            read_stdin: false,
            output_file: Some(PathBuf::from("output.md")),
            max_tokens: None,
            llm_tool: LlmTool::default(),
            quiet: false,
            verbose: false,
            config: None,
            progress: false,
            copy: false,
        };

        // Should not error for files in current directory
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_load_from_file_no_config() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = Config {
            prompt: None,
            prompt_flag: None,
            directories: vec![temp_dir.path().to_path_buf()],
            directories_positional: vec![],
            repo: None,
            read_stdin: false,
            output_file: None,
            max_tokens: None,
            llm_tool: LlmTool::default(),
            quiet: false,
            verbose: false,
            config: None,
            progress: false,
            copy: false,
        };

        // Should not error when no config file is found
        assert!(config.load_from_file().is_ok());
    }

    #[test]
    fn test_parse_multiple_directories() {
        use clap::Parser;

        // Test single directory (backward compatibility)
        let args = vec!["code-digest", "-d", "/path/one"];
        let config = Config::parse_from(args);
        assert_eq!(config.directories.len(), 1);
        assert_eq!(config.directories[0], PathBuf::from("/path/one"));
    }

    #[test]
    fn test_parse_multiple_directories_new_api() {
        use clap::Parser;

        // Test single directory (backward compatibility)
        let args = vec!["code-digest", "-d", "/path/one"];
        let config = Config::parse_from(args);
        assert_eq!(config.directories.len(), 1);
        assert_eq!(config.directories[0], PathBuf::from("/path/one"));

        // Test multiple directories
        let args = vec!["code-digest", "-d", "/path/one", "/path/two", "/path/three"];
        let config = Config::parse_from(args);
        assert_eq!(config.directories.len(), 3);
        assert_eq!(config.directories[0], PathBuf::from("/path/one"));
        assert_eq!(config.directories[1], PathBuf::from("/path/two"));
        assert_eq!(config.directories[2], PathBuf::from("/path/three"));

        // Test with prompt after directories using -- separator
        let args = vec![
            "code-digest",
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
            prompt_flag: None,
            directories: vec![dir1.clone(), dir2.clone()],
            directories_positional: vec![],
            repo: None,
            read_stdin: false,
            output_file: None,
            max_tokens: None,
            llm_tool: LlmTool::default(),
            quiet: false,
            verbose: false,
            config: None,
            progress: false,
            copy: false,
        };
        assert!(config.validate().is_ok());

        // One directory doesn't exist - should fail
        let config = Config {
            prompt: None,
            prompt_flag: None,
            directories: vec![dir1, PathBuf::from("/nonexistent/dir")],
            directories_positional: vec![],
            repo: None,
            read_stdin: false,
            output_file: None,
            max_tokens: None,
            llm_tool: LlmTool::default(),
            quiet: false,
            verbose: false,
            config: None,
            progress: false,
            copy: false,
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
            prompt_flag: None,
            directories: vec![dir1, file1],
            directories_positional: vec![],
            repo: None,
            read_stdin: false,
            output_file: None,
            max_tokens: None,
            llm_tool: LlmTool::default(),
            quiet: false,
            verbose: false,
            config: None,
            progress: false,
            copy: false,
        };
        assert!(config.validate().is_err());
    }
}
