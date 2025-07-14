//! Command-line interface configuration and parsing

use clap::{Parser, ValueEnum};
use std::path::PathBuf;

/// Help message explaining custom priority rules and usage
const AFTER_HELP_MSG: &str = "\
CUSTOM PRIORITY RULES:
  Custom priority rules are processed in a 'first-match-wins' basis. Rules are 
  evaluated in the order they are defined in your .code-digest.toml configuration 
  file. The first rule that matches a given file will be used, and all subsequent 
  rules will be ignored for that file.

  Example configuration:
    [[priorities]]
    pattern = \"src/**/*.rs\"
    weight = 10.0
    
    [[priorities]]  
    pattern = \"tests/*\"
    weight = -2.0

USAGE EXAMPLES:
  # Process current directory with a prompt
  code-digest --prompt \"Analyze this code\"
  
  # Process specific directories (positional arguments)
  code-digest src/ tests/ docs/
  
  # Process specific directories (explicit include flags)
  code-digest --include src/ --include tests/ --include docs/
  
  # Process files matching glob patterns (QUOTE patterns to prevent shell expansion)
  code-digest --include \"**/*.py\" --include \"src/**/*.{rs,toml}\"
  
  # Process specific file types across all directories
  code-digest --include \"**/*repository*.py\" --include \"**/test[0-9].py\"
  
  # Process a GitHub repository
  code-digest --repo https://github.com/owner/repo
  
  # Read prompt from stdin
  echo \"Review this code\" | code-digest --stdin .
";

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
#[command(author, version, about, long_about = None, after_help = AFTER_HELP_MSG)]
#[command(group(
    clap::ArgGroup::new("input_source")
        .required(true)
        .args(&["prompt", "paths", "repo", "read_stdin", "include"]),
))]
pub struct Config {
    /// The prompt to send to the LLM for processing
    #[arg(short = 'p', long = "prompt", help = "Process a text prompt directly")]
    pub prompt: Option<String>,

    /// One or more directory paths to process
    /// IMPORTANT: Use `get_directories()` to access the correct input paths.
    #[arg(value_name = "PATHS", help = "Process directories", conflicts_with = "include")]
    pub paths: Option<Vec<PathBuf>>,

    /// Include files and directories matching glob patterns
    /// IMPORTANT: Use `get_directories()` to access the correct input paths.
    #[arg(
        long,
        help = "Include files and directories matching the given glob pattern.\nPatterns use gitignore-style syntax. To prevent shell expansion,\nquote patterns: --include \"*.py\" --include \"src/**/*.{rs,toml}\""
    )]
    pub include: Option<Vec<String>>,

    /// GitHub repository URL to analyze (e.g., <https://github.com/owner/repo>)
    #[arg(long, help = "Process a GitHub repository")]
    pub repo: Option<String>,

    /// Read prompt from stdin
    #[arg(long = "stdin", help = "Read prompt from standard input")]
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

    /// Enable enhanced context with file metadata
    #[arg(long = "enhanced-context")]
    pub enhanced_context: bool,

    /// Custom priority rules loaded from config file (not a CLI argument)
    #[clap(skip)]
    pub custom_priorities: Vec<crate::config::Priority>,
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
            let directories = self.get_directories();
            for directory in &directories {
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

        // Validate include patterns if provided
        for pattern in &self.get_include_patterns() {
            // Skip empty or whitespace-only patterns
            if pattern.trim().is_empty() {
                continue;
            }

            // Validate pattern syntax using glob crate
            if let Err(e) = glob::Pattern::new(pattern) {
                return Err(CodeDigestError::InvalidConfiguration(format!(
                    "Invalid include pattern '{pattern}': {e}"
                )));
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
            // Store custom priorities for the walker
            self.custom_priorities = config_file.priorities.clone();

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

    /// Get the prompt from the explicit prompt flag
    pub fn get_prompt(&self) -> Option<String> {
        self.prompt.as_ref().filter(|s| !s.trim().is_empty()).cloned()
    }

    /// Get all directories from paths argument
    /// When using --include patterns, this returns the default directory (current dir)
    /// since patterns are handled separately by the walker
    pub fn get_directories(&self) -> Vec<PathBuf> {
        if self.include.is_some() {
            // When using include patterns, use current directory as base
            vec![PathBuf::from(".")]
        } else {
            self.paths.as_ref().cloned().unwrap_or_else(|| vec![PathBuf::from(".")])
        }
    }

    /// Get include patterns if specified
    pub fn get_include_patterns(&self) -> Vec<String> {
        self.include.as_ref().cloned().unwrap_or_default()
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

    impl Config {
        /// Helper function for creating Config instances in tests
        #[allow(dead_code)]
        fn new_for_test(paths: Option<Vec<PathBuf>>) -> Self {
            Self {
                prompt: None,
                paths,
                include: None,
                repo: None,
                read_stdin: false,
                output_file: None,
                max_tokens: None,
                llm_tool: LlmTool::default(),
                quiet: true, // Good default for tests
                verbose: false,
                config: None,
                progress: false,
                copy: false,
                enhanced_context: false,
                custom_priorities: vec![],
            }
        }

        /// Helper function for creating Config instances with include patterns in tests
        #[allow(dead_code)]
        fn new_for_test_with_include(include: Option<Vec<String>>) -> Self {
            Self {
                prompt: None,
                paths: None,
                include,
                repo: None,
                read_stdin: false,
                output_file: None,
                max_tokens: None,
                llm_tool: LlmTool::default(),
                quiet: true, // Good default for tests
                verbose: false,
                config: None,
                progress: false,
                copy: false,
                enhanced_context: false,
                custom_priorities: vec![],
            }
        }
    }

    #[test]
    fn test_config_validation_valid_directory() {
        let temp_dir = TempDir::new().unwrap();
        let config = Config {
            prompt: None,
            paths: Some(vec![temp_dir.path().to_path_buf()]),
            include: None,
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
            enhanced_context: false,
            custom_priorities: vec![],
        };

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation_invalid_directory() {
        let config = Config {
            prompt: None,
            paths: Some(vec![PathBuf::from("/nonexistent/directory")]),
            include: None,
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
            enhanced_context: false,
            custom_priorities: vec![],
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
            paths: Some(vec![file_path]),
            include: None,
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
            enhanced_context: false,
            custom_priorities: vec![],
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_invalid_output_directory() {
        let temp_dir = TempDir::new().unwrap();
        let config = Config {
            prompt: None,
            paths: Some(vec![temp_dir.path().to_path_buf()]),
            include: None,
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
            enhanced_context: false,
            custom_priorities: vec![],
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_mutually_exclusive_options() {
        let temp_dir = TempDir::new().unwrap();
        let config = Config {
            prompt: Some("test prompt".to_string()),
            paths: Some(vec![temp_dir.path().to_path_buf()]),
            include: None,
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
            enhanced_context: false,
            custom_priorities: vec![],
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
            paths: Some(vec![temp_dir.path().to_path_buf()]),
            include: None,
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
            enhanced_context: false,
            custom_priorities: vec![],
        };

        // Should not error for files in current directory
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_load_from_file_no_config() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = Config {
            prompt: None,
            paths: Some(vec![temp_dir.path().to_path_buf()]),
            include: None,
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
            enhanced_context: false,
            custom_priorities: vec![],
        };

        // Should not error when no config file is found
        assert!(config.load_from_file().is_ok());
    }

    #[test]
    fn test_parse_directories() {
        use clap::Parser;

        // Test single directory
        let args = vec!["code-digest", "/path/one"];
        let config = Config::parse_from(args);
        assert_eq!(config.paths.as_ref().unwrap().len(), 1);
        assert_eq!(config.paths.as_ref().unwrap()[0], PathBuf::from("/path/one"));
    }

    #[test]
    fn test_parse_multiple_directories() {
        use clap::Parser;

        // Test multiple directories
        let args = vec!["code-digest", "/path/one", "/path/two", "/path/three"];
        let config = Config::parse_from(args);
        assert_eq!(config.paths.as_ref().unwrap().len(), 3);
        assert_eq!(config.paths.as_ref().unwrap()[0], PathBuf::from("/path/one"));
        assert_eq!(config.paths.as_ref().unwrap()[1], PathBuf::from("/path/two"));
        assert_eq!(config.paths.as_ref().unwrap()[2], PathBuf::from("/path/three"));

        // Test with explicit prompt
        let args = vec!["code-digest", "--prompt", "Find duplicated patterns"];
        let config = Config::parse_from(args);
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
            paths: Some(vec![dir1.clone(), dir2.clone()]),
            include: None,
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
            enhanced_context: false,
            custom_priorities: vec![],
        };
        assert!(config.validate().is_ok());

        // One directory doesn't exist - should fail
        let config = Config {
            prompt: None,
            paths: Some(vec![dir1, PathBuf::from("/nonexistent/dir")]),
            include: None,
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
            enhanced_context: false,
            custom_priorities: vec![],
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
            paths: Some(vec![dir1, file1]),
            include: None,
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
            enhanced_context: false,
            custom_priorities: vec![],
        };
        assert!(config.validate().is_err());
    }
}
