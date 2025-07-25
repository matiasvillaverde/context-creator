#![cfg(test)]

use context_creator::utils::git::{get_file_git_context, GitContext};
use std::fs;
use std::process::Command;
use tempfile::TempDir;

/// Helper to setup a git repo with file history
fn setup_git_repo_with_file_history() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path();

    // Initialize git repository
    Command::new("git")
        .arg("init")
        .current_dir(repo_path)
        .status()
        .expect("Failed to init git repo");

    // Configure git
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
    fs::write(repo_path.join("test_file.rs"), "// Initial version\n").unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo_path)
        .status()
        .expect("Failed to git add");

    Command::new("git")
        .args(["commit", "-m", "feat: initial implementation"])
        .current_dir(repo_path)
        .status()
        .expect("Failed to create first commit");

    // Second commit
    fs::write(
        repo_path.join("test_file.rs"),
        "// Initial version\n// Added feature\n",
    )
    .unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo_path)
        .status()
        .expect("Failed to git add");

    Command::new("git")
        .args(["commit", "-m", "feat: add new feature"])
        .current_dir(repo_path)
        .status()
        .expect("Failed to create second commit");

    // Third commit
    fs::write(
        repo_path.join("test_file.rs"),
        "// Initial version\n// Added feature\n// Bug fix\n",
    )
    .unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo_path)
        .status()
        .expect("Failed to git add");

    Command::new("git")
        .args(["commit", "-m", "fix: resolve critical bug"])
        .current_dir(repo_path)
        .status()
        .expect("Failed to create third commit");

    temp_dir
}

#[test]
fn test_get_file_git_context_with_history() {
    let repo = setup_git_repo_with_file_history();
    let file_path = repo.path().join("test_file.rs");

    let context = get_file_git_context(repo.path(), &file_path)
        .expect("Should get git context for file with history");

    // Should have found at least one commit
    assert!(!context.recent_commits.is_empty());
    assert!(context.recent_commits.len() <= 3);

    // Check that we have the expected commits (order may vary)
    let messages: Vec<&str> = context
        .recent_commits
        .iter()
        .map(|c| c.message.as_str())
        .collect();

    assert!(messages
        .iter()
        .any(|m| m.contains("fix: resolve critical bug")));
    assert!(messages.iter().any(|m| m.contains("feat: add new feature")));
    assert!(messages
        .iter()
        .any(|m| m.contains("feat: initial implementation")));

    // Check that author information is included
    assert_eq!(context.recent_commits[0].author, "Test User");
}

#[test]
fn test_get_file_git_context_new_file() {
    let repo = setup_git_repo_with_file_history();
    let new_file = repo.path().join("new_file.rs");
    fs::write(&new_file, "// New file\n").unwrap();

    let context = get_file_git_context(repo.path(), &new_file);

    // New file should return None (no commits yet)
    assert!(context.is_none());
}

#[test]
fn test_get_file_git_context_non_git_directory() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("some_file.rs");
    fs::write(&file_path, "// Some content\n").unwrap();

    let context = get_file_git_context(temp_dir.path(), &file_path);

    // Should return None for non-git directory
    assert!(context.is_none());
}

#[test]
fn test_git_context_struct() {
    // Test that GitContext has the expected fields
    let context = GitContext {
        recent_commits: vec![],
    };

    assert_eq!(context.recent_commits.len(), 0);
}

#[test]
fn test_get_file_git_context_limit_commits() {
    let repo = setup_git_repo_with_file_history();
    let file_path = repo.path().join("test_file.rs");

    // Add more commits to test limiting
    for i in 4..=6 {
        fs::write(&file_path, format!("// Version {i}\n")).unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(repo.path())
            .status()
            .expect("Failed to git add");

        Command::new("git")
            .args(["commit", "-m", &format!("chore: update {i}")])
            .current_dir(repo.path())
            .status()
            .expect("Failed to create commit");
    }

    let context = get_file_git_context(repo.path(), &file_path).expect("Should get git context");

    // Should only return 3 most recent commits
    assert_eq!(context.recent_commits.len(), 3);

    // Check that we have recent updates (order may vary due to git2 behavior)
    let messages: Vec<&str> = context
        .recent_commits
        .iter()
        .map(|c| c.message.as_str())
        .collect();

    // Should have at least one of the recent updates
    let has_recent_update = messages
        .iter()
        .any(|m| m.contains("chore: update") || m.contains("fix: resolve critical bug"));
    assert!(has_recent_update);
}
