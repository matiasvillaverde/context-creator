use clap::Parser;
use code_digest::cli::Config;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_prompt_token_reservation_integration() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join(".code-digest.toml");

    // Create a config file with a specific token limit
    let config_content = r#"
[tokens]
gemini = 10000
"#;

    fs::write(&config_path, config_content).unwrap();

    // Test with a prompt that should reserve tokens
    let mut config = Config::parse_from([
        "code-digest",
        "--prompt",
        "This is a longer test prompt that should use several tokens for testing the reservation system",
        "--tool",
        "gemini",
        "--config",
        config_path.to_str().unwrap(),
    ]);

    config.load_from_file().unwrap();

    // The effective max tokens should be the full config limit
    assert_eq!(config.get_effective_max_tokens(), Some(10000));

    // The effective context tokens should be less due to prompt reservation
    let context_tokens = config.get_effective_context_tokens().unwrap();
    assert!(
        context_tokens < 10000,
        "Context tokens should be less than max due to prompt reservation"
    );
    assert!(
        context_tokens > 8000,
        "Context tokens should still be most of the budget"
    );

    // The difference should account for prompt tokens + safety buffer
    let reserved = 10000 - context_tokens;
    assert!(reserved >= 1000, "Should reserve at least safety buffer");
    assert!(reserved <= 2000, "Should not reserve too much"); // Generous upper bound
}

#[test]
fn test_no_prompt_uses_full_budget() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join(".code-digest.toml");

    // Create a config file with a specific token limit
    let config_content = r#"
[tokens]
gemini = 10000
"#;

    fs::write(&config_path, config_content).unwrap();

    // Test without prompt (using file processing)
    let mut config = Config::parse_from([
        "code-digest",
        "--tool",
        "gemini",
        "--config",
        config_path.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
    ]);

    config.load_from_file().unwrap();

    // Without prompt, effective max and context tokens should be the same
    assert_eq!(config.get_effective_max_tokens(), None); // No auto-limits without prompt
    assert_eq!(config.get_effective_context_tokens(), None);
}

#[test]
fn test_explicit_cli_override_with_prompt_reservation() {
    // Test that explicit CLI token limit is properly reserved for prompts
    let config = Config::parse_from([
        "code-digest",
        "--prompt",
        "Test prompt for explicit override",
        "--max-tokens",
        "5000",
        "--tool",
        "gemini",
    ]);

    // Explicit CLI value
    assert_eq!(config.get_effective_max_tokens(), Some(5000));

    // Context should be less due to prompt reservation
    let context_tokens = config.get_effective_context_tokens().unwrap();
    assert!(context_tokens < 5000);
    assert!(context_tokens > 3500); // Should still be most of the budget
}
