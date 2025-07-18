#![cfg(test)]

use clap::Parser;
use context_creator::cli::Config;
use std::path::PathBuf;
use tempfile::TempDir;

// COMPREHENSIVE EDGE CASE TESTING FOR CLI FLEXIBILITY
// These tests verify that the CLI can handle complex combinations and edge cases
// All tests are written to pass after ArgGroup restrictions are removed

#[test]
fn test_prompt_with_multiple_paths() {
    // Should work: prompt with multiple directory paths
    let temp_dir = TempDir::new().unwrap();
    let auth_dir = temp_dir.path().join("auth");
    let security_dir = temp_dir.path().join("security");
    let core_dir = temp_dir.path().join("core");
    let integration_dir = temp_dir.path().join("integration");
    std::fs::create_dir(&auth_dir).unwrap();
    std::fs::create_dir(&security_dir).unwrap();
    std::fs::create_dir(&core_dir).unwrap();
    std::fs::create_dir(&integration_dir).unwrap();

    let config = Config::parse_from([
        "context-creator",
        "--prompt",
        "Analyze multiple modules",
        auth_dir.to_str().unwrap(),
        security_dir.to_str().unwrap(),
        core_dir.to_str().unwrap(),
        integration_dir.to_str().unwrap(),
    ]);

    assert_eq!(
        config.get_prompt(),
        Some("Analyze multiple modules".to_string())
    );
    assert_eq!(
        config.paths,
        Some(vec![auth_dir, security_dir, core_dir, integration_dir])
    );

    // This should pass validation after we fix restrictions
    assert!(config.validate().is_ok());
}

#[test]
fn test_stdin_with_multiple_paths_and_patterns() {
    // Should work: stdin with paths and filtering patterns
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    let docs_dir = temp_dir.path().join("docs");
    std::fs::create_dir(&src_dir).unwrap();
    std::fs::create_dir(&docs_dir).unwrap();

    let config = Config::parse_from([
        "context-creator",
        "--stdin",
        "--include",
        "**/*.rs",
        "--include",
        "**/*.toml",
        "--ignore",
        "target/**",
        "--ignore",
        "**/*_test.rs",
        src_dir.to_str().unwrap(),
        docs_dir.to_str().unwrap(),
    ]);

    assert!(config.read_stdin);
    assert_eq!(config.get_include_patterns(), vec!["**/*.rs", "**/*.toml"]);
    assert_eq!(
        config.get_ignore_patterns(),
        vec!["target/**", "**/*_test.rs"]
    );
    assert_eq!(config.paths, Some(vec![src_dir, docs_dir]));

    // This should pass validation after we fix restrictions
    assert!(config.validate().is_ok());
}

#[test]
fn test_prompt_with_repo_and_options() {
    // Should work: prompt with repo and additional options
    let config = Config::parse_from([
        "context-creator",
        "--prompt",
        "Security audit of external repo",
        "--repo",
        "https://github.com/owner/repo",
        "--max-tokens",
        "500000",
        "--verbose",
        "--progress",
    ]);

    assert_eq!(
        config.get_prompt(),
        Some("Security audit of external repo".to_string())
    );
    assert_eq!(
        config.repo,
        Some("https://github.com/owner/repo".to_string())
    );
    assert_eq!(config.max_tokens, Some(500000));
    assert!(config.verbose);
    assert!(config.progress);

    // This should pass validation after we fix restrictions
    assert!(config.validate().is_ok());
}

#[test]
fn test_complex_include_exclude_patterns() {
    // Should work: complex pattern combinations
    let config = Config::parse_from([
        "context-creator",
        "--prompt",
        "Find specific patterns",
        "--include",
        "src/**/*.{rs,py,js}",
        "--include",
        "tests/**/test_*.rs",
        "--include",
        "docs/**/*.md",
        "--ignore",
        "target/**",
        "--ignore",
        "node_modules/**",
        "--ignore",
        "**/*.pyc",
        "--ignore",
        ".git/**",
    ]);

    assert_eq!(
        config.get_prompt(),
        Some("Find specific patterns".to_string())
    );
    assert_eq!(
        config.get_include_patterns(),
        vec!["src/**/*.{rs,py,js}", "tests/**/test_*.rs", "docs/**/*.md"]
    );
    assert_eq!(
        config.get_ignore_patterns(),
        vec!["target/**", "node_modules/**", "**/*.pyc", ".git/**"]
    );

    // This should pass validation after we fix restrictions
    assert!(config.validate().is_ok());
}

#[test]
fn test_all_semantic_options_with_prompt() {
    // Should work: all semantic analysis options with prompt
    let config = Config::parse_from([
        "context-creator",
        "--prompt",
        "Deep semantic analysis",
        "--include",
        "src/**/*.rs",
        "--trace-imports",
        "--include-callers",
        "--include-types",
        "--semantic-depth",
        "5",
        "--enhanced-context",
    ]);

    assert_eq!(
        config.get_prompt(),
        Some("Deep semantic analysis".to_string())
    );
    assert_eq!(config.get_include_patterns(), vec!["src/**/*.rs"]);
    assert!(config.trace_imports);
    assert!(config.include_callers);
    assert!(config.include_types);
    assert_eq!(config.semantic_depth, 5);
    assert!(config.enhanced_context);

    // This should pass validation after we fix restrictions
    assert!(config.validate().is_ok());
}

#[test]
fn test_stdin_with_all_options() {
    // Should work: stdin with maximum option flexibility
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir(&src_dir).unwrap();

    let config = Config::parse_from([
        "context-creator",
        "--stdin",
        "--include",
        "**/*.rs",
        "--ignore",
        "target/**",
        "--max-tokens",
        "1000000",
        "--tool",
        "codex",
        "--verbose",
        "--progress",
        "--enhanced-context",
        "--trace-imports",
        "--semantic-depth",
        "3",
        src_dir.to_str().unwrap(),
    ]);

    assert!(config.read_stdin);
    assert_eq!(config.get_include_patterns(), vec!["**/*.rs"]);
    assert_eq!(config.get_ignore_patterns(), vec!["target/**"]);
    assert_eq!(config.max_tokens, Some(1000000));
    assert_eq!(config.llm_tool.command(), "codex");
    assert!(config.verbose);
    assert!(config.progress);
    assert!(config.enhanced_context);
    assert!(config.trace_imports);
    assert_eq!(config.semantic_depth, 3);
    assert_eq!(config.paths, Some(vec![src_dir]));

    // This should pass validation after we fix restrictions
    assert!(config.validate().is_ok());
}

#[test]
fn test_whitespace_prompt_handling() {
    // Edge case: various whitespace in prompts
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir(&src_dir).unwrap();

    let config = Config::parse_from([
        "context-creator",
        "--prompt",
        "  \t  \n  ",
        src_dir.to_str().unwrap(),
    ]);

    assert_eq!(config.get_prompt(), None); // Whitespace-only prompt filtered out
    assert_eq!(config.paths, Some(vec![src_dir]));

    // This should pass validation after we fix restrictions
    assert!(config.validate().is_ok());
}

#[test]
fn test_very_long_prompt_with_paths() {
    // Edge case: very long prompt with paths
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    let tests_dir = temp_dir.path().join("tests");
    std::fs::create_dir(&src_dir).unwrap();
    std::fs::create_dir(&tests_dir).unwrap();

    let long_prompt = "a".repeat(10000);
    let config = Config::parse_from([
        "context-creator",
        "--prompt",
        &long_prompt,
        src_dir.to_str().unwrap(),
        tests_dir.to_str().unwrap(),
    ]);

    assert_eq!(config.get_prompt(), Some(long_prompt));
    assert_eq!(config.paths, Some(vec![src_dir, tests_dir]));

    // This should pass validation after we fix restrictions
    assert!(config.validate().is_ok());
}

#[test]
fn test_unicode_prompt_with_paths() {
    // Edge case: unicode characters in prompt
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir(&src_dir).unwrap();

    let unicode_prompt = "分析这个代码库 🚀 Analyze this codebase";
    let config = Config::parse_from([
        "context-creator",
        "--prompt",
        unicode_prompt,
        src_dir.to_str().unwrap(),
    ]);

    assert_eq!(config.get_prompt(), Some(unicode_prompt.to_string()));
    assert_eq!(config.paths, Some(vec![src_dir]));

    // This should pass validation after we fix restrictions
    assert!(config.validate().is_ok());
}

#[test]
fn test_many_include_patterns_with_prompt() {
    // Edge case: many include patterns
    let config = Config::parse_from([
        "context-creator",
        "--prompt",
        "Find all file types",
        "--include",
        "**/*.rs",
        "--include",
        "**/*.py",
        "--include",
        "**/*.js",
        "--include",
        "**/*.ts",
        "--include",
        "**/*.go",
        "--include",
        "**/*.java",
        "--include",
        "**/*.cpp",
        "--include",
        "**/*.c",
        "--include",
        "**/*.h",
        "--include",
        "**/*.hpp",
        "--include",
        "**/*.toml",
        "--include",
        "**/*.json",
        "--include",
        "**/*.yaml",
        "--include",
        "**/*.yml",
        "--include",
        "**/*.md",
    ]);

    assert_eq!(config.get_prompt(), Some("Find all file types".to_string()));
    assert_eq!(config.get_include_patterns().len(), 15);
    assert!(config
        .get_include_patterns()
        .contains(&"**/*.rs".to_string()));
    assert!(config
        .get_include_patterns()
        .contains(&"**/*.md".to_string()));

    // This should pass validation after we fix restrictions
    assert!(config.validate().is_ok());
}

#[test]
fn test_conflicting_include_ignore_patterns() {
    // Edge case: conflicting include/ignore patterns
    let config = Config::parse_from([
        "context-creator",
        "--prompt",
        "Test conflicting patterns",
        "--include",
        "src/**/*.rs",
        "--ignore",
        "src/**/*.rs", // Same pattern in both include and ignore
    ]);

    assert_eq!(
        config.get_prompt(),
        Some("Test conflicting patterns".to_string())
    );
    assert_eq!(config.get_include_patterns(), vec!["src/**/*.rs"]);
    assert_eq!(config.get_ignore_patterns(), vec!["src/**/*.rs"]);

    // This should pass validation after we fix restrictions (walker handles pattern conflicts)
    assert!(config.validate().is_ok());
}

// EDGE CASES THAT SHOULD STILL FAIL (legitimate restrictions)

#[test]
fn test_prompt_with_output_file_should_fail() {
    // Should fail: can't send to LLM and write to file simultaneously
    let config = Config::parse_from([
        "context-creator",
        "--prompt",
        "This should fail",
        "--output-file",
        "output.md",
        "src/",
    ]);

    assert_eq!(config.get_prompt(), Some("This should fail".to_string()));
    assert_eq!(config.output_file, Some(PathBuf::from("output.md")));

    // This should FAIL validation (legitimate restriction)
    assert!(config.validate().is_err());
}

#[test]
fn test_copy_with_output_file_should_fail() {
    // Should fail: can't copy to clipboard and write to file simultaneously
    let config = Config::parse_from([
        "context-creator",
        "--copy",
        "--output-file",
        "output.md",
        "src/",
    ]);

    assert!(config.copy);
    assert_eq!(config.output_file, Some(PathBuf::from("output.md")));

    // This should FAIL validation (legitimate restriction)
    assert!(config.validate().is_err());
}

#[test]
fn test_no_input_source_should_fail() {
    // Should fail: no input source provided
    let config = Config::parse_from(["context-creator", "--max-tokens", "100000", "--verbose"]);

    assert_eq!(config.get_prompt(), None);
    assert_eq!(config.paths, None);
    assert_eq!(config.include, None);
    assert_eq!(config.repo, None);
    assert!(!config.read_stdin);

    // This should FAIL validation (no input source)
    assert!(config.validate().is_err());
}

// BACKWARD COMPATIBILITY VERIFICATION

#[test]
fn test_existing_usage_patterns_still_work() {
    // Verify all existing usage patterns continue to work

    // Pattern 1: Just paths
    let config1 = Config::parse_from(["context-creator", "src/"]);
    assert!(config1.validate().is_ok());

    // Pattern 2: Just prompt
    let config2 = Config::parse_from(["context-creator", "--prompt", "Analyze"]);
    assert!(config2.validate().is_ok());

    // Pattern 3: Just include patterns
    let config3 = Config::parse_from(["context-creator", "--include", "**/*.rs"]);
    assert!(config3.validate().is_ok());

    // Pattern 4: Just repo
    let config4 =
        Config::parse_from(["context-creator", "--repo", "https://github.com/owner/repo"]);
    assert!(config4.validate().is_ok());

    // Pattern 5: Prompt with include patterns (already supported)
    let config5 = Config::parse_from([
        "context-creator",
        "--prompt",
        "Test",
        "--include",
        "**/*.rs",
    ]);
    assert!(config5.validate().is_ok());
}

// INTEGRATION TESTS WITH REAL FILE OPERATIONS

#[test]
fn test_prompt_with_existing_directories() {
    // Should work: prompt with real existing directories
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    let tests_dir = temp_dir.path().join("tests");
    std::fs::create_dir(&src_dir).unwrap();
    std::fs::create_dir(&tests_dir).unwrap();

    let config = Config::parse_from([
        "context-creator",
        "--prompt",
        "Analyze real directories",
        src_dir.to_str().unwrap(),
        tests_dir.to_str().unwrap(),
    ]);

    assert_eq!(
        config.get_prompt(),
        Some("Analyze real directories".to_string())
    );
    assert_eq!(config.paths, Some(vec![src_dir.clone(), tests_dir.clone()]));

    // This should pass validation after we fix restrictions
    assert!(config.validate().is_ok());
}

#[test]
fn test_stdin_with_nonexistent_directories_should_fail() {
    // Should fail: stdin with non-existent directories
    let config = Config::parse_from([
        "context-creator",
        "--stdin",
        "/nonexistent/directory",
        "/another/nonexistent/directory",
    ]);

    assert!(config.read_stdin);
    assert_eq!(
        config.paths,
        Some(vec![
            PathBuf::from("/nonexistent/directory"),
            PathBuf::from("/another/nonexistent/directory")
        ])
    );

    // This should FAIL validation (directories don't exist)
    assert!(config.validate().is_err());
}

#[test]
fn test_prompt_with_file_instead_of_directory_should_fail() {
    // Should fail: prompt with file instead of directory
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("file.txt");
    std::fs::write(&file_path, "test content").unwrap();

    let config = Config::parse_from([
        "context-creator",
        "--prompt",
        "This should fail",
        file_path.to_str().unwrap(),
    ]);

    assert_eq!(config.get_prompt(), Some("This should fail".to_string()));
    assert_eq!(config.paths, Some(vec![file_path]));

    // This should FAIL validation (file is not a directory)
    assert!(config.validate().is_err());
}

// PERFORMANCE AND STRESS TESTS

#[test]
fn test_maximum_command_line_length() {
    // Edge case: very long command line with many options
    let mut args = vec![
        "context-creator",
        "--prompt",
        "Test maximum command line length",
        "--max-tokens",
        "1000000",
        "--tool",
        "gemini",
        "--verbose",
        "--progress",
        "--enhanced-context",
        "--trace-imports",
        "--include-callers",
        "--include-types",
        "--semantic-depth",
        "5",
    ];

    // Add many include patterns
    for i in 0..100 {
        args.push("--include");
        args.push(Box::leak(format!("pattern{i}/**/*.rs").into_boxed_str()));
    }

    // Add many ignore patterns
    for i in 0..100 {
        args.push("--ignore");
        args.push(Box::leak(format!("ignore{i}/**").into_boxed_str()));
    }

    let config = Config::parse_from(args);

    assert_eq!(
        config.get_prompt(),
        Some("Test maximum command line length".to_string())
    );
    assert_eq!(config.get_include_patterns().len(), 100);
    assert_eq!(config.get_ignore_patterns().len(), 100);

    // This should pass validation after we fix restrictions
    assert!(config.validate().is_ok());
}

#[test]
fn test_zero_length_patterns() {
    // Edge case: zero-length patterns
    let config = Config::parse_from([
        "context-creator",
        "--prompt",
        "Test empty patterns",
        "--include",
        "",
        "--ignore",
        "",
    ]);

    assert_eq!(config.get_prompt(), Some("Test empty patterns".to_string()));
    assert_eq!(config.get_include_patterns(), vec![""]);
    assert_eq!(config.get_ignore_patterns(), vec![""]);

    // This should pass validation after we fix restrictions (walker handles empty patterns)
    assert!(config.validate().is_ok());
}
