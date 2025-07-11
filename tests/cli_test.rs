use clap::Parser;
use code_digest::cli::{Config, LlmTool};

#[test]
fn test_llm_tool_default() {
    let config = Config::parse_from(["code-digest"]);
    assert_eq!(config.llm_tool, LlmTool::GeminiCli);
}

#[test]
fn test_llm_tool_gemini() {
    let config = Config::parse_from(["code-digest", "--tool", "gemini-cli"]);
    assert_eq!(config.llm_tool, LlmTool::GeminiCli);
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
    assert_eq!(LlmTool::GeminiCli.command(), "gemini-cli");
    assert_eq!(LlmTool::Codex.command(), "codex");
}

#[test]
fn test_llm_tool_install_instructions() {
    assert!(LlmTool::GeminiCli.install_instructions().contains("pip install"));
    assert!(LlmTool::Codex.install_instructions().contains("github.com"));
}
