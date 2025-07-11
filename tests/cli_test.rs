use clap::Parser;
use code_digest::cli::{Config, LlmTool};

#[test]
fn test_llm_tool_default() {
    let config = Config::parse_from(["code-digest"]);
    assert_eq!(config.llm_tool, LlmTool::Gemini);
}

#[test]
fn test_llm_tool_gemini() {
    let config = Config::parse_from(["code-digest", "--tool", "gemini"]);
    assert_eq!(config.llm_tool, LlmTool::Gemini);
}

#[test]
fn test_llm_tool_codex() {
    let config = Config::parse_from(["code-digest", "--tool", "codex"]);
    assert_eq!(config.llm_tool, LlmTool::Codex);
}

#[test]
fn test_llm_tool_short_flag() {
    let config = Config::parse_from(["code-digest", "-t", "codex"]);
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
    let result = Config::try_parse_from([
        "code-digest",
        "--repo",
        "https://github.com/owner/repo",
        "-d",
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
    assert_eq!(config.repo, Some("https://github.com/matiasvillaverde/code-digest".to_string()));
}
