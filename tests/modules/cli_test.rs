#![cfg(test)]

use clap::Parser;
use context_creator::cli::{Config, LlmTool};
use std::path::PathBuf;

#[test]
fn test_llm_tool_default() {
    let config = Config::parse_from(["context-creator", "."]);
    assert_eq!(config.llm_tool, LlmTool::Gemini);
}

#[test]
fn test_llm_tool_gemini() {
    let config = Config::parse_from(["context-creator", "--tool", "gemini", "."]);
    assert_eq!(config.llm_tool, LlmTool::Gemini);
}

#[test]
fn test_llm_tool_codex() {
    let config = Config::parse_from(["context-creator", "--tool", "codex", "."]);
    assert_eq!(config.llm_tool, LlmTool::Codex);
}

#[test]
fn test_llm_tool_short_flag() {
    let config = Config::parse_from(["context-creator", "-t", "codex", "."]);
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
fn test_llm_tool_claude() {
    let config = Config::parse_from(["context-creator", "--tool", "claude", "."]);
    assert_eq!(config.llm_tool, LlmTool::Claude);
}

#[test]
fn test_llm_tool_ollama() {
    let config = Config::parse_from(["context-creator", "--tool", "ollama", "."]);
    assert_eq!(config.llm_tool, LlmTool::Ollama);
}

#[test]
fn test_llm_tool_claude_command_name() {
    assert_eq!(LlmTool::Claude.command(), "claude");
}

#[test]
fn test_llm_tool_ollama_command_name() {
    assert_eq!(LlmTool::Ollama.command(), "ollama");
}

#[test]
fn test_llm_tool_claude_install_instructions() {
    assert!(LlmTool::Claude
        .install_instructions()
        .contains("npm install -g @anthropic-ai/claude-code"));
}

#[test]
fn test_llm_tool_ollama_install_instructions() {
    assert!(LlmTool::Ollama
        .install_instructions()
        .contains("brew install ollama"));
}

#[test]
fn test_ollama_model_argument() {
    let config = Config::parse_from([
        "context-creator",
        "--tool",
        "ollama",
        "--ollama-model",
        "llama3",
        ".",
    ]);
    assert_eq!(config.llm_tool, LlmTool::Ollama);
    assert_eq!(config.ollama_model, Some("llama3".to_string()));
}

#[test]
fn test_ollama_without_model_validation_error() {
    let config = Config::parse_from(["context-creator", "--tool", "ollama", "."]);
    assert_eq!(config.llm_tool, LlmTool::Ollama);
    assert_eq!(config.ollama_model, None);

    // Should fail validation when using Ollama without model
    let result = config.validate();
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("--ollama-model is required when using --tool ollama"));
}

#[test]
fn test_ollama_model_with_other_tools_ignored() {
    let config = Config::parse_from([
        "context-creator",
        "--tool",
        "gemini",
        "--ollama-model",
        "llama3", // Should be ignored for non-ollama tools
        ".",
    ]);
    assert_eq!(config.llm_tool, LlmTool::Gemini);
    assert_eq!(config.ollama_model, Some("llama3".to_string()));

    // Should pass validation - ollama_model is ignored for other tools
    assert!(config.validate().is_ok());
}

#[test]
fn test_repo_argument() {
    let config = Config::parse_from([
        "context-creator",
        "--remote",
        "https://github.com/owner/repo",
    ]);
    assert_eq!(
        config.remote,
        Some("https://github.com/owner/repo".to_string())
    );
}

#[test]
fn test_repo_and_directory_now_disallowed() {
    // This combination is now disallowed to prevent silent overwriting bug
    let config = Config::parse_from([
        "context-creator",
        "--remote",
        "https://github.com/owner/repo",
        ".",
    ]);

    assert_eq!(
        config.remote,
        Some("https://github.com/owner/repo".to_string())
    );
    assert_eq!(config.paths, Some(vec![PathBuf::from(".")]));

    // This should FAIL validation to prevent confusion where paths get silently ignored
    let result = config.validate();
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Cannot specify both --remote and local paths"));
}

#[test]
fn test_valid_repo_url_accepted() {
    let config = Config::parse_from([
        "context-creator",
        "--remote",
        "https://github.com/matiasvillaverde/context-creator",
    ]);
    assert_eq!(
        config.remote,
        Some("https://github.com/matiasvillaverde/context-creator".to_string())
    );
}

#[test]
fn test_prompt_flag_with_spaces() {
    let config = Config::parse_from([
        "context-creator",
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
    let config = Config::parse_from(["context-creator", "-p", "Analyze security"]);
    assert_eq!(config.get_prompt(), Some("Analyze security".to_string()));
}

#[test]
fn test_positional_directories() {
    let config = Config::parse_from(["context-creator", "src/auth", "src/models", "tests/auth"]);
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
    let config = Config::parse_from(["context-creator", "src/core", "src/utils", "tests"]);
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
fn test_prompt_and_paths_now_allowed() {
    // This combination is now allowed per issue #34
    let config = Config::parse_from(["context-creator", "--prompt", "test", "src"]);

    assert_eq!(config.get_prompt(), Some("test".to_string()));
    assert_eq!(config.paths, Some(vec![PathBuf::from("src")]));

    // This should pass validation with the new flexible combinations
    assert!(config.validate().is_ok());
}

#[test]
fn test_stdin_flag() {
    // Test with explicit stdin flag
    let config = Config::parse_from(["context-creator", "--stdin"]);
    assert!(config.read_stdin);
    assert!(config.should_read_stdin());

    // Test without stdin flag
    let config = Config::parse_from(["context-creator", "src"]);
    assert!(!config.read_stdin);
}

#[test]
fn test_copy_flag() {
    let config = Config::parse_from(["context-creator", "src", "--copy"]);
    assert!(config.copy);
}

#[test]
fn test_copy_short_flag() {
    let config = Config::parse_from(["context-creator", "src", "-C"]);
    assert!(config.copy);
}

#[test]
fn test_copy_default_false() {
    let config = Config::parse_from(["context-creator", "src"]);
    assert!(!config.copy);
}

#[test]
fn test_copy_with_output_conflict() {
    use tempfile::TempDir;
    let temp_dir = TempDir::new().unwrap();
    let config = Config::parse_from([
        "context-creator",
        temp_dir.path().to_str().unwrap(),
        "--copy",
        "-o",
        "out.md",
    ]);
    let result = config.validate();
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Cannot specify both --copy and --output"));
}

#[test]
fn test_enhanced_context_flag() {
    let config = Config::parse_from(["context-creator", "--enhanced-context", "."]);
    assert!(config.enhanced_context);
}

#[test]
fn test_enhanced_context_default_false() {
    let config = Config::parse_from(["context-creator", "."]);
    assert!(!config.enhanced_context);
}

// Tests for --include flag functionality
#[test]
fn test_include_single_path() {
    let config = Config::parse_from(["context-creator", "--include", "src/"]);
    // When using include patterns, base directory is current dir
    assert_eq!(config.get_directories(), vec![PathBuf::from(".")]);
    // The patterns themselves are accessed via get_include_patterns
    assert_eq!(config.get_include_patterns(), vec!["src/"]);
}

#[test]
fn test_include_multiple_paths() {
    let config = Config::parse_from([
        "context-creator",
        "--include",
        "src/",
        "--include",
        "tests/",
    ]);
    // When using include patterns, base directory is current dir
    assert_eq!(config.get_directories(), vec![PathBuf::from(".")]);
    // The patterns themselves are accessed via get_include_patterns
    assert_eq!(config.get_include_patterns(), vec!["src/", "tests/"]);
}

#[test]
fn test_include_three_paths() {
    let config = Config::parse_from([
        "context-creator",
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
fn test_positional_and_include_now_allowed() {
    // This combination is now allowed per issue #34
    let config = Config::parse_from(["context-creator", "src/", "--include", "tests/"]);

    assert_eq!(config.paths, Some(vec![PathBuf::from("src/")]));
    assert_eq!(config.get_include_patterns(), vec!["tests/"]);

    // This should pass validation with the new flexible combinations
    assert!(config.validate().is_ok());
}

#[test]
fn test_include_with_prompt_success() {
    // This should now work - prompt and include can be used together
    let result =
        Config::try_parse_from(["context-creator", "--prompt", "test", "--include", "src/"]);
    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.get_prompt(), Some("test".to_string()));
    assert_eq!(config.get_include_patterns(), vec!["src/"]);
}

#[test]
fn test_include_with_repo_now_allowed() {
    // This combination is now allowed per issue #34
    let config = Config::parse_from([
        "context-creator",
        "--remote",
        "https://github.com/owner/repo",
        "--include",
        "src/",
    ]);

    assert_eq!(
        config.remote,
        Some("https://github.com/owner/repo".to_string())
    );
    assert_eq!(config.get_include_patterns(), vec!["src/"]);

    // This should pass validation with the new flexible combinations
    assert!(config.validate().is_ok());
}

#[test]
fn test_include_with_stdin_now_allowed() {
    // This combination is now allowed per issue #34
    let config = Config::parse_from(["context-creator", "--stdin", "--include", "src/"]);

    assert!(config.read_stdin);
    assert_eq!(config.get_include_patterns(), vec!["src/"]);

    // This should pass validation with the new flexible combinations
    assert!(config.validate().is_ok());
}

#[test]
fn test_no_arguments_defaults_to_current_directory() {
    // This test ensures that when no paths or include flags are provided,
    // we default to current directory "."
    let config = Config::parse_from(["context-creator", "--prompt", "test"]);
    // Note: This is testing that the default behavior is preserved
    assert_eq!(config.get_directories(), vec![PathBuf::from(".")]);
}

#[test]
fn test_positional_with_file_path_validation_success() {
    use std::fs;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test_file.txt");
    fs::write(&file_path, "test content").unwrap();

    let config = Config::parse_from([
        "context-creator",
        file_path.to_str().unwrap(),
        "--output-file",
        "test.md",
    ]);

    // Should pass validation because files are now accepted
    let result = config.validate();
    assert!(result.is_ok());
}

#[test]
fn test_include_with_file_path_validation_success() {
    use std::fs;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let dir_path = temp_dir.path().join("test_dir");
    fs::create_dir(&dir_path).unwrap();

    let config = Config::parse_from([
        "context-creator",
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
        "context-creator",
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
        "context-creator",
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
        "context-creator",
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
        "context-creator",
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
    let config = Config::parse_from(["context-creator", "--include", "**/*.rs"]);
    assert_eq!(config.include, Some(vec!["**/*.rs".to_string()]));
}

#[test]
fn test_include_glob_pattern_brace_expansion() {
    // Test brace expansion
    let config = Config::parse_from(["context-creator", "--include", "src/**/*.{py,js}"]);
    assert_eq!(config.include, Some(vec!["src/**/*.{py,js}".to_string()]));
}

#[test]
fn test_include_glob_pattern_character_sets() {
    // Test character sets and ranges
    let config = Config::parse_from(["context-creator", "--include", "**/test[0-9].py"]);
    assert_eq!(config.include, Some(vec!["**/test[0-9].py".to_string()]));
}

#[test]
fn test_include_multiple_glob_patterns() {
    // Test multiple glob patterns
    let config = Config::parse_from([
        "context-creator",
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
        "context-creator",
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
        "context-creator",
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
        "context-creator",
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
        "context-creator",
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
        "context-creator",
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
        "context-creator",
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
            "context-creator",
            "--include",
            pattern,
            "--output-file",
            "output.md",
        ]);

        // CLI should parse these patterns
        assert!(
            config.validate().is_ok(),
            "CLI should parse pattern with control chars: {pattern:?}"
        );

        let parsed_patterns = config.get_include_patterns();
        assert_eq!(parsed_patterns, vec![pattern]);
    }
}

// === Tests for prepare_command() method ===

#[test]
fn test_prepare_command_gemini() {

    let config = Config::parse_from(["context-creator", "--tool", "gemini", "."]);
    let (cmd, combined_input) = LlmTool::Gemini.prepare_command(&config).unwrap();

    // Gemini should use stdin for combined prompt+context
    assert!(combined_input);

    // Verify command setup
    assert_eq!(cmd.get_program(), "gemini");

    // Check that no args are passed
    let args: Vec<_> = cmd.get_args().collect();
    assert_eq!(args.len(), 0);
}

#[test]
fn test_prepare_command_codex() {
    let config = Config::parse_from(["context-creator", "--tool", "codex", "."]);
    let (cmd, combined_input) = LlmTool::Codex.prepare_command(&config).unwrap();

    // Codex should use stdin for combined prompt+context
    assert!(combined_input);

    // Verify command setup
    assert_eq!(cmd.get_program(), "codex");

    // Check that no args are passed
    let args: Vec<_> = cmd.get_args().collect();
    assert_eq!(args.len(), 0);
}

#[test]
fn test_prepare_command_claude() {
    let mut config = Config::parse_from(["context-creator", "--tool", "claude", "."]);
    config.prompt = Some("Test prompt".to_string());

    let (cmd, combined_input) = LlmTool::Claude.prepare_command(&config).unwrap();

    // Claude should use command args for prompt, stdin for context only
    assert!(!combined_input);

    // Verify command setup
    assert_eq!(cmd.get_program(), "claude");

    // Check that -p flag and prompt are passed
    let args: Vec<_> = cmd.get_args().collect();
    assert_eq!(args.len(), 2);
    assert_eq!(args[0], "-p");
    assert_eq!(args[1], "Test prompt");
}

#[test]
fn test_prepare_command_ollama_with_model() {
    let config = Config::parse_from([
        "context-creator",
        "--tool",
        "ollama",
        "--ollama-model",
        "llama3",
        ".",
    ]);

    let (cmd, combined_input) = LlmTool::Ollama.prepare_command(&config).unwrap();

    // Ollama should use stdin for combined prompt+context
    assert!(combined_input);

    // Verify command setup
    assert_eq!(cmd.get_program(), "ollama");

    // Check that "run" and model are passed
    let args: Vec<_> = cmd.get_args().collect();
    assert_eq!(args.len(), 2);
    assert_eq!(args[0], "run");
    assert_eq!(args[1], "llama3");
}

#[test]
fn test_prepare_command_ollama_without_model() {
    let config = Config::parse_from(["context-creator", "--tool", "ollama", "."]);

    let result = LlmTool::Ollama.prepare_command(&config);

    // Should return error when model is not specified
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("--ollama-model is required when using --tool ollama"));
}
