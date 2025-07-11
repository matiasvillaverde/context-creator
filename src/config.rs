//! Configuration file support for code-digest
//!
//! This module handles loading and parsing configuration files in TOML format.
//! Configuration files can specify defaults for CLI options and additional
//! settings like file priorities and ignore patterns.

use crate::cli::{Config as CliConfig, LlmTool};
use crate::utils::error::CodeDigestError;
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
    
    /// Ignore patterns beyond .gitignore and .digestignore
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
    pub fn load_from_file(path: &Path) -> Result<Self, CodeDigestError> {
        if !path.exists() {
            return Err(CodeDigestError::InvalidPath(format!(
                "Configuration file does not exist: {}",
                path.display()
            )));
        }

        let content = std::fs::read_to_string(path)
            .map_err(|e| CodeDigestError::ConfigError(format!(
                "Failed to read config file {}: {}",
                path.display(),
                e
            )))?;

        let config: ConfigFile = toml::from_str(&content)
            .map_err(|e| CodeDigestError::ConfigError(format!(
                "Failed to parse config file {}: {}",
                path.display(),
                e
            )))?;

        Ok(config)
    }

    /// Load configuration from default locations
    pub fn load_default() -> Result<Option<Self>, CodeDigestError> {
        // Try .code-digest.toml in current directory
        let local_config = Path::new(".code-digest.toml");
        if local_config.exists() {
            return Ok(Some(Self::load_from_file(local_config)?));
        }

        // Try .digestrc.toml in current directory
        let rc_config = Path::new(".digestrc.toml");
        if rc_config.exists() {
            return Ok(Some(Self::load_from_file(rc_config)?));
        }

        // Try in home directory
        if let Some(home) = dirs::home_dir() {
            let home_config = home.join(".code-digest.toml");
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
                    "gemini-cli" => cli_config.llm_tool = LlmTool::GeminiCli,
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
        if cli_config.directory == PathBuf::from(".") && self.defaults.directory.is_some() {
            cli_config.directory = self.defaults.directory.clone().unwrap();
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
            llm_tool: Some("gemini-cli".to_string()),
            progress: true,
            verbose: false,
            quiet: false,
            directory: None,
            output_file: None,
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
        include: vec![
            "!important/**".to_string(),
        ],
    };

    toml::to_string_pretty(&example).unwrap_or_else(|_| "# Failed to generate example config".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

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
llm_tool = "gemini-cli"
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
        assert_eq!(config.defaults.llm_tool, Some("gemini-cli".to_string()));
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
            directory: PathBuf::from("."),
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
        assert_eq!(cli_config.directory, PathBuf::from("/tmp"));
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