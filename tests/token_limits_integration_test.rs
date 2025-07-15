use code_digest::cli::Config;
use clap::Parser;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_token_limits_integration_with_config_file() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join(".code-digest.toml");

    // Create a config file with token limits
    let config_content = r#"
[defaults]
max_tokens = 150000

[tokens]
gemini = 2500000
codex = 1800000

[[priorities]]
pattern = "src/**/*.rs"
weight = 100.0
"#;

    fs::write(&config_path, config_content).unwrap();

    // Test Gemini with prompt (should use config token limit)
    let mut config = Config::parse_from([
        "code-digest",
        "--prompt",
        "test prompt",
        "--tool",
        "gemini",
        "--config",
        config_path.to_str().unwrap(),
    ]);

    config.load_from_file().unwrap();
    assert_eq!(config.get_effective_max_tokens(), Some(2_500_000));

    // Test Codex with prompt (should use config token limit)
    let mut config = Config::parse_from([
        "code-digest",
        "--prompt",
        "test prompt",
        "--tool",
        "codex",
        "--config",
        config_path.to_str().unwrap(),
    ]);

    config.load_from_file().unwrap();
    assert_eq!(config.get_effective_max_tokens(), Some(1_800_000));
}

#[test]
fn test_token_limits_explicit_override_config() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join(".code-digest.toml");

    // Create a config file with token limits
    let config_content = r#"
[tokens]
gemini = 2500000
codex = 1800000
"#;

    fs::write(&config_path, config_content).unwrap();

    // Explicit max_tokens should override config
    let mut config = Config::parse_from([
        "code-digest",
        "--prompt",
        "test prompt",
        "--tool",
        "gemini",
        "--max-tokens",
        "500000",
        "--config",
        config_path.to_str().unwrap(),
    ]);

    config.load_from_file().unwrap();
    assert_eq!(config.get_effective_max_tokens(), Some(500_000));
}

#[test]
fn test_token_limits_partial_config() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join(".code-digest.toml");

    // Create a config file with only Gemini token limit
    let config_content = r#"
[tokens]
gemini = 3000000
# codex not specified
"#;

    fs::write(&config_path, config_content).unwrap();

    // Test Gemini (should use config)
    let mut config = Config::parse_from([
        "code-digest",
        "--prompt",
        "test prompt",
        "--tool",
        "gemini",
        "--config",
        config_path.to_str().unwrap(),
    ]);

    config.load_from_file().unwrap();
    assert_eq!(config.get_effective_max_tokens(), Some(3_000_000));

    // Test Codex (should use hard-coded default)
    let mut config = Config::parse_from([
        "code-digest",
        "--prompt",
        "test prompt",
        "--tool",
        "codex",
        "--config",
        config_path.to_str().unwrap(),
    ]);

    config.load_from_file().unwrap();
    assert_eq!(config.get_effective_max_tokens(), Some(1_000_000));
}

#[test]
fn test_token_limits_no_prompt_no_limits() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join(".code-digest.toml");

    // Create a config file with token limits
    let config_content = r#"
[tokens]
gemini = 2500000
codex = 1800000
"#;

    fs::write(&config_path, config_content).unwrap();

    // Without prompt, no token limits should be applied
    let mut config = Config::parse_from([
        "code-digest",
        "--tool",
        "gemini",
        "--config",
        config_path.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
    ]);

    config.load_from_file().unwrap();
    assert_eq!(config.get_effective_max_tokens(), None);
}

#[test]
fn test_token_limits_precedence_order() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join(".code-digest.toml");

    // Create a config file with both defaults.max_tokens and tokens section
    let config_content = r#"
[defaults]
max_tokens = 150000

[tokens]
gemini = 2500000
codex = 1800000
"#;

    fs::write(&config_path, config_content).unwrap();

    // Test precedence: explicit > config tokens > defaults.max_tokens
    
    // 1. Explicit should win
    let mut config = Config::parse_from([
        "code-digest",
        "--prompt",
        "test prompt",
        "--tool",
        "gemini",
        "--max-tokens",
        "500000",
        "--config",
        config_path.to_str().unwrap(),
    ]);
    config.load_from_file().unwrap();
    assert_eq!(config.get_effective_max_tokens(), Some(500_000));

    // 2. Config tokens should win over defaults.max_tokens when prompt is present
    let mut config = Config::parse_from([
        "code-digest",
        "--prompt",
        "test prompt",
        "--tool",
        "gemini",
        "--config",
        config_path.to_str().unwrap(),
    ]);
    config.load_from_file().unwrap();
    assert_eq!(config.get_effective_max_tokens(), Some(2_500_000)); // From tokens section, not defaults
}

#[test]
fn test_token_limits_config_file_missing() {
    let temp_dir = TempDir::new().unwrap();
    let non_existent_config = temp_dir.path().join("missing.toml");

    // Should fail gracefully when config file doesn't exist
    let mut config = Config::parse_from([
        "code-digest",
        "--prompt",
        "test prompt",
        "--tool",
        "gemini",
        "--config",
        non_existent_config.to_str().unwrap(),
    ]);

    // Should fail to load config
    assert!(config.load_from_file().is_err());
}

#[test]
fn test_token_limits_malformed_config() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join(".code-digest.toml");

    // Create a malformed config file
    let config_content = r#"
[tokens]
gemini = "not_a_number"
codex = 1800000
"#;

    fs::write(&config_path, config_content).unwrap();

    let mut config = Config::parse_from([
        "code-digest",
        "--prompt",
        "test prompt",
        "--tool",
        "gemini",
        "--config",
        config_path.to_str().unwrap(),
    ]);

    // Should fail to parse the malformed config
    assert!(config.load_from_file().is_err());
}