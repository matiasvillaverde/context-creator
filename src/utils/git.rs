//! Git utilities for executing git commands and parsing output

use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Statistics from a git diff operation
#[derive(Debug, Clone, PartialEq)]
pub struct DiffStats {
    pub files_changed: usize,
    pub insertions: usize,
    pub deletions: usize,
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
    let output = Command::new("git")
        .args(["diff", "--name-only", from, to])
        .current_dir(repo_path.as_ref())
        .output()
        .map_err(|e| anyhow!("Failed to execute git command: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("Git command failed: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let files: Vec<PathBuf> = stdout
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| repo_path.as_ref().join(line.trim()))
        .collect();

    Ok(files)
}

/// Get diff statistics between two git references
pub fn get_diff_stats<P: AsRef<Path>>(repo_path: P, from: &str, to: &str) -> Result<DiffStats> {
    let output = Command::new("git")
        .args(["diff", "--numstat", from, to])
        .current_dir(repo_path.as_ref())
        .output()
        .map_err(|e| anyhow!("Failed to execute git command: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("Git command failed: {}", stderr));
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
        return Err(anyhow!("Not a git repository: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let root_path = stdout.trim();

    Ok(PathBuf::from(root_path))
}
