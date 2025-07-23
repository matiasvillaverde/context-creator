//! Command-line interface configuration and parsing

use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;
use tracing::debug;

/// Help message explaining custom priority rules and usage
const AFTER_HELP_MSG: &str = "\
CUSTOM PRIORITY RULES:
  Custom priority rules are processed in a 'first-match-wins' basis. Rules are 
  evaluated in the order they are defined in your .context-creator.toml configuration 
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
  context-creator --prompt \"Analyze this code\"
  
  # Process specific directories (positional arguments)
  context-creator src/ tests/ docs/
  
  # Process specific directories (explicit include flags)
  context-creator --include src/ --include tests/ --include docs/
  
  # Process files matching glob patterns (QUOTE patterns to prevent shell expansion)
  context-creator --include \"**/*.py\" --include \"src/**/*.{rs,toml}\"
  
  # Process specific file types across all directories
  context-creator --include \"**/*repository*.py\" --include \"**/test[0-9].py\"
  
  # Combine prompt with include patterns for targeted analysis
  context-creator --prompt \"Review security\" --include \"src/auth/**\" --include \"src/security/**\"
  
  # Use ignore patterns to exclude unwanted files
  context-creator --include \"**/*.rs\" --ignore \"target/**\" --ignore \"**/*_test.rs\"
  
  # Combine prompt with ignore patterns
  context-creator --prompt \"Analyze core logic\" --ignore \"tests/**\" --ignore \"docs/**\"
  
  # Process a GitHub repository
  context-creator --remote https://github.com/owner/repo
  
  # Read prompt from stdin
  echo \"Review this code\" | context-creator --stdin .
  
  # FLEXIBLE COMBINATIONS (NEW):
  # Combine prompt with specific directories
  context-creator --prompt \"Security audit\" src/auth/ src/security/
  
  # Combine prompt with GitHub repository
  context-creator --prompt \"Find bugs\" --remote https://github.com/owner/repo
  
  # Combine stdin with specific directories
  echo \"Analyze patterns\" | context-creator --stdin src/ tests/
  
  # Combine include patterns with GitHub repository
  context-creator --include \"**/*.rs\" --remote https://github.com/owner/repo
  
  # Combine stdin with include patterns
  echo \"Review code\" | context-creator --stdin --include \"**/*.py\"
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

/// Log output format options
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Default)]
pub enum LogFormat {
    /// Human-readable plain text format (default)
    #[value(name = "plain")]
    #[default]
    Plain,
    /// Machine-readable JSON format
    #[value(name = "json")]
    Json,
}

/// Output format options for the generated context
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Default)]
pub enum OutputFormat {
    /// Markdown format (default)
    #[value(name = "markdown")]
    #[default]
    Markdown,
    /// XML format with structured data
    #[value(name = "xml")]
    Xml,
    /// Plain text format
    #[value(name = "plain")]
    Plain,
    /// List of file paths only
    #[value(name = "paths")]
    Paths,
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

    /// Get the default maximum tokens for the tool
    pub fn default_max_tokens(&self) -> usize {
        match self {
            LlmTool::Gemini => 1_000_000,
            LlmTool::Codex => 1_000_000,
        }
    }

    /// Get the default maximum tokens for the tool with optional config override
    pub fn default_max_tokens_with_config(
        &self,
        config_token_limits: Option<&crate::config::TokenLimits>,
    ) -> usize {
        if let Some(token_limits) = config_token_limits {
            match self {
                LlmTool::Gemini => token_limits.gemini.unwrap_or(1_000_000),
                LlmTool::Codex => token_limits.codex.unwrap_or(1_000_000),
            }
        } else {
            self.default_max_tokens()
        }
    }
}

/// Available commands for context-creator
#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    /// Search for files containing the specified term
    Search {
        /// Search pattern (case-insensitive)
        pattern: String,

        /// Disable automatic semantic analysis
        #[arg(long = "no-semantic")]
        no_semantic: bool,

        /// Search within specific paths
        #[arg(value_name = "PATHS")]
        paths: Option<Vec<PathBuf>>,
    },
}

/// High-performance CLI tool to convert codebases to Markdown for LLM context
#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None, after_help = AFTER_HELP_MSG)]
pub struct Config {
    /// Subcommand to execute
    #[command(subcommand)]
    pub command: Option<Commands>,
    /// The prompt to send to the LLM for processing
    #[arg(short = 'p', long = "prompt", help = "Process a text prompt directly")]
    pub prompt: Option<String>,

    /// One or more directory paths to process
    /// IMPORTANT: Use `get_directories()` to access the correct input paths.
    #[arg(value_name = "PATHS", help = "Process files and directories")]
    pub paths: Option<Vec<PathBuf>>,

    /// Include files and directories matching glob patterns
    /// IMPORTANT: Use `get_directories()` to access the correct input paths.
    #[arg(
        long,
        help = "Include files and directories matching the given glob pattern.\nPatterns use gitignore-style syntax. To prevent shell expansion,\nquote patterns: --include \"*.py\" --include \"src/**/*.{rs,toml}\""
    )]
    pub include: Option<Vec<String>>,

    /// Ignore files and directories matching glob patterns
    #[arg(
        long,
        help = "Ignore files and directories matching the given glob pattern.\nPatterns use gitignore-style syntax. To prevent shell expansion,\nquote patterns: --ignore \"node_modules/**\" --ignore \"target/**\""
    )]
    pub ignore: Option<Vec<String>>,

    /// GitHub repository URL to analyze (e.g., <https://github.com/owner/repo>)
    #[arg(long, help = "Process a GitHub repository")]
    pub remote: Option<String>,

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

    /// Enable verbose logging (use -vv for trace level)
    #[arg(short = 'v', long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Log output format
    #[arg(long = "log-format", value_enum, default_value = "plain")]
    pub log_format: LogFormat,

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

    /// Output format style
    #[arg(long = "style", value_enum, default_value = "markdown")]
    pub output_format: OutputFormat,

    /// Enable import tracing for included files
    #[arg(long, help = "Include files that import the specified modules")]
    pub trace_imports: bool,

    /// Include files that call functions from specified modules
    #[arg(long, help = "Include files containing callers of specified functions")]
    pub include_callers: bool,

    /// Include type definitions used by specified files
    #[arg(long, help = "Include type definitions and interfaces")]
    pub include_types: bool,

    /// Maximum depth for semantic dependency traversal
    #[arg(
        long,
        default_value = "5",
        help = "Depth limit for dependency traversal"
    )]
    pub semantic_depth: usize,

    /// Custom priority rules loaded from config file (not a CLI argument)
    #[clap(skip)]
    pub custom_priorities: Vec<crate::config::Priority>,

    /// Token limits loaded from config file (not a CLI argument)
    #[clap(skip)]
    pub config_token_limits: Option<crate::config::TokenLimits>,

    /// Maximum tokens from config defaults (not a CLI argument)
    #[clap(skip)]
    pub config_defaults_max_tokens: Option<usize>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            command: None,
            prompt: None,
            paths: None,
            include: None,
            ignore: None,
            remote: None,
            read_stdin: false,
            output_file: None,
            max_tokens: None,
            llm_tool: LlmTool::default(),
            quiet: false,
            verbose: 0,
            log_format: LogFormat::default(),
            config: None,
            progress: false,
            copy: false,
            enhanced_context: false,
            output_format: OutputFormat::default(),
            trace_imports: false,
            include_callers: false,
            include_types: false,
            semantic_depth: 5,
            custom_priorities: vec![],
            config_token_limits: None,
            config_defaults_max_tokens: None,
        }
    }
}

impl Config {
    /// Validate the configuration
    pub fn validate(&self) -> Result<(), crate::utils::error::ContextCreatorError> {
        use crate::utils::error::ContextCreatorError;

        // If a command is provided, it's a valid input source on its own
        if self.command.is_some() {
            return Ok(());
        }

        // Validate that at least one input source is provided
        let has_input_source = self.get_prompt().is_some()
            || self.paths.is_some()
            || self.include.is_some()
            || self.remote.is_some()
            || self.read_stdin;

        if !has_input_source {
            return Err(ContextCreatorError::InvalidConfiguration(
                "At least one input source must be provided: --prompt, paths, --include, --remote, or --stdin".to_string(),
            ));
        }

        // Validate verbose and quiet mutual exclusion
        if self.verbose > 0 && self.quiet {
            return Err(ContextCreatorError::InvalidConfiguration(
                "Cannot use both --verbose (-v) and --quiet (-q) flags together".to_string(),
            ));
        }

        // Note: Removed overly restrictive validation rules per issue #34
        // Now allowing flexible combinations like:
        // - --prompt with paths (--prompt "text" src/)
        // - --prompt with --remote (--prompt "text" --remote url)
        // - --stdin with paths (echo "prompt" | context-creator --stdin src/)
        // - --include with --remote (--include "**/*.rs" --remote url)
        // - --include with --stdin (--stdin --include "**/*.rs")
        //
        // The only remaining restrictions are for legitimate conflicts:
        // - --prompt with --output-file (can't send to LLM and write to file)
        // - --copy with --output-file (can't copy to clipboard and write to file)

        // Validate repo URL if provided
        if let Some(repo_url) = &self.remote {
            if !repo_url.starts_with("https://github.com/")
                && !repo_url.starts_with("http://github.com/")
            {
                return Err(ContextCreatorError::InvalidConfiguration(
                    "Repository URL must be a GitHub URL (https://github.com/owner/repo)"
                        .to_string(),
                ));
            }
        } else {
            // Only validate paths if repo is not provided
            let paths = self.get_directories();
            for path in &paths {
                if !path.exists() {
                    return Err(ContextCreatorError::InvalidPath(format!(
                        "Path does not exist: {}",
                        path.display()
                    )));
                }

                // Allow both files and directories
                if !path.is_dir() && !path.is_file() {
                    return Err(ContextCreatorError::InvalidPath(format!(
                        "Path is neither a file nor a directory: {}",
                        path.display()
                    )));
                }
            }
        }

        // Note: Pattern validation is handled by OverrideBuilder in walker.rs
        // which provides better security and ReDoS protection

        // Validate output file parent directory exists if specified
        if let Some(output) = &self.output_file {
            if let Some(parent) = output.parent() {
                // Handle empty parent (current directory) and check if parent exists
                if !parent.as_os_str().is_empty() && !parent.exists() {
                    return Err(ContextCreatorError::InvalidPath(format!(
                        "Output directory does not exist: {}",
                        parent.display()
                    )));
                }
            }
        }

        // Validate mutually exclusive options
        if self.output_file.is_some() && self.get_prompt().is_some() {
            return Err(ContextCreatorError::InvalidConfiguration(
                "Cannot specify both --output and a prompt".to_string(),
            ));
        }

        // Validate copy and output mutual exclusivity
        if self.copy && self.output_file.is_some() {
            return Err(ContextCreatorError::InvalidConfiguration(
                "Cannot specify both --copy and --output".to_string(),
            ));
        }

        // Validate repo and paths mutual exclusivity
        // When --remote is specified, any positional paths are silently ignored in run()
        // This prevents user confusion by failing early with a clear error message
        if self.remote.is_some() && self.paths.is_some() {
            return Err(ContextCreatorError::InvalidConfiguration(
                "Cannot specify both --remote and local paths. Use --remote to analyze a remote repository, or provide local paths to analyze local directories.".to_string(),
            ));
        }

        Ok(())
    }

    /// Load configuration from file if specified
    pub fn load_from_file(&mut self) -> Result<(), crate::utils::error::ContextCreatorError> {
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

            // Store token limits for token resolution
            self.config_token_limits = Some(config_file.tokens.clone());

            config_file.apply_to_cli_config(self);

            if self.verbose > 0 {
                if let Some(ref config_path) = self.config {
                    debug!("Loaded configuration from: {}", config_path.display());
                } else {
                    debug!("Loaded configuration from default location");
                }
            }
        }

        Ok(())
    }

    /// Get the prompt from the explicit prompt flag
    pub fn get_prompt(&self) -> Option<String> {
        self.prompt
            .as_ref()
            .filter(|s| !s.trim().is_empty())
            .cloned()
    }

    /// Get all directories from paths argument
    /// When using --include patterns, this returns the default directory (current dir)
    /// unless explicit paths are also provided (flexible combinations)
    pub fn get_directories(&self) -> Vec<PathBuf> {
        // If explicit paths are provided, use them
        if let Some(paths) = &self.paths {
            paths.clone()
        } else if self.include.is_some() {
            // When using include patterns without explicit paths, use current directory as base
            vec![PathBuf::from(".")]
        } else {
            // Default to current directory
            vec![PathBuf::from(".")]
        }
    }

    /// Get include patterns if specified
    pub fn get_include_patterns(&self) -> Vec<String> {
        self.include.as_ref().cloned().unwrap_or_default()
    }

    /// Get ignore patterns if specified
    pub fn get_ignore_patterns(&self) -> Vec<String> {
        self.ignore.as_ref().cloned().unwrap_or_default()
    }

    /// Get effective max tokens with precedence: explicit CLI > token limits (if prompt) > config defaults > hard-coded defaults (if prompt) > None
    pub fn get_effective_max_tokens(&self) -> Option<usize> {
        // 1. Explicit CLI value always takes precedence
        if let Some(explicit_tokens) = self.max_tokens {
            return Some(explicit_tokens);
        }

        // 2. If using prompt, check token limits from config first
        if let Some(_prompt) = self.get_prompt() {
            // Check if we have config token limits for this tool
            if let Some(token_limits) = &self.config_token_limits {
                let config_limit = match self.llm_tool {
                    LlmTool::Gemini => token_limits.gemini,
                    LlmTool::Codex => token_limits.codex,
                };

                if let Some(limit) = config_limit {
                    return Some(limit);
                }
            }

            // 3. Fall back to config defaults if available
            if let Some(defaults_tokens) = self.config_defaults_max_tokens {
                return Some(defaults_tokens);
            }

            // 4. Fall back to hard-coded defaults for prompts
            return Some(self.llm_tool.default_max_tokens());
        }

        // 5. For non-prompt usage, check config defaults
        if let Some(defaults_tokens) = self.config_defaults_max_tokens {
            return Some(defaults_tokens);
        }

        // 6. No automatic token limits for non-prompt usage
        None
    }

    /// Get effective context tokens with prompt reservation
    /// This accounts for prompt tokens when calculating available space for codebase context
    pub fn get_effective_context_tokens(&self) -> Option<usize> {
        if let Some(max_tokens) = self.get_effective_max_tokens() {
            if let Some(prompt) = self.get_prompt() {
                // Create token counter to measure prompt
                if let Ok(counter) = crate::core::token::TokenCounter::new() {
                    if let Ok(prompt_tokens) = counter.count_tokens(&prompt) {
                        // Reserve space for prompt + safety buffer for response
                        let safety_buffer = 1000; // Reserve for LLM response
                        let reserved = prompt_tokens + safety_buffer;
                        let available = max_tokens.saturating_sub(reserved);
                        return Some(available);
                    }
                }
                // Fallback: rough estimation if tiktoken fails
                let estimated_prompt_tokens = prompt.len().div_ceil(4); // ~4 chars per token
                let safety_buffer = 1000;
                let reserved = estimated_prompt_tokens + safety_buffer;
                let available = max_tokens.saturating_sub(reserved);
                Some(available)
            } else {
                // No prompt, use full token budget
                Some(max_tokens)
            }
        } else {
            None
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

    impl Config {
        /// Helper function for creating Config instances in tests
        #[allow(dead_code)]
        fn new_for_test(paths: Option<Vec<PathBuf>>) -> Self {
            Self {
                paths,
                quiet: true, // Good default for tests
                ..Self::default()
            }
        }

        /// Helper function for creating Config instances with include patterns in tests
        #[allow(dead_code)]
        fn new_for_test_with_include(include: Option<Vec<String>>) -> Self {
            Self {
                include,
                quiet: true, // Good default for tests
                ..Self::default()
            }
        }
    }

    #[test]
    fn test_config_validation_valid_directory() {
        let temp_dir = TempDir::new().unwrap();
        let config = Config {
            paths: Some(vec![temp_dir.path().to_path_buf()]),
            ..Default::default()
        };

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation_invalid_directory() {
        let config = Config {
            paths: Some(vec![PathBuf::from("/nonexistent/directory")]),
            ..Default::default()
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_file_as_directory() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("file.txt");
        fs::write(&file_path, "test").unwrap();

        let config = Config {
            paths: Some(vec![file_path]),
            ..Default::default()
        };

        // Files are now allowed as paths
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation_invalid_output_directory() {
        let temp_dir = TempDir::new().unwrap();
        let config = Config {
            paths: Some(vec![temp_dir.path().to_path_buf()]),
            output_file: Some(PathBuf::from("/nonexistent/directory/output.md")),
            ..Default::default()
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_mutually_exclusive_options() {
        let temp_dir = TempDir::new().unwrap();
        let config = Config {
            prompt: Some("test prompt".to_string()),
            paths: Some(vec![temp_dir.path().to_path_buf()]),
            output_file: Some(temp_dir.path().join("output.md")),
            ..Default::default()
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_llm_tool_enum_values() {
        assert_eq!(LlmTool::Gemini.command(), "gemini");
        assert_eq!(LlmTool::Codex.command(), "codex");

        assert!(LlmTool::Gemini
            .install_instructions()
            .contains("pip install"));
        assert!(LlmTool::Codex.install_instructions().contains("github.com"));

        assert_eq!(LlmTool::default(), LlmTool::Gemini);
    }

    #[test]
    fn test_llm_tool_default_max_tokens() {
        assert_eq!(LlmTool::Gemini.default_max_tokens(), 1_000_000);
        assert_eq!(LlmTool::Codex.default_max_tokens(), 1_000_000);
    }

    #[test]
    fn test_config_get_effective_max_tokens_with_explicit() {
        let config = Config {
            prompt: Some("test prompt".to_string()),
            max_tokens: Some(500_000),
            llm_tool: LlmTool::Gemini,
            ..Config::new_for_test(None)
        };
        assert_eq!(config.get_effective_max_tokens(), Some(500_000));
    }

    #[test]
    fn test_config_get_effective_max_tokens_with_prompt_default() {
        let config = Config {
            prompt: Some("test prompt".to_string()),
            max_tokens: None,
            llm_tool: LlmTool::Gemini,
            ..Config::new_for_test(None)
        };
        assert_eq!(config.get_effective_max_tokens(), Some(1_000_000));
    }

    #[test]
    fn test_config_get_effective_max_tokens_no_prompt() {
        let config = Config {
            prompt: None,
            max_tokens: None,
            llm_tool: LlmTool::Gemini,
            ..Config::new_for_test(None)
        };
        assert_eq!(config.get_effective_max_tokens(), None);
    }

    #[test]
    fn test_config_get_effective_max_tokens_with_config_gemini() {
        use crate::config::TokenLimits;

        let config = Config {
            prompt: Some("test prompt".to_string()),
            max_tokens: None,
            llm_tool: LlmTool::Gemini,
            config_token_limits: Some(TokenLimits {
                gemini: Some(2_500_000),
                codex: Some(1_800_000),
            }),
            ..Config::new_for_test(None)
        };
        assert_eq!(config.get_effective_max_tokens(), Some(2_500_000));
    }

    #[test]
    fn test_config_get_effective_max_tokens_with_config_codex() {
        use crate::config::TokenLimits;

        let config = Config {
            prompt: Some("test prompt".to_string()),
            max_tokens: None,
            llm_tool: LlmTool::Codex,
            config_token_limits: Some(TokenLimits {
                gemini: Some(2_500_000),
                codex: Some(1_800_000),
            }),
            ..Config::new_for_test(None)
        };
        assert_eq!(config.get_effective_max_tokens(), Some(1_800_000));
    }

    #[test]
    fn test_config_get_effective_max_tokens_explicit_overrides_config() {
        use crate::config::TokenLimits;

        let config = Config {
            prompt: Some("test prompt".to_string()),
            max_tokens: Some(500_000), // Explicit value should override config
            llm_tool: LlmTool::Gemini,
            config_token_limits: Some(TokenLimits {
                gemini: Some(2_500_000),
                codex: Some(1_800_000),
            }),
            ..Config::new_for_test(None)
        };
        assert_eq!(config.get_effective_max_tokens(), Some(500_000));
    }

    #[test]
    fn test_config_get_effective_max_tokens_config_partial_gemini() {
        use crate::config::TokenLimits;

        let config = Config {
            prompt: Some("test prompt".to_string()),
            max_tokens: None,
            llm_tool: LlmTool::Gemini,
            config_token_limits: Some(TokenLimits {
                gemini: Some(3_000_000),
                codex: None, // Codex not configured
            }),
            ..Config::new_for_test(None)
        };
        assert_eq!(config.get_effective_max_tokens(), Some(3_000_000));
    }

    #[test]
    fn test_config_get_effective_max_tokens_config_partial_codex() {
        use crate::config::TokenLimits;

        let config = Config {
            prompt: Some("test prompt".to_string()),
            max_tokens: None,
            llm_tool: LlmTool::Codex,
            config_token_limits: Some(TokenLimits {
                gemini: None, // Gemini not configured
                codex: Some(1_200_000),
            }),
            ..Config::new_for_test(None)
        };
        assert_eq!(config.get_effective_max_tokens(), Some(1_200_000));
    }

    #[test]
    fn test_config_get_effective_max_tokens_config_fallback_to_default() {
        use crate::config::TokenLimits;

        let config = Config {
            prompt: Some("test prompt".to_string()),
            max_tokens: None,
            llm_tool: LlmTool::Gemini,
            config_token_limits: Some(TokenLimits {
                gemini: None, // No limit configured for Gemini
                codex: Some(1_800_000),
            }),
            ..Config::new_for_test(None)
        };
        // Should fall back to hard-coded default
        assert_eq!(config.get_effective_max_tokens(), Some(1_000_000));
    }

    #[test]
    fn test_llm_tool_default_max_tokens_with_config() {
        use crate::config::TokenLimits;

        let token_limits = TokenLimits {
            gemini: Some(2_500_000),
            codex: Some(1_800_000),
        };

        assert_eq!(
            LlmTool::Gemini.default_max_tokens_with_config(Some(&token_limits)),
            2_500_000
        );
        assert_eq!(
            LlmTool::Codex.default_max_tokens_with_config(Some(&token_limits)),
            1_800_000
        );
    }

    #[test]
    fn test_llm_tool_default_max_tokens_with_config_partial() {
        use crate::config::TokenLimits;

        let token_limits = TokenLimits {
            gemini: Some(3_000_000),
            codex: None, // Codex not configured
        };

        assert_eq!(
            LlmTool::Gemini.default_max_tokens_with_config(Some(&token_limits)),
            3_000_000
        );
        // Should fall back to hard-coded default
        assert_eq!(
            LlmTool::Codex.default_max_tokens_with_config(Some(&token_limits)),
            1_000_000
        );
    }

    #[test]
    fn test_llm_tool_default_max_tokens_with_no_config() {
        assert_eq!(
            LlmTool::Gemini.default_max_tokens_with_config(None),
            1_000_000
        );
        assert_eq!(
            LlmTool::Codex.default_max_tokens_with_config(None),
            1_000_000
        );
    }

    #[test]
    fn test_get_effective_context_tokens_with_prompt() {
        let config = Config {
            prompt: Some("This is a test prompt".to_string()),
            max_tokens: Some(10000),
            llm_tool: LlmTool::Gemini,
            ..Config::new_for_test(None)
        };

        let context_tokens = config.get_effective_context_tokens().unwrap();
        // Should be less than max_tokens due to prompt + safety buffer reservation
        assert!(context_tokens < 10000);
        // Should be at least max_tokens - 1000 (safety buffer) - prompt tokens
        assert!(context_tokens > 8000); // Conservative estimate
    }

    #[test]
    fn test_get_effective_context_tokens_no_prompt() {
        let config = Config {
            prompt: None,
            max_tokens: Some(10000),
            llm_tool: LlmTool::Gemini,
            ..Config::new_for_test(None)
        };

        // Without prompt, should use full token budget
        assert_eq!(config.get_effective_context_tokens(), Some(10000));
    }

    #[test]
    fn test_get_effective_context_tokens_no_limit() {
        let config = Config {
            prompt: None, // No prompt means no auto-limits
            max_tokens: None,
            llm_tool: LlmTool::Gemini,
            ..Config::new_for_test(None)
        };

        // No max tokens configured and no prompt, should return None
        assert_eq!(config.get_effective_context_tokens(), None);
    }

    #[test]
    fn test_get_effective_context_tokens_with_config_limits() {
        use crate::config::TokenLimits;

        let config = Config {
            prompt: Some("This is a longer test prompt for token counting".to_string()),
            max_tokens: None, // Use config limits instead
            llm_tool: LlmTool::Gemini,
            config_token_limits: Some(TokenLimits {
                gemini: Some(50000),
                codex: Some(40000),
            }),
            ..Config::new_for_test(None)
        };

        let context_tokens = config.get_effective_context_tokens().unwrap();
        // Should be less than config limit due to prompt reservation
        assert!(context_tokens < 50000);
        assert!(context_tokens > 45000); // Should be most of the budget
    }

    #[test]
    fn test_config_validation_output_file_in_current_dir() {
        let temp_dir = TempDir::new().unwrap();
        let config = Config {
            paths: Some(vec![temp_dir.path().to_path_buf()]),
            output_file: Some(PathBuf::from("output.md")),
            ..Default::default()
        };

        // Should not error for files in current directory
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_load_from_file_no_config() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = Config {
            paths: Some(vec![temp_dir.path().to_path_buf()]),
            ..Default::default()
        };

        // Should not error when no config file is found
        assert!(config.load_from_file().is_ok());
    }

    #[test]
    fn test_parse_directories() {
        use clap::Parser;

        // Test single directory
        let args = vec!["context-creator", "/path/one"];
        let config = Config::parse_from(args);
        assert_eq!(config.paths.as_ref().unwrap().len(), 1);
        assert_eq!(
            config.paths.as_ref().unwrap()[0],
            PathBuf::from("/path/one")
        );
    }

    #[test]
    fn test_parse_multiple_directories() {
        use clap::Parser;

        // Test multiple directories
        let args = vec!["context-creator", "/path/one", "/path/two", "/path/three"];
        let config = Config::parse_from(args);
        assert_eq!(config.paths.as_ref().unwrap().len(), 3);
        assert_eq!(
            config.paths.as_ref().unwrap()[0],
            PathBuf::from("/path/one")
        );
        assert_eq!(
            config.paths.as_ref().unwrap()[1],
            PathBuf::from("/path/two")
        );
        assert_eq!(
            config.paths.as_ref().unwrap()[2],
            PathBuf::from("/path/three")
        );

        // Test with explicit prompt
        let args = vec!["context-creator", "--prompt", "Find duplicated patterns"];
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
            paths: Some(vec![dir1.clone(), dir2.clone()]),
            ..Default::default()
        };
        assert!(config.validate().is_ok());

        // One directory doesn't exist - should fail
        let config = Config {
            paths: Some(vec![dir1, PathBuf::from("/nonexistent/dir")]),
            ..Default::default()
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

        // Mix of directory and file - now allowed
        let config = Config {
            paths: Some(vec![dir1, file1]),
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }
}
