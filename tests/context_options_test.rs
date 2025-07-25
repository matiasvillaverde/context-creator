#![cfg(test)]

use context_creator::cli::Config;
use context_creator::core::context_builder::ContextOptions;
use tempfile::TempDir;

#[test]
fn test_context_options_includes_git_context() {
    let temp_dir = TempDir::new().unwrap();
    let config = Config {
        paths: Some(vec![temp_dir.path().to_path_buf()]),
        git_context: true,
        ..Config::default()
    };

    let options = ContextOptions::from_config(&config).unwrap();
    assert!(
        options.git_context,
        "ContextOptions should include git_context flag"
    );
}

#[test]
fn test_context_options_git_context_default_false() {
    let temp_dir = TempDir::new().unwrap();
    let config = Config {
        paths: Some(vec![temp_dir.path().to_path_buf()]),
        ..Config::default()
    };

    let options = ContextOptions::from_config(&config).unwrap();
    assert!(!options.git_context, "git_context should default to false");
}

#[test]
fn test_context_options_git_context_with_enhanced_context() {
    let temp_dir = TempDir::new().unwrap();
    let config = Config {
        paths: Some(vec![temp_dir.path().to_path_buf()]),
        git_context: true,
        enhanced_context: true,
        ..Config::default()
    };

    let options = ContextOptions::from_config(&config).unwrap();
    assert!(options.git_context, "git_context should be true");
    assert!(options.enhanced_context, "enhanced_context should be true");
}
