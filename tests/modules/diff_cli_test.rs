#![cfg(test)]

use clap::Parser;
use context_creator::cli::{Commands, Config};

/// Test CLI parsing for diff command
#[test]
fn test_diff_command_basic_parsing() {
    let config = Config::parse_from(["context-creator", "diff", "HEAD~1", "HEAD"]);

    match &config.command {
        Some(Commands::Diff { from, to }) => {
            assert_eq!(from, "HEAD~1");
            assert_eq!(to, "HEAD");
        }
        _ => panic!("Expected Diff command, got {:?}", config.command),
    }
}

#[test]
fn test_diff_command_with_branches() {
    let config = Config::parse_from(["context-creator", "diff", "main", "feature-branch"]);

    match &config.command {
        Some(Commands::Diff { from, to }) => {
            assert_eq!(from, "main");
            assert_eq!(to, "feature-branch");
        }
        _ => panic!("Expected Diff command, got {:?}", config.command),
    }
}

#[test]
fn test_diff_command_with_commit_hashes() {
    let config = Config::parse_from(["context-creator", "diff", "abc123", "def456"]);

    match &config.command {
        Some(Commands::Diff { from, to }) => {
            assert_eq!(from, "abc123");
            assert_eq!(to, "def456");
        }
        _ => panic!("Expected Diff command, got {:?}", config.command),
    }
}

#[test]
fn test_diff_command_with_max_tokens() {
    let config = Config::parse_from([
        "context-creator",
        "--max-tokens",
        "5000",
        "diff",
        "HEAD~1",
        "HEAD",
    ]);

    match &config.command {
        Some(Commands::Diff { from, to }) => {
            assert_eq!(from, "HEAD~1");
            assert_eq!(to, "HEAD");
            assert_eq!(config.max_tokens, Some(5000));
        }
        _ => panic!("Expected Diff command, got {:?}", config.command),
    }
}

#[test]
fn test_diff_command_with_output_file() {
    let config = Config::parse_from([
        "context-creator",
        "--output-file",
        "changes.md",
        "diff",
        "HEAD~1",
        "HEAD",
    ]);

    match &config.command {
        Some(Commands::Diff { from, to }) => {
            assert_eq!(from, "HEAD~1");
            assert_eq!(to, "HEAD");
            assert_eq!(
                config.output_file.as_ref().unwrap().to_str().unwrap(),
                "changes.md"
            );
        }
        _ => panic!("Expected Diff command, got {:?}", config.command),
    }
}
