//! Configuration file support for context-creator
//!
//! This module handles loading and parsing configuration files in TOML format.
//! Configuration files can specify defaults for CLI options and additional
//! settings like file priorities and ignore patterns.

use crate::cli::{Config as CliConfig, LlmTool};
use crate::utils::error::ContextCreatorError;
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

    /// Ignore patterns beyond .gitignore and .context-creator-ignore
    #[serde(default)]
    pub ignore: Vec<String>,

    /// Include patterns to force inclusion
    #[serde(default)]
    pub include: Vec<String>,

    /// Token limits for different LLM tools
    #[serde(default)]
    pub tokens: TokenLimits,
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

/// Token limits configuration for different LLM tools
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenLimits {
    /// Maximum tokens for Gemini
    pub gemini: Option<usize>,
    /// Maximum tokens for Codex
    pub codex: Option<usize>,
}

impl ConfigFile {
    /// Load configuration from a file
    pub fn load_from_file(path: &Path) -> Result<Self, ContextCreatorError> {
        if !path.exists() {
            return Err(ContextCreatorError::InvalidPath(format!(
                "Configuration file does not exist: {}",
                path.display()
            )));
        }

        let content = std::fs::read_to_string(path).map_err(|e| {
            ContextCreatorError::ConfigError(format!(
                "Failed to read config file {}: {}",
                path.display(),
                e
            ))
        })?;

        let config: ConfigFile = toml::from_str(&content).map_err(|e| {
            ContextCreatorError::ConfigError(format!(
                "Failed to parse config file {}: {}",
                path.display(),
                e
            ))
        })?;

        Ok(config)
    }

    /// Load configuration from default locations
    pub fn load_default() -> Result<Option<Self>, ContextCreatorError> {
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
        // Apply custom priorities from config file
        cli_config.custom_priorities = self.priorities.clone();

        // Apply token limits from config file
        cli_config.config_token_limits = Some(self.tokens.clone());

        // Store defaults.max_tokens separately to distinguish from explicit CLI values
        if cli_config.max_tokens.is_none() && self.defaults.max_tokens.is_some() {
            cli_config.config_defaults_max_tokens = self.defaults.max_tokens;
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

        // Apply directory default if CLI used default (".") AND no repo is specified
        // This prevents conflict with --repo validation
        let current_paths = cli_config.get_directories();
        if current_paths.len() == 1
            && current_paths[0] == PathBuf::from(".")
            && self.defaults.directory.is_some()
            && cli_config.repo.is_none()
        {
            cli_config.paths = Some(vec![self.defaults.directory.clone().unwrap()]);
        }

        // Apply output file default if not specified
        if cli_config.output_file.is_none() && self.defaults.output_file.is_some() {
            cli_config.output_file = self.defaults.output_file.clone();
        }

        // Apply ignore patterns from config file if no CLI ignore patterns provided
        // CLI ignore patterns always take precedence over config file patterns
        if cli_config.ignore.is_none() && !self.ignore.is_empty() {
            cli_config.ignore = Some(self.ignore.clone());
        }

        // Apply include patterns from config file if no CLI include patterns provided
        // CLI include patterns always take precedence over config file patterns
        if cli_config.include.is_none() && !self.include.is_empty() {
            cli_config.include = Some(self.include.clone());
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
        tokens: TokenLimits {
            gemini: Some(2_000_000),
            codex: Some(1_500_000),
        },
        priorities: vec![
            Priority {
                pattern: "src/**/*.rs".to_string(),
                weight: 100.0,
            },
            Priority {
                pattern: "src/main.rs".to_string(),
                weight: 150.0,
            },
            Priority {
                pattern: "tests/**/*.rs".to_string(),
                weight: 50.0,
            },
            Priority {
                pattern: "docs/**/*.md".to_string(),
                weight: 30.0,
            },
            Priority {
                pattern: "*.toml".to_string(),
                weight: 80.0,
            },
            Priority {
                pattern: "*.json".to_string(),
                weight: 60.0,
            },
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
            tokens: TokenLimits::default(),
            priorities: vec![],
            ignore: vec![],
            include: vec![],
        };

        let mut cli_config = CliConfig {
            prompt: None,
            paths: Some(vec![PathBuf::from(".")]),
            include: None,
            ignore: None,
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
            trace_imports: false,
            include_callers: false,
            include_types: false,
            semantic_depth: 3,
            custom_priorities: vec![],
            config_token_limits: None,
            config_defaults_max_tokens: None,
        };

        config_file.apply_to_cli_config(&mut cli_config);

        assert_eq!(cli_config.config_defaults_max_tokens, Some(75000));
        assert_eq!(cli_config.llm_tool, LlmTool::Codex);
        assert!(cli_config.progress);
        assert!(cli_config.verbose);
        assert_eq!(cli_config.get_directories(), vec![PathBuf::from("/tmp")]);
        assert_eq!(cli_config.output_file, Some(PathBuf::from("output.md")));
    }

    #[test]
    fn test_example_config_generation() {
        let example = create_example_config();
        assert!(example.contains("[defaults]"));
        assert!(example.contains("max_tokens"));
        assert!(example.contains("[tokens]"));
        assert!(example.contains("gemini"));
        assert!(example.contains("codex"));
        assert!(example.contains("[[priorities]]"));
        assert!(example.contains("pattern"));
        assert!(example.contains("weight"));
    }

    #[test]
    fn test_token_limits_parsing() {
        let config_content = r#"
[tokens]
gemini = 2000000
codex = 1500000

[defaults]
max_tokens = 100000
"#;

        let config: ConfigFile = toml::from_str(config_content).unwrap();
        assert_eq!(config.tokens.gemini, Some(2_000_000));
        assert_eq!(config.tokens.codex, Some(1_500_000));
        assert_eq!(config.defaults.max_tokens, Some(100_000));
    }

    #[test]
    fn test_token_limits_partial_parsing() {
        let config_content = r#"
[tokens]
gemini = 3000000
# codex not specified, should use default

[defaults]
max_tokens = 150000
"#;

        let config: ConfigFile = toml::from_str(config_content).unwrap();
        assert_eq!(config.tokens.gemini, Some(3_000_000));
        assert_eq!(config.tokens.codex, None);
    }

    #[test]
    fn test_token_limits_empty_section() {
        let config_content = r#"
[tokens]
# No limits specified

[defaults]
max_tokens = 200000
"#;

        let config: ConfigFile = toml::from_str(config_content).unwrap();
        assert_eq!(config.tokens.gemini, None);
        assert_eq!(config.tokens.codex, None);
    }

    #[test]
    fn test_apply_to_cli_config_with_token_limits() {
        let config_file = ConfigFile {
            defaults: Defaults {
                max_tokens: Some(75000),
                llm_tool: Some("gemini".to_string()),
                progress: true,
                verbose: false,
                quiet: false,
                directory: None,
                output_file: None,
            },
            tokens: TokenLimits {
                gemini: Some(2_500_000),
                codex: Some(1_800_000),
            },
            priorities: vec![],
            ignore: vec![],
            include: vec![],
        };

        let mut cli_config = CliConfig {
            prompt: None,
            paths: Some(vec![PathBuf::from(".")]),
            include: None,
            ignore: None,
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
            trace_imports: false,
            include_callers: false,
            include_types: false,
            semantic_depth: 3,
            custom_priorities: vec![],
            config_token_limits: None,
            config_defaults_max_tokens: None,
        };

        config_file.apply_to_cli_config(&mut cli_config);

        // Token limits should be stored but not directly applied to max_tokens
        assert_eq!(cli_config.config_defaults_max_tokens, Some(75000)); // From defaults
        assert!(cli_config.config_token_limits.is_some());
        let token_limits = cli_config.config_token_limits.as_ref().unwrap();
        assert_eq!(token_limits.gemini, Some(2_500_000));
        assert_eq!(token_limits.codex, Some(1_800_000));
    }
}
