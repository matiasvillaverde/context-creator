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
    assert!(LlmTool::Gemini.install_instructions().contains("pip install"));
    assert!(LlmTool::Codex.install_instructions().contains("github.com"));
}

#[test]
fn test_repo_argument() {
    let config = Config::parse_from(["code-digest", "--repo", "https://github.com/owner/repo"]);
    assert_eq!(config.repo, Some("https://github.com/owner/repo".to_string()));
}

#[test]
fn test_repo_and_directory_mutually_exclusive() {
    let result =
        Config::try_parse_from(["code-digest", "--repo", "https://github.com/owner/repo", "."]);
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
    assert_eq!(config.repo, Some("https://github.com/matiasvillaverde/code-digest".to_string()));
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
        vec![PathBuf::from("src/auth"), PathBuf::from("src/models"), PathBuf::from("tests/auth")]
    );
}

#[test]
fn test_multiple_directories() {
    let config = Config::parse_from(["code-digest", "src/core", "src/utils", "tests"]);
    assert_eq!(
        config.get_directories(),
        vec![PathBuf::from("src/core"), PathBuf::from("src/utils"), PathBuf::from("tests")]
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
    assert!(result.unwrap_err().to_string().contains("Cannot specify both"));
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
