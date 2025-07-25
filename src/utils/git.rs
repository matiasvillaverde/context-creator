//! Git utilities for executing git commands and parsing output

use anyhow::{anyhow, Result};
use git2::{Repository, Sort};
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::{debug, trace, warn};

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
    get_file_git_context_with_depth(repo_path, file_path, 3)
}

/// Get git context (recent commits) for a specific file with configurable depth
pub fn get_file_git_context_with_depth<P: AsRef<Path>>(
    repo_path: P,
    file_path: P,
    max_commits: usize,
) -> Option<GitContext> {
    let repo_path_str = repo_path.as_ref().display();
    let file_path_str = file_path.as_ref().display();

    trace!(
        "Getting git context for file: {} in repo: {}",
        file_path_str,
        repo_path_str
    );

    // First, try to discover the actual repository root
    let repo = match Repository::discover(repo_path.as_ref()) {
        Ok(r) => {
            debug!(
                "Successfully discovered git repository at: {}",
                repo_path_str
            );
            r
        }
        Err(e) => {
            debug!(
                "Failed to discover git repository at {}: {}",
                repo_path_str, e
            );
            return None;
        }
    };

    // Get the repository root path
    let repo_root = match repo.workdir() {
        Some(root) => {
            trace!("Repository workdir: {}", root.display());
            root
        }
        None => {
            warn!("Repository has no working directory (bare repository)");
            return None;
        }
    };

    // Get the relative path from repo root
    let file_canonical = match file_path.as_ref().canonicalize() {
        Ok(path) => path,
        Err(e) => {
            debug!("Failed to canonicalize file path {}: {}", file_path_str, e);
            return None;
        }
    };

    let repo_canonical = match repo_root.canonicalize() {
        Ok(path) => path,
        Err(e) => {
            warn!(
                "Failed to canonicalize repository path {}: {}",
                repo_root.display(),
                e
            );
            return None;
        }
    };

    let relative_path = match file_canonical.strip_prefix(repo_canonical) {
        Ok(path) => {
            trace!("Relative path in repository: {}", path.display());
            path
        }
        Err(e) => {
            debug!(
                "File {} is not within repository {}: {}",
                file_path_str,
                repo_root.display(),
                e
            );
            return None;
        }
    };

    // Create a revwalk starting from HEAD
    let mut revwalk = match repo.revwalk() {
        Ok(walk) => {
            trace!("Created revwalk for repository");
            walk
        }
        Err(e) => {
            warn!("Failed to create revwalk: {}", e);
            return None;
        }
    };

    // Configure sorting
    if let Err(e) = revwalk.set_sorting(Sort::TIME) {
        warn!("Failed to set revwalk sorting: {}", e);
        return None;
    }

    if let Err(e) = revwalk.push_head() {
        debug!(
            "Failed to push HEAD to revwalk (repository may be empty): {}",
            e
        );
        return None;
    }

    let mut commits = Vec::new();
    let mut commits_processed = 0;

    trace!(
        "Walking through commits to find those affecting file: {}",
        relative_path.display()
    );

    // Walk through commits
    for oid_result in revwalk {
        if commits.len() >= max_commits {
            trace!("Reached maximum commit limit of {}", max_commits);
            break;
        }

        let oid = match oid_result {
            Ok(o) => o,
            Err(e) => {
                debug!("Failed to get commit OID: {}", e);
                continue;
            }
        };

        let commit = match repo.find_commit(oid) {
            Ok(c) => c,
            Err(e) => {
                debug!("Failed to find commit {}: {}", oid, e);
                continue;
            }
        };

        commits_processed += 1;

        // Check if this commit touches our file
        let touches_file = if let Ok(parent) = commit.parent(0) {
            let parent_tree = match parent.tree() {
                Ok(tree) => tree,
                Err(e) => {
                    debug!("Failed to get parent tree: {}", e);
                    continue;
                }
            };
            let commit_tree = match commit.tree() {
                Ok(tree) => tree,
                Err(e) => {
                    debug!("Failed to get commit tree: {}", e);
                    continue;
                }
            };
            let diff = match repo.diff_tree_to_tree(Some(&parent_tree), Some(&commit_tree), None) {
                Ok(diff) => diff,
                Err(e) => {
                    debug!("Failed to create diff: {}", e);
                    continue;
                }
            };

            diff.deltas().any(|delta| {
                delta.old_file().path() == Some(relative_path)
                    || delta.new_file().path() == Some(relative_path)
            })
        } else {
            // First commit - check if file exists
            let tree = match commit.tree() {
                Ok(tree) => tree,
                Err(e) => {
                    debug!("Failed to get tree for root commit: {}", e);
                    continue;
                }
            };
            tree.get_path(relative_path).is_ok()
        };

        if touches_file {
            let message = commit
                .message()
                .unwrap_or("<no message>")
                .lines()
                .next()
                .unwrap_or("<no message>")
                .to_string();
            let author = commit.author().name().unwrap_or("Unknown").to_string();

            trace!("Found relevant commit: {} by {}", message, author);
            commits.push(CommitInfo { message, author });
        }
    }

    debug!(
        "Processed {} commits, found {} relevant commits for file {}",
        commits_processed,
        commits.len(),
        relative_path.display()
    );

    if commits.is_empty() {
        debug!("No git history found for file: {}", file_path_str);
        None
    } else {
        trace!("Returning git context with {} commits", commits.len());
        Some(GitContext {
            recent_commits: commits,
        })
    }
}

/// Format git context as markdown string
pub fn format_git_context_to_markdown(git_context: &GitContext) -> String {
    if git_context.recent_commits.is_empty() {
        return String::new();
    }

    let mut output = String::new();
    output.push('\n');
    output.push_str("Git history:\n");

    for (i, commit) in git_context.recent_commits.iter().enumerate().take(3) {
        if i > 0 {
            output.push('\n');
        }
        output.push_str(&format!(
            "  - {} by {}",
            commit.message.trim(),
            commit.author
        ));
    }
    output.push('\n');

    output
}
