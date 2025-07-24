#![cfg(test)]

use std::fs;
use std::process::Command;
use tempfile::TempDir;

/// Helper function to set up a git repository with commits for testing
fn setup_git_repo_with_history() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path();

    // Initialize git repository
    Command::new("git")
        .arg("init")
        .current_dir(repo_path)
        .status()
        .expect("Failed to init git repo");

    // Configure git for testing
    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(repo_path)
        .status()
        .expect("Failed to configure git user name");

    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(repo_path)
        .status()
        .expect("Failed to configure git user email");

    // Create first commit
    fs::write(repo_path.join("file1.txt"), "initial content\n").unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo_path)
        .status()
        .expect("Failed to git add");

    Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(repo_path)
        .status()
        .expect("Failed to create first commit");

    // Create second commit with changes
    fs::write(
        repo_path.join("file1.txt"),
        "initial content\nmodified line\n",
    )
    .unwrap();
    fs::write(repo_path.join("file2.txt"), "new file content\n").unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo_path)
        .status()
        .expect("Failed to git add second commit");

    Command::new("git")
        .args(["commit", "-m", "Second commit"])
        .current_dir(repo_path)
        .status()
        .expect("Failed to create second commit");

    temp_dir
}

/// Test git utility: check if directory is a git repository
#[test]
fn test_is_git_repository_true() {
    let repo = setup_git_repo_with_history();

    // This test will fail until we implement the git utilities
    let result = context_creator::utils::git::is_git_repository(repo.path());
    assert!(result, "Should detect git repository");
}

#[test]
fn test_is_git_repository_false() {
    let temp_dir = TempDir::new().unwrap();

    let result = context_creator::utils::git::is_git_repository(temp_dir.path());
    assert!(
        !result,
        "Should not detect non-git directory as git repository"
    );
}

/// Test git utility: get changed files between references
#[test]
fn test_get_changed_files_valid_refs() {
    let repo = setup_git_repo_with_history();

    let files = context_creator::utils::git::get_changed_files(repo.path(), "HEAD~1", "HEAD")
        .expect("Should successfully get changed files");

    assert!(!files.is_empty(), "Should find changed files");
    assert!(
        files.iter().any(|f| f.file_name().unwrap() == "file1.txt"),
        "Should include modified file1.txt"
    );
    assert!(
        files.iter().any(|f| f.file_name().unwrap() == "file2.txt"),
        "Should include new file2.txt"
    );
}

#[test]
fn test_get_changed_files_identical_refs() {
    let repo = setup_git_repo_with_history();

    let files = context_creator::utils::git::get_changed_files(repo.path(), "HEAD", "HEAD")
        .expect("Should handle identical refs");

    assert!(
        files.is_empty(),
        "Should return empty list for identical refs"
    );
}

#[test]
fn test_get_changed_files_invalid_ref() {
    let repo = setup_git_repo_with_history();

    let result =
        context_creator::utils::git::get_changed_files(repo.path(), "invalid-ref-12345", "HEAD");

    assert!(result.is_err(), "Should fail with invalid git reference");
}

#[test]
fn test_get_changed_files_not_git_repo() {
    let temp_dir = TempDir::new().unwrap();

    let result = context_creator::utils::git::get_changed_files(temp_dir.path(), "HEAD~1", "HEAD");

    assert!(result.is_err(), "Should fail when not in git repository");
}

/// Test git utility: get diff statistics
#[test]
fn test_get_diff_stats_valid_refs() {
    let repo = setup_git_repo_with_history();

    let stats = context_creator::utils::git::get_diff_stats(repo.path(), "HEAD~1", "HEAD")
        .expect("Should get diff statistics");

    assert!(stats.files_changed > 0, "Should report files changed");
    assert!(stats.insertions > 0, "Should report line insertions");
}

#[test]
fn test_get_diff_stats_identical_refs() {
    let repo = setup_git_repo_with_history();

    let stats = context_creator::utils::git::get_diff_stats(repo.path(), "HEAD", "HEAD")
        .expect("Should handle identical refs");

    assert_eq!(stats.files_changed, 0, "Should report zero files changed");
    assert_eq!(stats.insertions, 0, "Should report zero insertions");
    assert_eq!(stats.deletions, 0, "Should report zero deletions");
}

/// Test git utility: get repository root
#[test]
fn test_get_repository_root_from_root() {
    let repo = setup_git_repo_with_history();

    let root = context_creator::utils::git::get_repository_root(repo.path())
        .expect("Should find repository root");

    // Canonicalize both paths to handle macOS symlinks (/var vs /private/var)
    let expected = repo.path().canonicalize().unwrap();
    let actual = root.canonicalize().unwrap();
    assert_eq!(
        actual, expected,
        "Should return same path when already at root"
    );
}

#[test]
fn test_get_repository_root_from_subdirectory() {
    let repo = setup_git_repo_with_history();
    let subdir = repo.path().join("subdir");
    fs::create_dir(&subdir).unwrap();

    let root = context_creator::utils::git::get_repository_root(&subdir)
        .expect("Should find repository root from subdirectory");

    // Canonicalize both paths to handle macOS symlinks (/var vs /private/var)
    let expected = repo.path().canonicalize().unwrap();
    let actual = root.canonicalize().unwrap();
    assert_eq!(actual, expected, "Should return git root from subdirectory");
}

#[test]
fn test_get_repository_root_not_git_repo() {
    let temp_dir = TempDir::new().unwrap();

    let result = context_creator::utils::git::get_repository_root(temp_dir.path());

    assert!(result.is_err(), "Should fail when not in git repository");
}
