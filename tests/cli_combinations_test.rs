use clap::Parser;
use context_creator::cli::Config;
use std::path::PathBuf;

#[test]
fn test_prompt_with_include_patterns() {
    // This should work after we fix the ArgGroup
    let config = Config::parse_from([
        "context-creator",
        "--prompt",
        "Analyze authentication",
        "--include",
        "src/auth/**",
        "--include",
        "tests/auth/**",
    ]);

    assert_eq!(
        config.get_prompt(),
        Some("Analyze authentication".to_string())
    );
    assert_eq!(
        config.get_include_patterns(),
        vec!["src/auth/**", "tests/auth/**"]
    );
    assert_eq!(config.get_directories(), vec![PathBuf::from(".")]);
}

#[test]
fn test_prompt_with_ignore_patterns() {
    // This should work after we add --ignore flag
    let config = Config::parse_from([
        "context-creator",
        "--prompt",
        "Security review",
        "--include",
        "src/security/**",
        "--ignore",
        "**/*_test.rs",
    ]);

    assert_eq!(config.get_prompt(), Some("Security review".to_string()));
    assert_eq!(config.get_include_patterns(), vec!["src/security/**"]);
    assert_eq!(config.get_ignore_patterns(), vec!["**/*_test.rs"]);
}

#[test]
fn test_complex_pattern_combinations() {
    // Test multiple include and ignore patterns
    let config = Config::parse_from([
        "context-creator",
        "--prompt",
        "Review core functionality",
        "--include",
        "src/core/**",
        "--include",
        "src/utils/**",
        "--ignore",
        "node_modules/**",
        "--ignore",
        "target/**",
    ]);

    assert_eq!(
        config.get_prompt(),
        Some("Review core functionality".to_string())
    );
    assert_eq!(
        config.get_include_patterns(),
        vec!["src/core/**", "src/utils/**"]
    );
    assert_eq!(
        config.get_ignore_patterns(),
        vec!["node_modules/**", "target/**"]
    );
}

#[test]
fn test_ignore_without_prompt() {
    // Test that ignore works without prompt too
    let config = Config::parse_from([
        "context-creator",
        "--include",
        "**/*.rs",
        "--ignore",
        "target/**",
    ]);

    assert_eq!(config.get_prompt(), None);
    assert_eq!(config.get_include_patterns(), vec!["**/*.rs"]);
    assert_eq!(config.get_ignore_patterns(), vec!["target/**"]);
}

#[test]
fn test_backward_compatibility_paths() {
    // Ensure existing path arguments still work
    let config = Config::parse_from(["context-creator", "src/", "tests/"]);

    assert_eq!(config.get_prompt(), None);
    assert_eq!(
        config.get_directories(),
        vec![PathBuf::from("src/"), PathBuf::from("tests/")]
    );
    assert_eq!(config.get_include_patterns(), Vec::<String>::new());
    assert_eq!(config.get_ignore_patterns(), Vec::<String>::new());
}

#[test]
fn test_backward_compatibility_prompt_only() {
    // Ensure existing prompt-only usage still works
    let config = Config::parse_from(["context-creator", "--prompt", "Analyze this code"]);

    assert_eq!(config.get_prompt(), Some("Analyze this code".to_string()));
    assert_eq!(config.get_directories(), vec![PathBuf::from(".")]);
    assert_eq!(config.get_include_patterns(), Vec::<String>::new());
    assert_eq!(config.get_ignore_patterns(), Vec::<String>::new());
}

#[test]
fn test_backward_compatibility_include_only() {
    // Ensure existing include-only usage still works
    let config = Config::parse_from([
        "context-creator",
        "--include",
        "src/**/*.rs",
        "--include",
        "tests/**/*.rs",
    ]);

    assert_eq!(config.get_prompt(), None);
    assert_eq!(
        config.get_include_patterns(),
        vec!["src/**/*.rs", "tests/**/*.rs"]
    );
    assert_eq!(config.get_ignore_patterns(), Vec::<String>::new());
}

#[test]
fn test_mutually_exclusive_prompt_and_paths() {
    // This should still fail - prompt and paths are mutually exclusive
    let result = Config::try_parse_from(["context-creator", "--prompt", "Analyze", "src/"]);

    // Either parsing fails or validation fails
    match result {
        Err(_) => {} // Parsing failed - that's fine
        Ok(config) => {
            // Parsing succeeded, so validation should fail
            assert!(config.validate().is_err());
        }
    }
}

#[test]
fn test_mutually_exclusive_include_and_paths() {
    // This should still fail - include and paths are mutually exclusive
    let result = Config::try_parse_from(["context-creator", "--include", "src/**", "src/"]);

    assert!(result.is_err());
}

#[test]
fn test_mutually_exclusive_prompt_and_repo() {
    // This should still fail - prompt and repo are mutually exclusive
    let result = Config::try_parse_from([
        "context-creator",
        "--prompt",
        "Analyze",
        "--repo",
        "https://github.com/owner/repo",
    ]);

    // Either parsing fails or validation fails
    match result {
        Err(_) => {} // Parsing failed - that's fine
        Ok(config) => {
            // Parsing succeeded, so validation should fail
            assert!(config.validate().is_err());
        }
    }
}
