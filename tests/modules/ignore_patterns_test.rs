#![cfg(test)]

use clap::Parser;
use context_creator::cli::Config;
use std::path::PathBuf;

/// Test that ignore patterns from CLI are properly integrated into WalkOptions
#[test]
fn test_ignore_patterns_cli_integration() {
    let config = Config::parse_from([
        "context-creator",
        "--include",
        "src/**/*.rs",
        "--ignore",
        "target/**",
        "--ignore",
        "**/*_test.rs",
    ]);

    let ignore_patterns = config.get_ignore_patterns();
    assert_eq!(ignore_patterns, vec!["target/**", "**/*_test.rs"]);
}

/// Test that ignore patterns work with various CLI combinations
#[test]
fn test_ignore_patterns_with_prompt() {
    let config = Config::parse_from([
        "context-creator",
        "--prompt",
        "Analyze code",
        "--include",
        "src/**/*.rs",
        "--ignore",
        "node_modules/**",
        "--ignore",
        "*.log",
    ]);

    assert_eq!(config.get_prompt(), Some("Analyze code".to_string()));
    assert_eq!(config.get_include_patterns(), vec!["src/**/*.rs"]);
    assert_eq!(
        config.get_ignore_patterns(),
        vec!["node_modules/**", "*.log"]
    );
}

/// Test that ignore patterns work without include patterns
#[test]
fn test_ignore_patterns_without_include() {
    let config = Config::parse_from([
        "context-creator",
        "--ignore",
        "target/**",
        "--ignore",
        "node_modules/**",
        ".",
    ]);

    assert_eq!(config.get_directories(), vec![PathBuf::from(".")]);
    assert_eq!(config.get_include_patterns(), Vec::<String>::new());
    assert_eq!(
        config.get_ignore_patterns(),
        vec!["target/**", "node_modules/**"]
    );
}

/// Test that empty ignore patterns are handled correctly
#[test]
fn test_ignore_patterns_empty() {
    let config = Config::parse_from([
        "context-creator",
        "--ignore",
        "",
        "--ignore",
        "target/**",
        "--ignore",
        "   ",
        ".",
    ]);

    // Empty and whitespace-only patterns should be included and filtered later
    assert_eq!(config.get_ignore_patterns(), vec!["", "target/**", "   "]);
}

/// Test that ignore patterns work with complex glob patterns
#[test]
fn test_ignore_patterns_complex_globs() {
    let config = Config::parse_from([
        "context-creator",
        "--include",
        "src/**/*.{rs,py}",
        "--ignore",
        "**/*{test,spec}*.{rs,py}",
        "--ignore",
        "**/target/**",
        "--ignore",
        "**/*.{log,tmp,bak}",
    ]);

    assert_eq!(config.get_include_patterns(), vec!["src/**/*.{rs,py}"]);
    assert_eq!(
        config.get_ignore_patterns(),
        vec![
            "**/*{test,spec}*.{rs,py}",
            "**/target/**",
            "**/*.{log,tmp,bak}"
        ]
    );
}

/// Test that ignore patterns work with single file patterns
#[test]
fn test_ignore_patterns_single_files() {
    let config = Config::parse_from([
        "context-creator",
        "--include",
        "**/*.rs",
        "--ignore",
        "main.rs",
        "--ignore",
        "lib.rs",
        "--ignore",
        "**/mod.rs",
    ]);

    assert_eq!(
        config.get_ignore_patterns(),
        vec!["main.rs", "lib.rs", "**/mod.rs"]
    );
}

/// Test that ignore patterns work with directory patterns
#[test]
fn test_ignore_patterns_directories() {
    let config = Config::parse_from([
        "context-creator",
        "--include",
        "**/*.rs",
        "--ignore",
        "target/",
        "--ignore",
        "node_modules/",
        "--ignore",
        "**/test_data/",
    ]);

    assert_eq!(
        config.get_ignore_patterns(),
        vec!["target/", "node_modules/", "**/test_data/"]
    );
}

/// Test that ignore patterns work with extension patterns
#[test]
fn test_ignore_patterns_extensions() {
    let config = Config::parse_from([
        "context-creator",
        "--include",
        "**/*",
        "--ignore",
        "*.log",
        "--ignore",
        "*.tmp",
        "--ignore",
        "**/*.{bak,orig,swp}",
    ]);

    assert_eq!(
        config.get_ignore_patterns(),
        vec!["*.log", "*.tmp", "**/*.{bak,orig,swp}"]
    );
}

/// Test that ignore patterns work with mixed pattern types
#[test]
fn test_ignore_patterns_mixed_types() {
    let config = Config::parse_from([
        "context-creator",
        "--include",
        "src/**/*.rs",
        "--ignore",
        "target/**", // Directory pattern
        "--ignore",
        "*.log", // Extension pattern
        "--ignore",
        "main.rs", // Single file pattern
        "--ignore",
        "**/test_*", // Prefix pattern
        "--ignore",
        "**/*_backup.*", // Suffix pattern
    ]);

    assert_eq!(
        config.get_ignore_patterns(),
        vec![
            "target/**",
            "*.log",
            "main.rs",
            "**/test_*",
            "**/*_backup.*"
        ]
    );
}

/// Test that ignore patterns work with negation patterns
#[test]
fn test_ignore_patterns_with_negation() {
    let config = Config::parse_from([
        "context-creator",
        "--include",
        "**/*.rs",
        "--ignore",
        "target/**",
        "--ignore",
        "!target/debug/important.rs", // Negation pattern
    ]);

    assert_eq!(
        config.get_ignore_patterns(),
        vec!["target/**", "!target/debug/important.rs"]
    );
}

/// Test that ignore patterns work with case sensitivity
#[test]
fn test_ignore_patterns_case_sensitivity() {
    let config = Config::parse_from([
        "context-creator",
        "--include",
        "**/*.rs",
        "--ignore",
        "*.LOG",
        "--ignore",
        "*.Log",
        "--ignore",
        "*.log",
    ]);

    assert_eq!(
        config.get_ignore_patterns(),
        vec!["*.LOG", "*.Log", "*.log"]
    );
}

/// Test that ignore patterns work with special characters
#[test]
fn test_ignore_patterns_special_chars() {
    let config = Config::parse_from([
        "context-creator",
        "--include",
        "**/*.rs",
        "--ignore",
        "file with spaces.txt",
        "--ignore",
        "file-with-dashes.rs",
        "--ignore",
        "file_with_underscores.rs",
        "--ignore",
        "file.with.dots.rs",
    ]);

    assert_eq!(
        config.get_ignore_patterns(),
        vec![
            "file with spaces.txt",
            "file-with-dashes.rs",
            "file_with_underscores.rs",
            "file.with.dots.rs"
        ]
    );
}

/// Test that ignore patterns work with relative paths
#[test]
fn test_ignore_patterns_relative_paths() {
    let config = Config::parse_from([
        "context-creator",
        "--include",
        "**/*.rs",
        "--ignore",
        "./target/**",
        "--ignore",
        "../sibling_project/**",
        "--ignore",
        "../../parent_project/**",
    ]);

    assert_eq!(
        config.get_ignore_patterns(),
        vec![
            "./target/**",
            "../sibling_project/**",
            "../../parent_project/**"
        ]
    );
}

/// Test that ignore patterns work with absolute paths
#[test]
fn test_ignore_patterns_absolute_paths() {
    let config = Config::parse_from([
        "context-creator",
        "--include",
        "**/*.rs",
        "--ignore",
        "/tmp/**",
        "--ignore",
        "/var/log/**",
    ]);

    assert_eq!(config.get_ignore_patterns(), vec!["/tmp/**", "/var/log/**"]);
}

/// Test that ignore patterns work with multiple CLI flag invocations
#[test]
fn test_ignore_patterns_multiple_flags() {
    let config = Config::parse_from([
        "context-creator",
        "--include",
        "**/*.rs",
        "--ignore",
        "target/**",
        "--ignore",
        "node_modules/**",
        "--ignore",
        "*.log",
        "--ignore",
        "*.tmp",
    ]);

    assert_eq!(
        config.get_ignore_patterns(),
        vec!["target/**", "node_modules/**", "*.log", "*.tmp"]
    );
}

/// Test that ignore patterns are accessible via WalkOptions
#[test]
fn test_ignore_patterns_in_walk_options() {
    let config = Config::parse_from([
        "context-creator",
        "--include",
        "**/*.rs",
        "--ignore",
        "target/**",
        "--ignore",
        "*.log",
    ]);

    // Verify that the patterns are accessible from config
    let ignore_patterns = config.get_ignore_patterns();
    assert_eq!(ignore_patterns, vec!["target/**", "*.log"]);

    // Test WalkOptions::from_config() integration
    let walk_options = context_creator::core::walker::WalkOptions::from_config(&config).unwrap();
    assert_eq!(walk_options.ignore_patterns, vec!["target/**", "*.log"]);
    assert_eq!(walk_options.include_patterns, vec!["**/*.rs"]);
}

/// Test that empty and whitespace patterns are filtered out in WalkOptions
#[test]
fn test_ignore_patterns_filter_empty_in_walk_options() {
    let config = Config::parse_from([
        "context-creator",
        "--include",
        "**/*.rs",
        "--ignore",
        "",
        "--ignore",
        "target/**",
        "--ignore",
        "   ",
        "--ignore",
        "*.log",
    ]);

    // Config should contain all patterns including empty ones
    let ignore_patterns = config.get_ignore_patterns();
    assert_eq!(ignore_patterns, vec!["", "target/**", "   ", "*.log"]);

    // WalkOptions should filter out empty/whitespace patterns
    let walk_options = context_creator::core::walker::WalkOptions::from_config(&config).unwrap();
    assert_eq!(walk_options.ignore_patterns, vec!["target/**", "*.log"]);
}

/// Test that ignore patterns work with the default current directory
#[test]
fn test_ignore_patterns_with_default_directory() {
    let config = Config::parse_from([
        "context-creator",
        "--prompt",
        "Analyze code",
        "--ignore",
        "target/**",
        "--ignore",
        "*.log",
    ]);

    // When no paths/include are specified, default to current directory
    assert_eq!(config.get_directories(), vec![PathBuf::from(".")]);
    assert_eq!(config.get_ignore_patterns(), vec!["target/**", "*.log"]);
}

/// Test that ignore patterns work with backward compatibility
#[test]
fn test_ignore_patterns_backward_compatibility() {
    // Test that existing functionality still works when ignore patterns are not used
    let config = Config::parse_from(["context-creator", "--include", "**/*.rs"]);

    assert_eq!(config.get_include_patterns(), vec!["**/*.rs"]);
    assert_eq!(config.get_ignore_patterns(), Vec::<String>::new());
}

/// Test that ignore patterns work with repo argument
#[test]
fn test_ignore_patterns_with_repo() {
    let config = Config::parse_from([
        "context-creator",
        "--remote",
        "https://github.com/owner/repo",
        "--ignore",
        "target/**",
        "--ignore",
        "*.log",
    ]);

    assert_eq!(
        config.remote,
        Some("https://github.com/owner/repo".to_string())
    );
    assert_eq!(config.get_ignore_patterns(), vec!["target/**", "*.log"]);
}

/// Test that ignore patterns work with LLM tool selection
#[test]
fn test_ignore_patterns_with_llm_tool() {
    let config = Config::parse_from([
        "context-creator",
        "--tool",
        "codex",
        "--include",
        "**/*.rs",
        "--ignore",
        "target/**",
    ]);

    assert_eq!(config.llm_tool, context_creator::cli::LlmTool::Codex);
    assert_eq!(config.get_ignore_patterns(), vec!["target/**"]);
}

/// Test that ignore patterns work with output file
#[test]
fn test_ignore_patterns_with_output_file() {
    let config = Config::parse_from([
        "context-creator",
        "--include",
        "**/*.rs",
        "--ignore",
        "target/**",
        "--output-file",
        "output.md",
    ]);

    assert_eq!(config.output_file, Some(PathBuf::from("output.md")));
    assert_eq!(config.get_ignore_patterns(), vec!["target/**"]);
}

/// Test that ignore patterns work with copy flag
#[test]
fn test_ignore_patterns_with_copy_flag() {
    let config = Config::parse_from([
        "context-creator",
        "--include",
        "**/*.rs",
        "--ignore",
        "target/**",
        "--copy",
    ]);

    assert!(config.copy);
    assert_eq!(config.get_ignore_patterns(), vec!["target/**"]);
}

/// Test that ignore patterns work with enhanced context
#[test]
fn test_ignore_patterns_with_enhanced_context() {
    let config = Config::parse_from([
        "context-creator",
        "--include",
        "**/*.rs",
        "--ignore",
        "target/**",
        "--enhanced-context",
    ]);

    assert!(config.enhanced_context);
    assert_eq!(config.get_ignore_patterns(), vec!["target/**"]);
}

/// Test that ignore patterns work with configuration file
#[test]
fn test_ignore_patterns_with_config_file() {
    let config = Config::parse_from([
        "context-creator",
        "--include",
        "**/*.rs",
        "--ignore",
        "target/**",
        "--config",
        "custom-config.toml",
    ]);

    assert_eq!(config.config, Some(PathBuf::from("custom-config.toml")));
    assert_eq!(config.get_ignore_patterns(), vec!["target/**"]);
}

/// Test that ignore patterns work with max tokens
#[test]
fn test_ignore_patterns_with_max_tokens() {
    let config = Config::parse_from([
        "context-creator",
        "--include",
        "**/*.rs",
        "--ignore",
        "target/**",
        "--max-tokens",
        "500000",
    ]);

    assert_eq!(config.max_tokens, Some(500000));
    assert_eq!(config.get_ignore_patterns(), vec!["target/**"]);
}

/// Test that ignore patterns work with quiet flag
#[test]
fn test_ignore_patterns_with_quiet_flag() {
    let config = Config::parse_from([
        "context-creator",
        "--include",
        "**/*.rs",
        "--ignore",
        "target/**",
        "--quiet",
    ]);

    assert!(config.quiet);
    assert_eq!(config.get_ignore_patterns(), vec!["target/**"]);
}

/// Test that ignore patterns work with verbose flag
#[test]
fn test_ignore_patterns_with_verbose_flag() {
    let config = Config::parse_from([
        "context-creator",
        "--include",
        "**/*.rs",
        "--ignore",
        "target/**",
        "--verbose",
    ]);

    assert_eq!(config.verbose, 1);
    assert_eq!(config.get_ignore_patterns(), vec!["target/**"]);
}
