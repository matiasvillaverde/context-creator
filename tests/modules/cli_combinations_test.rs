#![cfg(test)]

use clap::Parser;
use context_creator::cli::Config;
use std::path::PathBuf;
use tempfile::TempDir;

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
fn test_prompt_and_paths_now_allowed() {
    // This should now work - prompt and paths are now allowed together
    let config = Config::parse_from(["context-creator", "--prompt", "Analyze", "src/"]);

    assert_eq!(config.get_prompt(), Some("Analyze".to_string()));
    assert_eq!(config.paths, Some(vec![PathBuf::from("src/")]));

    // This should pass validation after we fix restrictions
    assert!(config.validate().is_ok());
}

#[test]
fn test_include_and_paths_now_allowed() {
    // This should now work - include and paths are now allowed together
    let config = Config::parse_from(["context-creator", "--include", "src/**", "src/"]);

    assert_eq!(config.get_include_patterns(), vec!["src/**"]);
    assert_eq!(config.paths, Some(vec![PathBuf::from("src/")]));

    // This should pass validation after we fix restrictions
    assert!(config.validate().is_ok());
}

#[test]
fn test_prompt_and_repo_now_allowed() {
    // This should now work - prompt and repo are now allowed together
    let config = Config::parse_from([
        "context-creator",
        "--prompt",
        "Analyze",
        "--repo",
        "https://github.com/owner/repo",
    ]);

    assert_eq!(config.get_prompt(), Some("Analyze".to_string()));
    assert_eq!(
        config.repo,
        Some("https://github.com/owner/repo".to_string())
    );

    // This should pass validation after we fix restrictions
    assert!(config.validate().is_ok());
}

// NEW FLEXIBLE COMBINATION TESTS - These should work after fixing ArgGroup restrictions

#[test]
fn test_prompt_with_paths() {
    // Should work: process specific directories with a prompt
    let temp_dir = TempDir::new().unwrap();
    let auth_dir = temp_dir.path().join("auth");
    let security_dir = temp_dir.path().join("security");
    std::fs::create_dir(&auth_dir).unwrap();
    std::fs::create_dir(&security_dir).unwrap();

    let config = Config::parse_from([
        "context-creator",
        "--prompt",
        "Analyze security",
        auth_dir.to_str().unwrap(),
        security_dir.to_str().unwrap(),
    ]);

    assert_eq!(config.get_prompt(), Some("Analyze security".to_string()));
    assert_eq!(
        config.paths,
        Some(vec![auth_dir.clone(), security_dir.clone()])
    );
    assert_eq!(config.get_directories(), vec![auth_dir, security_dir]);

    // This should pass validation after we fix restrictions
    assert!(config.validate().is_ok());
}

#[test]
fn test_stdin_with_paths() {
    // Should work: read prompt from stdin, process specific paths
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    let tests_dir = temp_dir.path().join("tests");
    std::fs::create_dir(&src_dir).unwrap();
    std::fs::create_dir(&tests_dir).unwrap();

    let config = Config::parse_from([
        "context-creator",
        "--stdin",
        src_dir.to_str().unwrap(),
        tests_dir.to_str().unwrap(),
    ]);

    assert!(config.read_stdin);
    assert_eq!(config.paths, Some(vec![src_dir.clone(), tests_dir.clone()]));
    assert_eq!(config.get_directories(), vec![src_dir, tests_dir]);

    // This should pass validation after we fix restrictions
    assert!(config.validate().is_ok());
}

#[test]
fn test_prompt_with_repo() {
    // Should work: analyze remote repo with specific prompt
    let config = Config::parse_from([
        "context-creator",
        "--prompt",
        "Find bugs",
        "--repo",
        "https://github.com/owner/repo",
    ]);

    assert_eq!(config.get_prompt(), Some("Find bugs".to_string()));
    assert_eq!(
        config.repo,
        Some("https://github.com/owner/repo".to_string())
    );

    // This should pass validation after we fix restrictions
    assert!(config.validate().is_ok());
}

#[test]
fn test_stdin_with_include_patterns() {
    // Should work: stdin prompt with pattern filtering
    let config = Config::parse_from([
        "context-creator",
        "--stdin",
        "--include",
        "**/*.py",
        "--ignore",
        "tests/**",
    ]);

    assert!(config.read_stdin);
    assert_eq!(config.get_include_patterns(), vec!["**/*.py"]);
    assert_eq!(config.get_ignore_patterns(), vec!["tests/**"]);

    // This should pass validation after we fix restrictions
    assert!(config.validate().is_ok());
}

#[test]
fn test_include_with_repo() {
    // Should work: include patterns with repo (for future enhancement)
    let config = Config::parse_from([
        "context-creator",
        "--include",
        "src/**/*.js",
        "--repo",
        "https://github.com/owner/repo",
    ]);

    assert_eq!(config.get_include_patterns(), vec!["src/**/*.js"]);
    assert_eq!(
        config.repo,
        Some("https://github.com/owner/repo".to_string())
    );

    // This should pass validation after we fix restrictions
    assert!(config.validate().is_ok());
}

#[test]
fn test_all_options_combined() {
    // Should work: maximum flexibility like repomix
    let config = Config::parse_from([
        "context-creator",
        "--prompt",
        "Security audit",
        "--include",
        "src/**/*.rs",
        "--ignore",
        "target/**",
        "--output-file",
        "analysis.md",
    ]);

    assert_eq!(config.get_prompt(), Some("Security audit".to_string()));
    assert_eq!(config.get_include_patterns(), vec!["src/**/*.rs"]);
    assert_eq!(config.get_ignore_patterns(), vec!["target/**"]);
    assert_eq!(config.output_file, Some(PathBuf::from("analysis.md")));

    // This should FAIL validation because prompt + output_file is legitimately restricted
    assert!(config.validate().is_err());
}

#[test]
fn test_multiple_input_sources() {
    // Should work: process both local paths and patterns
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("local_src");
    let tests_dir = temp_dir.path().join("local_tests");
    std::fs::create_dir(&src_dir).unwrap();
    std::fs::create_dir(&tests_dir).unwrap();

    let config = Config::parse_from([
        "context-creator",
        "--include",
        "external/**/*.js",
        src_dir.to_str().unwrap(),
        tests_dir.to_str().unwrap(),
    ]);

    assert_eq!(config.get_include_patterns(), vec!["external/**/*.js"]);
    assert_eq!(config.paths, Some(vec![src_dir, tests_dir]));

    // This should now work with flexible combinations
    assert!(config.validate().is_ok());
}

#[test]
fn test_empty_prompt_with_paths() {
    // Should work: empty prompt should be ignored, paths should work
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir(&src_dir).unwrap();

    let config = Config::parse_from(["context-creator", "--prompt", "", src_dir.to_str().unwrap()]);

    assert_eq!(config.get_prompt(), None); // Empty prompt filtered out
    assert_eq!(config.paths, Some(vec![src_dir]));

    // This should pass validation after we fix restrictions
    assert!(config.validate().is_ok());
}
