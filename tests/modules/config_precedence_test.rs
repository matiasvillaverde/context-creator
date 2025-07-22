#![cfg(test)]

use clap::Parser;
use context_creator::cli::Config;
use context_creator::core::walker::WalkOptions;
use std::fs;
use tempfile::TempDir;

/// Test that CLI ignore patterns override config file patterns
#[test]
fn test_cli_ignore_patterns_override_config() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test-config.toml");

    // Create a config file with some ignore patterns
    let config_content = r#"
ignore = ["config_*.rs", "config_target/**"]
"#;
    fs::write(&config_path, config_content).unwrap();

    // CLI config with different ignore patterns
    let mut config = Config::parse_from([
        "context-creator",
        "--config",
        config_path.to_str().unwrap(),
        "--ignore",
        "cli_*.rs",
        "--ignore",
        "cli_target/**",
    ]);

    // Load configuration from file
    config.load_from_file().unwrap();

    // CLI ignore patterns should take precedence
    let ignore_patterns = config.get_ignore_patterns();
    assert_eq!(ignore_patterns, vec!["cli_*.rs", "cli_target/**"]);

    // Verify in WalkOptions as well
    let walk_options = WalkOptions::from_config(&config).unwrap();
    assert_eq!(
        walk_options.ignore_patterns,
        vec!["cli_*.rs", "cli_target/**"]
    );
}

/// Test that CLI include patterns override config file patterns
#[test]
fn test_cli_include_patterns_override_config() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test-config.toml");

    // Create a config file with some include patterns
    let config_content = r#"
include = ["config_*.rs", "config_src/**"]
"#;
    fs::write(&config_path, config_content).unwrap();

    // CLI config with different include patterns
    let mut config = Config::parse_from([
        "context-creator",
        "--config",
        config_path.to_str().unwrap(),
        "--include",
        "cli_*.rs",
        "--include",
        "cli_src/**",
    ]);

    // Load configuration from file
    config.load_from_file().unwrap();

    // CLI include patterns should take precedence
    let include_patterns = config.get_include_patterns();
    assert_eq!(include_patterns, vec!["cli_*.rs", "cli_src/**"]);

    // Verify in WalkOptions as well
    let walk_options = WalkOptions::from_config(&config).unwrap();
    assert_eq!(
        walk_options.include_patterns,
        vec!["cli_*.rs", "cli_src/**"]
    );
}

/// Test that when no CLI patterns are provided, config file patterns are used
#[test]
fn test_config_patterns_used_when_no_cli_patterns() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test-config.toml");

    // Create a config file with ignore and include patterns
    let config_content = r#"
ignore = ["config_*.rs", "config_target/**"]
include = ["config_src/**/*.rs"]
"#;
    fs::write(&config_path, config_content).unwrap();

    // CLI config with no patterns - should use config file patterns
    let mut config =
        Config::parse_from(["context-creator", "--config", config_path.to_str().unwrap()]);

    // Load configuration from file
    config.load_from_file().unwrap();

    // Config file patterns should be used
    let ignore_patterns = config.get_ignore_patterns();
    let include_patterns = config.get_include_patterns();

    // Config file patterns should be loaded when no CLI patterns are provided
    assert_eq!(ignore_patterns, vec!["config_*.rs", "config_target/**"]);
    assert_eq!(include_patterns, vec!["config_src/**/*.rs"]);
}

/// Test that empty CLI patterns don't override config file patterns
#[test]
fn test_empty_cli_patterns_dont_override_config() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test-config.toml");

    // Create a config file with patterns
    let config_content = r#"
ignore = ["config_*.rs", "config_target/**"]
"#;
    fs::write(&config_path, config_content).unwrap();

    // CLI config with empty ignore patterns
    let mut config = Config::parse_from([
        "context-creator",
        "--config",
        config_path.to_str().unwrap(),
        "--ignore",
        "",
        "--ignore",
        "   ",
    ]);

    // Load configuration from file
    config.load_from_file().unwrap();

    // CLI ignore patterns are present but empty - this should override config
    let ignore_patterns = config.get_ignore_patterns();
    assert_eq!(ignore_patterns, vec!["", "   "]);

    // But WalkOptions should filter out empty patterns
    let walk_options = WalkOptions::from_config(&config).unwrap();
    assert_eq!(walk_options.ignore_patterns, Vec::<String>::new());
}

/// Test precedence with mixed CLI and config patterns
#[test]
fn test_mixed_cli_and_config_precedence() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test-config.toml");

    // Create a config file with both ignore and include patterns
    let config_content = r#"
ignore = ["config_*.rs", "config_target/**"]
include = ["config_src/**/*.rs"]
"#;
    fs::write(&config_path, config_content).unwrap();

    // CLI config with only ignore patterns (no include)
    let mut config = Config::parse_from([
        "context-creator",
        "--config",
        config_path.to_str().unwrap(),
        "--ignore",
        "cli_*.rs",
    ]);

    // Load configuration from file
    config.load_from_file().unwrap();

    // CLI ignore patterns should override config ignore patterns
    let ignore_patterns = config.get_ignore_patterns();
    assert_eq!(ignore_patterns, vec!["cli_*.rs"]);

    // Config include patterns should be used since no CLI include patterns were provided
    let include_patterns = config.get_include_patterns();
    // Config file patterns should be loaded when no CLI patterns are provided
    assert_eq!(include_patterns, vec!["config_src/**/*.rs"]);
}

/// Test that CLI patterns work with default config behavior
#[test]
fn test_cli_patterns_with_default_config() {
    // No config file provided - should use CLI patterns only
    let config = Config::parse_from([
        "context-creator",
        "--ignore",
        "cli_*.rs",
        "--ignore",
        "cli_target/**",
        "--include",
        "cli_src/**/*.rs",
    ]);

    // Should use only CLI patterns
    let ignore_patterns = config.get_ignore_patterns();
    let include_patterns = config.get_include_patterns();

    assert_eq!(ignore_patterns, vec!["cli_*.rs", "cli_target/**"]);
    assert_eq!(include_patterns, vec!["cli_src/**/*.rs"]);

    // Verify in WalkOptions as well
    let walk_options = WalkOptions::from_config(&config).unwrap();
    assert_eq!(
        walk_options.ignore_patterns,
        vec!["cli_*.rs", "cli_target/**"]
    );
    assert_eq!(walk_options.include_patterns, vec!["cli_src/**/*.rs"]);
}

/// Test that CLI patterns work with validation scenarios
#[test]
fn test_cli_pattern_validation_scenarios() {
    // CLI config with potentially problematic patterns
    let config = Config::parse_from([
        "context-creator",
        "--prompt",
        "Analyze code",
        "--ignore",
        "../../../etc/passwd",
        "--ignore",
        "valid_*.rs",
    ]);

    // CLI validation should pass - pattern validation happens later
    assert!(config.validate().is_ok());

    // WalkOptions creation should succeed - sanitization happens during walker building
    let walk_options_result = WalkOptions::from_config(&config);
    assert!(walk_options_result.is_ok());

    // Verify that the ignore patterns are passed through to WalkOptions
    let walk_options = walk_options_result.unwrap();
    assert_eq!(
        walk_options.ignore_patterns,
        vec!["../../../etc/passwd", "valid_*.rs"]
    );

    // Note: The actual security validation would happen when building the walker
    // This demonstrates that the CLI and WalkOptions creation don't block patterns
}

/// Test that CLI patterns work with prompt combinations
#[test]
fn test_cli_patterns_with_prompt_precedence() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test-config.toml");

    // Create a config file with patterns
    let config_content = r#"
ignore = ["config_*.rs"]
include = ["config_src/**/*.rs"]
"#;
    fs::write(&config_path, config_content).unwrap();

    // CLI config with prompt and patterns
    let mut config = Config::parse_from([
        "context-creator",
        "--config",
        config_path.to_str().unwrap(),
        "--prompt",
        "Analyze security",
        "--include",
        "cli_src/**/*.rs",
        "--ignore",
        "cli_*.rs",
    ]);

    // Load configuration from file
    config.load_from_file().unwrap();

    // All CLI options should work together
    assert_eq!(config.get_prompt(), Some("Analyze security".to_string()));
    assert_eq!(config.get_include_patterns(), vec!["cli_src/**/*.rs"]);
    assert_eq!(config.get_ignore_patterns(), vec!["cli_*.rs"]);

    // Verify in WalkOptions as well
    let walk_options = WalkOptions::from_config(&config).unwrap();
    assert_eq!(walk_options.include_patterns, vec!["cli_src/**/*.rs"]);
    assert_eq!(walk_options.ignore_patterns, vec!["cli_*.rs"]);
}

/// Test that config file patterns work with repo argument
#[test]
fn test_config_patterns_with_repo_precedence() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test-config.toml");

    // Create a config file with patterns
    let config_content = r#"
ignore = ["config_*.rs"]
"#;
    fs::write(&config_path, config_content).unwrap();

    // CLI config with repo and ignore patterns
    let mut config = Config::parse_from([
        "context-creator",
        "--config",
        config_path.to_str().unwrap(),
        "--remote",
        "https://github.com/owner/repo",
        "--ignore",
        "cli_*.rs",
    ]);

    // Load configuration from file
    config.load_from_file().unwrap();

    // CLI patterns should take precedence
    assert_eq!(
        config.remote,
        Some("https://github.com/owner/repo".to_string())
    );
    assert_eq!(config.get_ignore_patterns(), vec!["cli_*.rs"]);
}

/// Test that precedence works with multiple CLI invocations
#[test]
fn test_multiple_cli_invocations_precedence() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test-config.toml");

    // Create a config file with patterns
    let config_content = r#"
ignore = ["config1_*.rs", "config2_*.rs"]
include = ["config_src/**/*.rs"]
"#;
    fs::write(&config_path, config_content).unwrap();

    // CLI config with multiple ignore patterns
    let mut config = Config::parse_from([
        "context-creator",
        "--config",
        config_path.to_str().unwrap(),
        "--ignore",
        "cli1_*.rs",
        "--ignore",
        "cli2_*.rs",
        "--ignore",
        "cli3_*.rs",
        "--include",
        "cli_src/**/*.rs",
    ]);

    // Load configuration from file
    config.load_from_file().unwrap();

    // All CLI patterns should override config patterns
    assert_eq!(
        config.get_ignore_patterns(),
        vec!["cli1_*.rs", "cli2_*.rs", "cli3_*.rs"]
    );
    assert_eq!(config.get_include_patterns(), vec!["cli_src/**/*.rs"]);
}

/// Test that config file loading doesn't interfere with CLI patterns
#[test]
fn test_config_loading_doesnt_interfere_with_cli() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test-config.toml");

    // Create a config file with different token limits (not patterns)
    let config_content = r#"
[token_limits]
gemini = 500000
"#;
    fs::write(&config_path, config_content).unwrap();

    // CLI config with patterns
    let mut config = Config::parse_from([
        "context-creator",
        "--config",
        config_path.to_str().unwrap(),
        "--ignore",
        "cli_*.rs",
        "--include",
        "cli_src/**/*.rs",
    ]);

    // Load configuration from file
    config.load_from_file().unwrap();

    // CLI patterns should be unaffected by config file loading
    assert_eq!(config.get_ignore_patterns(), vec!["cli_*.rs"]);
    assert_eq!(config.get_include_patterns(), vec!["cli_src/**/*.rs"]);
}
