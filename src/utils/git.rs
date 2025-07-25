//! Git utilities for executing git commands and parsing output

use anyhow::{anyhow, Result};
use git2::{Repository, Sort};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Statistics from a git diff operation
#[derive(Debug, Clone, PartialEq)]
pub struct DiffStats {
    pub files_changed: usize,
    pub insertions: usize,
    pub deletions: usize,
}

/// Information about a single commit
#[derive(Debug, Clone)]
pub struct CommitInfo {
    pub message: String,
    pub author: String,
}

/// Git context for a file containing recent commit history
#[derive(Debug, Clone)]
pub struct GitContext {
    pub recent_commits: Vec<CommitInfo>,
}

/// Validate that a git reference looks safe and reasonable
fn validate_git_reference(git_ref: &str) -> Result<()> {
    // Basic validation to prevent command injection
    if git_ref.is_empty() {
        return Err(anyhow!("Git reference cannot be empty"));
    }

    // Check for dangerous characters that could be used for command injection
    let dangerous_chars = [';', '&', '|', '`', '$', '(', ')', '\n', '\r'];
    for &ch in &dangerous_chars {
        if git_ref.contains(ch) {
            return Err(anyhow!("Invalid character in git reference: '{}'", ch));
        }
    }

    // Additional length check to prevent extremely long inputs
    if git_ref.len() > 256 {
        return Err(anyhow!("Git reference too long"));
    }

    Ok(())
}

/// Sanitize error messages to prevent information disclosure
fn sanitize_git_error(error_output: &str) -> String {
    // Remove potentially sensitive paths and information
    let sanitized = error_output
        .lines()
        .filter(|line| !line.contains("fatal:") || line.contains("unknown revision"))
        .collect::<Vec<_>>()
        .join("\n");

    if sanitized.is_empty() {
        "Invalid git reference".to_string()
    } else {
        format!("Git error: {sanitized}")
    }
}

/// Validate that a file path is safe (no directory traversal)
fn validate_file_path(path: &str) -> Result<PathBuf> {
    if path.contains("..") || path.starts_with('/') {
        return Err(anyhow!("Unsafe file path detected: {}", path));
    }
    Ok(PathBuf::from(path))
}

/// Check if a directory is a git repository
pub fn is_git_repository<P: AsRef<Path>>(path: P) -> bool {
    let git_dir = path.as_ref().join(".git");
    git_dir.exists()
}

/// Get the list of files changed between two git references
pub fn get_changed_files<P: AsRef<Path>>(
    repo_path: P,
    from: &str,
    to: &str,
) -> Result<Vec<PathBuf>> {
    // Validate git references to prevent command injection
    validate_git_reference(from)?;
    validate_git_reference(to)?;

    let output = Command::new("git")
        .args(["diff", "--name-only", from, to])
        .current_dir(repo_path.as_ref())
        .output()
        .map_err(|e| anyhow!("Failed to execute git command: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("{}", sanitize_git_error(&stderr)));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut files = Vec::new();

    for line in stdout.lines() {
        let line = line.trim();
        if !line.is_empty() {
            // Validate each file path to prevent path traversal
            let safe_path = validate_file_path(line)?;
            files.push(repo_path.as_ref().join(safe_path));
        }
    }

    Ok(files)
}

/// Get diff statistics between two git references
pub fn get_diff_stats<P: AsRef<Path>>(repo_path: P, from: &str, to: &str) -> Result<DiffStats> {
    // Validate git references to prevent command injection
    validate_git_reference(from)?;
    validate_git_reference(to)?;

    let output = Command::new("git")
        .args(["diff", "--numstat", from, to])
        .current_dir(repo_path.as_ref())
        .output()
        .map_err(|e| anyhow!("Failed to execute git command: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("{}", sanitize_git_error(&stderr)));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut stats = DiffStats {
        files_changed: 0,
        insertions: 0,
        deletions: 0,
    };

    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            stats.files_changed += 1;

            // Parse insertions (first column)
            if let Ok(insertions) = parts[0].parse::<usize>() {
                stats.insertions += insertions;
            }

            // Parse deletions (second column)
            if let Ok(deletions) = parts[1].parse::<usize>() {
                stats.deletions += deletions;
            }
        }
    }

    Ok(stats)
}

/// Get the root directory of the git repository
pub fn get_repository_root<P: AsRef<Path>>(path: P) -> Result<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(path.as_ref())
        .output()
        .map_err(|e| anyhow!("Failed to execute git command: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("{}", sanitize_git_error(&stderr)));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let root_path = stdout.trim();

    Ok(PathBuf::from(root_path))
}

/// Get git context (recent commits) for a specific file
pub fn get_file_git_context<P: AsRef<Path>>(repo_path: P, file_path: P) -> Option<GitContext> {
    // Try to open the repository
    let repo = match Repository::open(repo_path.as_ref()) {
        Ok(r) => r,
        Err(_) => return None,
    };

    // Get the relative path from repo root
    let relative_path = match file_path.as_ref().strip_prefix(repo_path.as_ref()) {
        Ok(p) => p,
        Err(_) => file_path.as_ref(),
    };

    // Create a revwalk starting from HEAD
    let mut revwalk = match repo.revwalk() {
        Ok(walk) => walk,
        Err(_) => return None,
    };

    // Configure sorting
    revwalk.set_sorting(Sort::TIME).ok()?;
    revwalk.push_head().ok()?;

    let mut commits = Vec::new();
    let max_commits = 3;

    // Walk through commits
    for oid_result in revwalk {
        if commits.len() >= max_commits {
            break;
        }

        let oid = match oid_result {
            Ok(o) => o,
            Err(_) => continue,
        };

        let commit = match repo.find_commit(oid) {
            Ok(c) => c,
            Err(_) => continue,
        };

        // Check if this commit touches our file
        let touches_file = if let Ok(parent) = commit.parent(0) {
            let parent_tree = parent.tree().ok()?;
            let commit_tree = commit.tree().ok()?;
            let diff = repo
                .diff_tree_to_tree(Some(&parent_tree), Some(&commit_tree), None)
                .ok()?;

            diff.deltas().any(|delta| {
                delta.old_file().path() == Some(relative_path)
                    || delta.new_file().path() == Some(relative_path)
            })
        } else {
            // First commit - check if file exists
            let tree = commit.tree().ok()?;
            tree.get_path(relative_path).is_ok()
        };

        if touches_file {
            let message = commit
                .message()
                .unwrap_or("")
                .lines()
                .next()
                .unwrap_or("")
                .to_string();
            let author = commit.author().name().unwrap_or("Unknown").to_string();

            commits.push(CommitInfo { message, author });
        }
    }

    if commits.is_empty() {
        None
    } else {
        Some(GitContext {
            recent_commits: commits,
        })
    }
}
