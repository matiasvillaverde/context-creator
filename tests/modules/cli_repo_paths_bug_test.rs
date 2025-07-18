#![cfg(test)]

use clap::Parser;
use context_creator::cli::Config;
use tempfile::TempDir;

// TEST TO DEMONSTRATE BUG: --repo overwrites positional PATHS without warning

#[test]
fn test_repo_overwrites_paths_bug() {
    // This test demonstrates the bug where --repo silently overwrites PATHS
    // The user provides both a repo URL and local paths, but the paths are ignored

    let temp_dir = TempDir::new().unwrap();
    let local_src = temp_dir.path().join("src");
    let local_tests = temp_dir.path().join("tests");
    std::fs::create_dir(&local_src).unwrap();
    std::fs::create_dir(&local_tests).unwrap();

    let config = Config::parse_from([
        "context-creator",
        "--repo",
        "https://github.com/owner/repo",
        local_src.to_str().unwrap(),
        local_tests.to_str().unwrap(),
    ]);

    // CLI parsing should work
    assert_eq!(
        config.repo,
        Some("https://github.com/owner/repo".to_string())
    );
    assert_eq!(
        config.paths,
        Some(vec![local_src.clone(), local_tests.clone()])
    );

    // FIXED: This should now FAIL validation with a clear error message
    // preventing the silent overwriting bug
    let result = config.validate();
    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("Cannot specify both --repo and local paths"));

    // This test documents the fixed behavior:
    // Validation now fails with clear error instead of silently overwriting paths
}

#[test]
fn test_repo_with_paths_should_fail_validation() {
    // This test shows what SHOULD happen - validation should fail with clear error
    // when both --repo and paths are provided, since they conflict

    let temp_dir = TempDir::new().unwrap();
    let local_src = temp_dir.path().join("src");
    std::fs::create_dir(&local_src).unwrap();

    let config = Config::parse_from([
        "context-creator",
        "--repo",
        "https://github.com/owner/repo",
        local_src.to_str().unwrap(),
    ]);

    // Parsing should work
    assert_eq!(
        config.repo,
        Some("https://github.com/owner/repo".to_string())
    );
    assert_eq!(config.paths, Some(vec![local_src]));

    // FIXED: This should now FAIL validation with a clear error message
    let result = config.validate();
    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("Cannot specify both --repo and local paths"));
}

#[test]
fn test_repo_only_should_work() {
    // Sanity check: --repo by itself should work fine
    let config = Config::parse_from(["context-creator", "--repo", "https://github.com/owner/repo"]);

    assert_eq!(
        config.repo,
        Some("https://github.com/owner/repo".to_string())
    );
    assert_eq!(config.paths, None);

    // This should pass validation
    assert!(config.validate().is_ok());
}

#[test]
fn test_paths_only_should_work() {
    // Sanity check: paths by themselves should work fine
    let temp_dir = TempDir::new().unwrap();
    let local_src = temp_dir.path().join("src");
    std::fs::create_dir(&local_src).unwrap();

    let config = Config::parse_from(["context-creator", local_src.to_str().unwrap()]);

    assert_eq!(config.repo, None);
    assert_eq!(config.paths, Some(vec![local_src]));

    // This should pass validation
    assert!(config.validate().is_ok());
}

#[test]
fn test_repo_with_prompt_and_paths_complex_scenario() {
    // This tests a more complex scenario where user provides repo, prompt, and paths
    let temp_dir = TempDir::new().unwrap();
    let local_src = temp_dir.path().join("src");
    std::fs::create_dir(&local_src).unwrap();

    let config = Config::parse_from([
        "context-creator",
        "--prompt",
        "Compare local and remote code",
        "--repo",
        "https://github.com/owner/repo",
        local_src.to_str().unwrap(),
    ]);

    assert_eq!(
        config.get_prompt(),
        Some("Compare local and remote code".to_string())
    );
    assert_eq!(
        config.repo,
        Some("https://github.com/owner/repo".to_string())
    );
    assert_eq!(config.paths, Some(vec![local_src]));

    // FIXED: This should now FAIL validation with a clear error message
    // about conflicting input sources
    let result = config.validate();
    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("Cannot specify both --repo and local paths"));
}

#[test]
fn test_repo_only_debug() {
    // Debug test to understand why repo-only commands are failing
    let config = Config::parse_from(["context-creator", "--repo", "https://github.com/owner/repo"]);

    println!("DEBUG: config.repo = {:?}", config.repo);
    println!("DEBUG: config.paths = {:?}", config.paths);
    println!("DEBUG: config.include = {:?}", config.include);

    assert_eq!(
        config.repo,
        Some("https://github.com/owner/repo".to_string())
    );
    assert_eq!(config.paths, None);

    // This should pass validation
    let result = config.validate();
    if let Err(e) = &result {
        println!("DEBUG: Validation error: {e}");
    }
    assert!(result.is_ok());
}

#[test]
fn test_repo_with_config_loading() {
    // Debug test to understand config loading behavior
    let mut config =
        Config::parse_from(["context-creator", "--repo", "https://github.com/owner/repo"]);

    println!(
        "DEBUG: Before load_from_file: config.paths = {:?}",
        config.paths
    );
    println!(
        "DEBUG: Before load_from_file: config.repo = {:?}",
        config.repo
    );

    // This mimics what happens in the main application
    config.load_from_file().unwrap();

    println!(
        "DEBUG: After load_from_file: config.paths = {:?}",
        config.paths
    );
    println!(
        "DEBUG: After load_from_file: config.repo = {:?}",
        config.repo
    );

    // This should pass validation
    let result = config.validate();
    if let Err(e) = &result {
        println!("DEBUG: Validation error: {e}");
    }
    assert!(result.is_ok());
}

#[test]
fn test_subprocess_repo_only_issue() {
    // Debug test to understand why the subprocess test is failing
    use std::process::Command;

    let mut cmd = Command::new("cargo");
    cmd.arg("run")
        .arg("--")
        .arg("--repo")
        .arg("https://github.com/fake/repo");

    let output = cmd.output().unwrap();

    println!(
        "DEBUG: Process exit code: {}",
        output.status.code().unwrap_or(-1)
    );
    println!(
        "DEBUG: Process stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    println!(
        "DEBUG: Process stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // The process should fail because we don't have gh/git available, but NOT because of path validation
    let stderr_str = String::from_utf8_lossy(&output.stderr);

    // Check if the error is about path validation (which would be wrong)
    if stderr_str.contains("Cannot specify both --repo and local paths") {
        panic!("Process failed due to path validation error when it should not have: {stderr_str}");
    }
}

#[test]
fn test_binary_vs_cargo_run() {
    // Test both cargo run and Command::cargo_bin to see if they behave differently
    use assert_cmd::Command as AssertCommand;
    use std::process::Command;

    println!("=== Testing cargo run ===");
    let mut cmd = Command::new("cargo");
    cmd.arg("run")
        .arg("--")
        .arg("--repo")
        .arg("https://github.com/fake/repo");

    let output = cmd.output().unwrap();
    println!(
        "cargo run exit code: {}",
        output.status.code().unwrap_or(-1)
    );
    println!(
        "cargo run stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    println!("\n=== Testing Command::cargo_bin ===");
    let mut cmd = AssertCommand::cargo_bin("context-creator").unwrap();
    cmd.arg("--repo").arg("https://github.com/fake/repo");

    let output = cmd.output().unwrap();
    println!(
        "cargo_bin exit code: {}",
        output.status.code().unwrap_or(-1)
    );
    println!(
        "cargo_bin stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Both should fail with remote fetch error, not path validation error
    let stderr_str = String::from_utf8_lossy(&output.stderr);
    if stderr_str.contains("Cannot specify both --repo and local paths") {
        panic!("Binary process failed due to path validation error when it should not have: {stderr_str}");
    }
}
