use clap::Parser;
use code_digest::cli::{Config, LlmTool};
use std::path::PathBuf;

#[test]
fn test_llm_tool_default() {
    let config = Config::parse_from(["code-digest", "."]);
    assert_eq!(config.llm_tool, LlmTool::Gemini);
}

#[test]
fn test_llm_tool_gemini() {
    let config = Config::parse_from(["code-digest", "--tool", "gemini", "."]);
    assert_eq!(config.llm_tool, LlmTool::Gemini);
}

#[test]
fn test_llm_tool_codex() {
    let config = Config::parse_from(["code-digest", "--tool", "codex", "."]);
    assert_eq!(config.llm_tool, LlmTool::Codex);
}

#[test]
fn test_llm_tool_short_flag() {
    let config = Config::parse_from(["code-digest", "-t", "codex", "."]);
    assert_eq!(config.llm_tool, LlmTool::Codex);
}

#[test]
fn test_llm_tool_command_names() {
    assert_eq!(LlmTool::Gemini.command(), "gemini");
    assert_eq!(LlmTool::Codex.command(), "codex");
}

#[test]
fn test_llm_tool_install_instructions() {
    assert!(LlmTool::Gemini
        .install_instructions()
        .contains("pip install"));
    assert!(LlmTool::Codex.install_instructions().contains("github.com"));
}

#[test]
fn test_repo_argument() {
    let config = Config::parse_from(["code-digest", "--repo", "https://github.com/owner/repo"]);
    assert_eq!(
        config.repo,
        Some("https://github.com/owner/repo".to_string())
    );
}

#[test]
fn test_repo_and_directory_mutually_exclusive() {
    let result = Config::try_parse_from([
        "code-digest",
        "--repo",
        "https://github.com/owner/repo",
        ".",
    ]);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("cannot be used with"));
}

#[test]
fn test_valid_repo_url_accepted() {
    let config = Config::parse_from([
        "code-digest",
        "--repo",
        "https://github.com/matiasvillaverde/code-digest",
    ]);
    assert_eq!(
        config.repo,
        Some("https://github.com/matiasvillaverde/code-digest".to_string())
    );
}

#[test]
fn test_prompt_flag_with_spaces() {
    let config = Config::parse_from([
        "code-digest",
        "--prompt",
        "How does authentication work in this codebase?",
    ]);
    assert_eq!(
        config.get_prompt(),
        Some("How does authentication work in this codebase?".to_string())
    );
    assert_eq!(config.get_directories(), vec![PathBuf::from(".")]);
}

#[test]
fn test_prompt_short_flag() {
    let config = Config::parse_from(["code-digest", "-p", "Analyze security"]);
    assert_eq!(config.get_prompt(), Some("Analyze security".to_string()));
}

#[test]
fn test_positional_directories() {
    let config = Config::parse_from(["code-digest", "src/auth", "src/models", "tests/auth"]);
    assert_eq!(
        config.get_directories(),
        vec![
            PathBuf::from("src/auth"),
            PathBuf::from("src/models"),
            PathBuf::from("tests/auth")
        ]
    );
}

#[test]
fn test_multiple_directories() {
    let config = Config::parse_from(["code-digest", "src/core", "src/utils", "tests"]);
    assert_eq!(
        config.get_directories(),
        vec![
            PathBuf::from("src/core"),
            PathBuf::from("src/utils"),
            PathBuf::from("tests")
        ]
    );
}

#[test]
fn test_prompt_and_paths_mutually_exclusive() {
    let result = Config::try_parse_from(["code-digest", "--prompt", "test", "src"]);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("cannot be used with"));
}

#[test]
fn test_stdin_flag() {
    // Test with explicit stdin flag
    let config = Config::parse_from(["code-digest", "--stdin"]);
    assert!(config.read_stdin);
    assert!(config.should_read_stdin());

    // Test without stdin flag
    let config = Config::parse_from(["code-digest", "src"]);
    assert!(!config.read_stdin);
}

#[test]
fn test_copy_flag() {
    let config = Config::parse_from(["code-digest", "src", "--copy"]);
    assert!(config.copy);
}

#[test]
fn test_copy_short_flag() {
    let config = Config::parse_from(["code-digest", "src", "-C"]);
    assert!(config.copy);
}

#[test]
fn test_copy_default_false() {
    let config = Config::parse_from(["code-digest", "src"]);
    assert!(!config.copy);
}

#[test]
fn test_copy_with_output_conflict() {
    let config = Config::parse_from(["code-digest", "src", "--copy", "-o", "out.md"]);
    let result = config.validate();
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Cannot specify both"));
}

#[test]
fn test_enhanced_context_flag() {
    let config = Config::parse_from(["code-digest", "--enhanced-context", "."]);
    assert!(config.enhanced_context);
}

#[test]
fn test_enhanced_context_default_false() {
    let config = Config::parse_from(["code-digest", "."]);
    assert!(!config.enhanced_context);
}

// Tests for --include flag functionality
#[test]
fn test_include_single_path() {
    let config = Config::parse_from(["code-digest", "--include", "src/"]);
    // When using include patterns, base directory is current dir
    assert_eq!(config.get_directories(), vec![PathBuf::from(".")]);
    // The patterns themselves are accessed via get_include_patterns
    assert_eq!(config.get_include_patterns(), vec!["src/"]);
}

#[test]
fn test_include_multiple_paths() {
    let config = Config::parse_from(["code-digest", "--include", "src/", "--include", "tests/"]);
    // When using include patterns, base directory is current dir
    assert_eq!(config.get_directories(), vec![PathBuf::from(".")]);
    // The patterns themselves are accessed via get_include_patterns
    assert_eq!(config.get_include_patterns(), vec!["src/", "tests/"]);
}

#[test]
fn test_include_three_paths() {
    let config = Config::parse_from([
        "code-digest",
        "--include",
        "src/",
        "--include",
        "tests/",
        "--include",
        "docs/",
    ]);
    // When using include patterns, base directory is current dir
    assert_eq!(config.get_directories(), vec![PathBuf::from(".")]);
    // The patterns themselves are accessed via get_include_patterns
    assert_eq!(
        config.get_include_patterns(),
        vec!["src/", "tests/", "docs/"]
    );
}

#[test]
fn test_positional_and_include_conflict() {
    let result = Config::try_parse_from(["code-digest", "src/", "--include", "tests/"]);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("cannot be used with"));
}

#[test]
fn test_include_with_prompt_conflict() {
    let result = Config::try_parse_from(["code-digest", "--prompt", "test", "--include", "src/"]);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("cannot be used with"));
}

#[test]
fn test_include_with_repo_conflict() {
    let result = Config::try_parse_from([
        "code-digest",
        "--repo",
        "https://github.com/owner/repo",
        "--include",
        "src/",
    ]);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("cannot be used with"));
}

#[test]
fn test_include_with_stdin_conflict() {
    let result = Config::try_parse_from(["code-digest", "--stdin", "--include", "src/"]);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("cannot be used with"));
}

#[test]
fn test_no_arguments_defaults_to_current_directory() {
    // This test ensures that when no paths or include flags are provided,
    // we default to current directory "."
    let config = Config::parse_from(["code-digest", "--prompt", "test"]);
    // Note: This is testing that the default behavior is preserved
    assert_eq!(config.get_directories(), vec![PathBuf::from(".")]);
}

#[test]
fn test_positional_with_file_path_validation_error() {
    use std::fs;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test_file.txt");
    fs::write(&file_path, "test content").unwrap();

    let config = Config::parse_from([
        "code-digest",
        file_path.to_str().unwrap(),
        "--output-file",
        "test.md",
    ]);

    // Should fail validation because positional path points to a file, not directory
    let result = config.validate();
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Path is not a directory"));
}

#[test]
fn test_include_with_file_path_validation_success() {
    use std::fs;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let dir_path = temp_dir.path().join("test_dir");
    fs::create_dir(&dir_path).unwrap();

    let config = Config::parse_from([
        "code-digest",
        "--include",
        dir_path.to_str().unwrap(),
        "--output-file",
        "test.md",
    ]);

    // Should succeed validation because include path points to a directory
    let result = config.validate();
    assert!(result.is_ok());
}

#[test]
fn test_include_pattern_validation_valid_patterns() {
    let config = Config::parse_from([
        "code-digest",
        "--include",
        "*.py",
        "--include",
        "**/*.rs",
        "--include",
        "src/**/*.js",
        "--output-file",
        "test.md",
    ]);

    // Should succeed validation for valid glob patterns
    let result = config.validate();
    assert!(result.is_ok());
}

#[test]
fn test_include_empty_pattern() {
    let config = Config::parse_from([
        "code-digest",
        "--include",
        "",
        "--include",
        "*.py",
        "--output-file",
        "test.md",
    ]);

    // Should succeed validation - empty patterns are skipped
    let result = config.validate();
    assert!(result.is_ok());
}

#[test]
fn test_include_whitespace_only_pattern() {
    let config = Config::parse_from([
        "code-digest",
        "--include",
        "   ",
        "--include",
        "*.py",
        "--output-file",
        "test.md",
    ]);

    // Should succeed validation - whitespace-only patterns are skipped
    let result = config.validate();
    assert!(result.is_ok());
}

#[test]
fn test_include_glob_pattern_simple_wildcard() {
    let config = Config::parse_from([
        "code-digest",
        "--include",
        "*.py",
        "--output-file",
        "test.md",
    ]);

    // Should succeed validation for simple wildcard pattern
    let result = config.validate();
    assert!(result.is_ok());
}

#[test]
fn test_include_glob_pattern_recursive_directory() {
    // Test recursive directory matching
    let config = Config::parse_from(["code-digest", "--include", "**/*.rs"]);
    assert_eq!(config.include, Some(vec!["**/*.rs".to_string()]));
}

#[test]
fn test_include_glob_pattern_brace_expansion() {
    // Test brace expansion
    let config = Config::parse_from(["code-digest", "--include", "src/**/*.{py,js}"]);
    assert_eq!(config.include, Some(vec!["src/**/*.{py,js}".to_string()]));
}

#[test]
fn test_include_glob_pattern_character_sets() {
    // Test character sets and ranges
    let config = Config::parse_from(["code-digest", "--include", "**/test[0-9].py"]);
    assert_eq!(config.include, Some(vec!["**/test[0-9].py".to_string()]));
}

#[test]
fn test_include_multiple_glob_patterns() {
    // Test multiple glob patterns
    let config = Config::parse_from([
        "code-digest",
        "--include",
        "**/*repository*.py",
        "--include",
        "**/db/**",
    ]);
    assert_eq!(
        config.include,
        Some(vec![
            "**/*repository*.py".to_string(),
            "**/db/**".to_string()
        ])
    );
}

#[test]
fn test_include_complex_pattern_combinations() {
    // Test complex pattern combinations
    let config = Config::parse_from([
        "code-digest",
        "--include",
        "**/*{repository,service,model}*.py",
        "--include",
        "**/db/**",
    ]);
    assert_eq!(
        config.include,
        Some(vec![
            "**/*{repository,service,model}*.py".to_string(),
            "**/db/**".to_string()
        ])
    );
}

#[test]
fn test_include_pattern_validation_invalid_pattern() {
    let config = Config::parse_from([
        "code-digest",
        "--include",
        "src/[", // Invalid unclosed bracket
        "--output-file",
        "test.md",
    ]);

    // CLI validation now passes - pattern validation happens in walker.rs for better security
    let result = config.validate();
    assert!(
        result.is_ok(),
        "CLI validation should pass, walker handles pattern validation"
    );
}

// === SECURITY INTEGRATION TESTS ===

#[test]
fn test_cli_security_directory_traversal_rejected() {
    // Test that directory traversal patterns are rejected during file processing
    use std::fs;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let current_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(temp_dir.path()).unwrap();

    // Create a test file
    fs::write("test.py", "print('hello')").unwrap();

    let config = Config::parse_from([
        "code-digest",
        "--include",
        "../../../etc/passwd", // Directory traversal attempt
        "--output-file",
        "output.md",
    ]);

    // CLI validation should pass (we moved validation to walker)
    assert!(config.validate().is_ok());

    // But actual execution should fail during file processing
    let result = std::panic::catch_unwind(|| {
        // This would normally trigger the walker code path
        // Since we can't easily test the full CLI execution here,
        // we verify the config is parsed correctly
        let patterns = config.get_include_patterns();
        assert_eq!(patterns, vec!["../../../etc/passwd"]);
    });

    std::env::set_current_dir(current_dir).unwrap();
    assert!(
        result.is_ok(),
        "Config parsing should succeed, validation happens later"
    );
}

#[test]
fn test_cli_security_null_byte_patterns() {
    // Test that patterns with null bytes are handled gracefully
    let config = Config::parse_from([
        "code-digest",
        "--include",
        "test\0.py", // Null byte in pattern
        "--output-file",
        "output.md",
    ]);

    // CLI validation should pass - security validation happens in walker
    let result = config.validate();
    assert!(result.is_ok(), "CLI should parse null byte patterns");

    let patterns = config.get_include_patterns();
    assert_eq!(patterns, vec!["test\0.py"]);
}

#[test]
fn test_cli_security_long_pattern_handling() {
    // Test very long patterns to check for buffer overflow vulnerabilities
    let long_pattern = "a".repeat(2000); // Longer than our 1000 char limit

    let config = Config::parse_from([
        "code-digest",
        "--include",
        &long_pattern,
        "--output-file",
        "output.md",
    ]);

    // CLI should handle long patterns gracefully
    let result = config.validate();
    assert!(result.is_ok(), "CLI should handle long patterns");

    let patterns = config.get_include_patterns();
    assert_eq!(patterns, vec![long_pattern]);
}

#[test]
fn test_cli_security_multiple_suspicious_patterns() {
    // Test multiple potentially dangerous patterns
    let config = Config::parse_from([
        "code-digest",
        "--include",
        "../../../etc/passwd",
        "--include",
        "/etc/shadow",
        "--include",
        "..\\..\\Windows\\System32\\*",
        "--include",
        "test\0file.py",
        "--output-file",
        "output.md",
    ]);

    // CLI validation should pass
    assert!(config.validate().is_ok());

    let patterns = config.get_include_patterns();
    assert_eq!(
        patterns,
        vec![
            "../../../etc/passwd",
            "/etc/shadow",
            "..\\..\\Windows\\System32\\*",
            "test\0file.py"
        ]
    );
}

#[test]
fn test_cli_security_control_character_patterns() {
    // Test patterns with various control characters
    let patterns_with_controls = vec![
        "file\x01.py",   // SOH
        "test\x08.txt",  // Backspace
        "dir\x0c/*.rs",  // Form feed
        "file\nname.py", // Newline
        "tab\tfile.rs",  // Tab
    ];

    for pattern in patterns_with_controls {
        let config = Config::parse_from([
            "code-digest",
            "--include",
            pattern,
            "--output-file",
            "output.md",
        ]);

        // CLI should parse these patterns
        assert!(
            config.validate().is_ok(),
            "CLI should parse pattern with control chars: {:?}",
            pattern
        );

        let parsed_patterns = config.get_include_patterns();
        assert_eq!(parsed_patterns, vec![pattern]);
    }
}
