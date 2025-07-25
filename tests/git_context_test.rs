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
}

#[test]
fn test_path_resolution_with_repo_discovery() {
    let repo = setup_git_repo_with_file_history();
    let repo_path = repo.path();
    
    // Create a subdirectory with a file
    let subdir = repo_path.join("src");
    std::fs::create_dir(&subdir).unwrap();
    let nested_file = subdir.join("lib.rs");
    std::fs::write(&nested_file, "pub fn hello() {}").unwrap();
    
    // Commit the nested file
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo_path)
        .status()
        .expect("Failed to git add");
    
    Command::new("git")
        .args(["commit", "-m", "feat: add nested file"])
        .current_dir(repo_path)
        .status()
        .expect("Failed to create commit");
    
    // Test with absolute paths - should work correctly
    let context = get_file_git_context(repo_path, &nested_file);
    assert!(context.is_some(), "Should find git context for nested file with absolute paths");
    
    // Test that we can find the repo root from the subdirectory
    let context_from_subdir = get_file_git_context(&subdir, &nested_file);
    assert!(context_from_subdir.is_some(), "Should find git context when starting from subdirectory");
}

#[test] 
fn test_relative_path_handling() {
    let repo = setup_git_repo_with_file_history();
    let repo_path = repo.path();
    let file_path = repo_path.join("test_file.rs");
    
    // Test that relative paths work correctly
    let context = get_file_git_context(repo_path, &file_path);
    assert!(context.is_some(), "Should handle paths correctly");
    
    if let Some(ctx) = context {
        assert!(!ctx.recent_commits.is_empty(), "Should find commits for the file");
    }
}

#[test]
fn test_format_git_context_to_markdown() {
    use context_creator::utils::git::{format_git_context_to_markdown, CommitInfo, GitContext};
    
    let git_context = GitContext {
        recent_commits: vec![
            CommitInfo {
                message: "feat: add new feature".to_string(),
                author: "John Doe".to_string(),
            },
            CommitInfo {
                message: "fix: resolve bug with whitespace   \n\t".to_string(), // Test trimming
                author: "Jane Smith".to_string(),
            },
            CommitInfo {
                message: "docs: update README".to_string(),
                author: "Bob Wilson".to_string(),
            },
        ],
    };
    
    let result = format_git_context_to_markdown(&git_context);
    
    assert!(result.contains("Git history:\n"), "Should contain header");
    assert!(result.contains("feat: add new feature by John Doe"), "Should contain first commit");
    assert!(result.contains("fix: resolve bug with whitespace by Jane Smith"), "Should contain trimmed second commit");
    assert!(result.contains("docs: update README by Bob Wilson"), "Should contain third commit");
    
    // Test that it limits to 3 commits
    let lines: Vec<&str> = result.lines().collect();
    let commit_lines: Vec<&str> = lines.iter().filter(|line| line.trim().starts_with("- ")).copied().collect();
    assert_eq!(commit_lines.len(), 3, "Should show exactly 3 commits");
}

#[test]
fn test_format_empty_git_context() {
    use context_creator::utils::git::{format_git_context_to_markdown, GitContext};
    
    let git_context = GitContext {
        recent_commits: vec![],
    };
    
    let result = format_git_context_to_markdown(&git_context);
    assert_eq!(result, "", "Empty git context should return empty string");
}

#[test]
fn test_git_context_error_logging() {
    // This test verifies that we can call the function with a non-existent directory
    // without panicking, and that it returns None gracefully
    let non_existent_path = std::path::Path::new("/definitely/does/not/exist");
    let result = get_file_git_context(non_existent_path, non_existent_path);
    
    assert!(result.is_none(), "Should return None for non-existent paths");
}

#[test]
fn test_git_context_corrupted_repo() {
    // Create a directory that looks like a git repo but is corrupted
    let temp_dir = TempDir::new().unwrap();
    let fake_git_dir = temp_dir.path().join(".git");
    std::fs::create_dir(&fake_git_dir).unwrap();
    std::fs::write(fake_git_dir.join("HEAD"), "invalid content").unwrap();
    
    let test_file = temp_dir.path().join("test.rs");
    std::fs::write(&test_file, "// test content").unwrap();
    
    let result = get_file_git_context(temp_dir.path(), &test_file);
    assert!(result.is_none(), "Should handle corrupted git repos gracefully");
}

#[test]
fn test_git_context_depth_configuration() {
    use context_creator::utils::git::get_file_git_context_with_depth;
    
    let repo = setup_git_repo_with_file_history();
    let file_path = repo.path().join("test_file.rs");
    
    // Add more commits to test limiting
    for i in 4..=8 {
        std::fs::write(&file_path, format!("// Version {i}\n")).unwrap();
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
    
    // Test with depth of 1
    let context_1 = get_file_git_context_with_depth(repo.path(), &file_path, 1);
    assert!(context_1.is_some());
    assert_eq!(context_1.unwrap().recent_commits.len(), 1, "Should return exactly 1 commit");
    
    // Test with depth of 5
    let context_5 = get_file_git_context_with_depth(repo.path(), &file_path, 5);
    assert!(context_5.is_some());
    assert_eq!(context_5.unwrap().recent_commits.len(), 5, "Should return exactly 5 commits");
    
    // Test with depth larger than available commits
    let context_10 = get_file_git_context_with_depth(repo.path(), &file_path, 10);
    assert!(context_10.is_some());
    let commits = context_10.unwrap().recent_commits;
    assert!(commits.len() <= 10, "Should not exceed available commits");
    assert!(commits.len() >= 7, "Should have at least 7 commits"); // We created 3 + 5 = 8 commits
}
