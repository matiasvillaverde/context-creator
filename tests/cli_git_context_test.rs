#![cfg(test)]

use clap::Parser;
use context_creator::cli::Config;

#[test]
fn test_git_context_flag_parsing() {
    // Test that --git-context flag is parsed correctly
    let args = vec!["context-creator", "--git-context", "."];
    let config = Config::parse_from(args);
    assert!(
        config.git_context,
        "git_context flag should be true when specified"
    );
}

#[test]
fn test_git_context_default_false() {
    // Test that git_context defaults to false
    let args = vec!["context-creator", "."];
    let config = Config::parse_from(args);
    assert!(
        !config.git_context,
        "git_context flag should default to false"
    );
}

#[test]
fn test_git_context_with_enhanced_context() {
    // Test combination with other flags
    let args = vec![
        "context-creator",
        "--git-context",
        "--enhanced-context",
        ".",
    ];
    let config = Config::parse_from(args);
    assert!(config.git_context, "git_context flag should be true");
    assert!(
        config.enhanced_context,
        "enhanced_context flag should be true"
    );
}
