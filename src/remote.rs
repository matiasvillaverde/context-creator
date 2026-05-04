//! Remote repository fetching functionality

use crate::utils::error::ContextCreatorError;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

#[cfg(unix)]
use std::fs;

fn tool_command(tool: &str) -> Command {
    let executable = resolve_tool_on_path(tool).unwrap_or_else(|| PathBuf::from(tool));

    #[cfg(windows)]
    {
        if is_windows_script(&executable) {
            let mut command = Command::new("cmd");
            command.arg("/C").arg(executable);
            return command;
        }
    }

    Command::new(executable)
}

#[cfg(windows)]
fn is_windows_script(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            extension.eq_ignore_ascii_case("bat") || extension.eq_ignore_ascii_case("cmd")
        })
        .unwrap_or(false)
}

#[cfg(windows)]
fn resolve_tool_on_path(tool: &str) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;

    for dir in std::env::split_paths(&path_var) {
        for extension in ["exe", "cmd", "bat"] {
            let candidate = dir.join(format!("{tool}.{extension}"));
            if candidate.is_file() {
                return Some(candidate);
            }
        }

        let candidate = dir.join(tool);
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    None
}

#[cfg(not(windows))]
fn resolve_tool_on_path(_tool: &str) -> Option<PathBuf> {
    None
}

/// Check if gh CLI is available
pub fn gh_available() -> bool {
    tool_command("gh")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Check if git is available
pub fn git_available() -> bool {
    tool_command("git")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Parse GitHub URL to extract owner and repo
pub fn parse_github_url(url: &str) -> Result<(String, String), ContextCreatorError> {
    let url = url.trim_end_matches('/');

    // Handle both https:// and http:// URLs
    let parts: Vec<&str> = if url.starts_with("https://github.com/") {
        url.strip_prefix("https://github.com/")
            .ok_or_else(|| {
                ContextCreatorError::InvalidConfiguration("Invalid GitHub URL".to_string())
            })?
            .split('/')
            .collect()
    } else if url.starts_with("http://github.com/") {
        url.strip_prefix("http://github.com/")
            .ok_or_else(|| {
                ContextCreatorError::InvalidConfiguration("Invalid GitHub URL".to_string())
            })?
            .split('/')
            .collect()
    } else {
        return Err(ContextCreatorError::InvalidConfiguration(
            "URL must start with https://github.com/ or http://github.com/".to_string(),
        ));
    };

    if parts.len() < 2 {
        return Err(ContextCreatorError::InvalidConfiguration(
            "GitHub URL must contain owner and repository name".to_string(),
        ));
    }

    Ok((parts[0].to_string(), repo_dir_name(parts[1])))
}

/// Fetch a repository from GitHub
pub fn fetch_repository(repo_url: &str, verbose: bool) -> Result<TempDir, ContextCreatorError> {
    let temp_dir = TempDir::new().map_err(|e| {
        ContextCreatorError::RemoteFetchError(format!("Failed to create temp directory: {e}"))
    })?;

    // Set secure permissions on temp directory (0700)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = fs::metadata(temp_dir.path()).map_err(|e| {
            ContextCreatorError::RemoteFetchError(format!(
                "Failed to get temp directory metadata: {e}"
            ))
        })?;
        let mut perms = metadata.permissions();
        perms.set_mode(0o700);
        fs::set_permissions(temp_dir.path(), perms).map_err(|e| {
            ContextCreatorError::RemoteFetchError(format!(
                "Failed to set temp directory permissions: {e}"
            ))
        })?;
    }

    let success = if let Ok((owner, repo)) = parse_github_url(repo_url) {
        if verbose {
            eprintln!("📥 Fetching repository: {owner}/{repo}");
        }

        // Try gh first, then fall back to git
        if gh_available() {
            if verbose {
                eprintln!("🔧 Using gh CLI for optimal performance");
            }
            clone_with_gh(&owner, &repo, temp_dir.path(), verbose)?
        } else if git_available() {
            if verbose {
                eprintln!("🔧 Using git clone (gh CLI not available)");
            }
            clone_with_git(repo_url, temp_dir.path(), verbose)?
        } else {
            return Err(ContextCreatorError::RemoteFetchError(
                "Neither gh CLI nor git is available. Please install one of them.".to_string(),
            ));
        }
    } else if is_local_git_remote(repo_url) {
        if verbose {
            eprintln!("📥 Fetching local git repository: {repo_url}");
        }
        if git_available() {
            clone_with_git(repo_url, temp_dir.path(), verbose)?
        } else {
            return Err(ContextCreatorError::RemoteFetchError(
                "git is required to clone local remote repositories.".to_string(),
            ));
        }
    } else {
        parse_github_url(repo_url)?;
        unreachable!("parse_github_url returned an unexpected result");
    };

    if !success {
        return Err(ContextCreatorError::RemoteFetchError(
            "Failed to clone repository".to_string(),
        ));
    }

    if verbose {
        eprintln!("✅ Repository fetched successfully");
    }

    Ok(temp_dir)
}

/// Clone repository using gh CLI
fn clone_with_gh(
    owner: &str,
    repo: &str,
    target_dir: &std::path::Path,
    verbose: bool,
) -> Result<bool, ContextCreatorError> {
    let repo_spec = format!("{owner}/{repo}");
    let mut cmd = tool_command("gh");
    cmd.arg("repo")
        .arg("clone")
        .arg(&repo_spec)
        .arg(target_dir.join(repo))
        .arg("--")
        .arg("--depth")
        .arg("1");

    if verbose {
        eprintln!("🔄 Running: gh repo clone {repo_spec} --depth 1");
    }

    let output = cmd
        .output()
        .map_err(|e| ContextCreatorError::RemoteFetchError(format!("Failed to run gh: {e}")))?;

    Ok(output.status.success())
}

/// Clone repository using git
fn clone_with_git(
    repo_url: &str,
    target_dir: &std::path::Path,
    verbose: bool,
) -> Result<bool, ContextCreatorError> {
    let repo_name = repo_dir_name(repo_url);
    if repo_name.is_empty() {
        return Err(ContextCreatorError::InvalidConfiguration(
            "Invalid repository URL".to_string(),
        ));
    }

    let mut cmd = tool_command("git");
    cmd.arg("clone")
        .arg("--depth")
        .arg("1")
        .arg(repo_url)
        .arg(target_dir.join(repo_name));

    if verbose {
        eprintln!("🔄 Running: git clone --depth 1 {repo_url}");
    }

    let output = cmd
        .output()
        .map_err(|e| ContextCreatorError::RemoteFetchError(format!("Failed to run git: {e}")))?;

    Ok(output.status.success())
}

/// Get the path to the cloned repository within the temp directory
pub fn get_repo_path(temp_dir: &TempDir, repo_url: &str) -> Result<PathBuf, ContextCreatorError> {
    let repo = if let Ok((_, repo)) = parse_github_url(repo_url) {
        repo
    } else if is_local_git_remote(repo_url) {
        repo_dir_name(repo_url)
    } else {
        parse_github_url(repo_url)?;
        unreachable!("parse_github_url returned an unexpected result");
    };
    let repo_path = temp_dir.path().join(&repo);

    if !repo_path.exists() {
        return Err(ContextCreatorError::RemoteFetchError(format!(
            "Repository directory not found after cloning: {}",
            repo_path.display()
        )));
    }

    Ok(repo_path)
}

fn is_local_git_remote(repo_url: &str) -> bool {
    repo_url.starts_with("file://") || Path::new(repo_url).exists()
}

fn repo_dir_name(repo_url: &str) -> String {
    let trimmed = repo_url.trim_end_matches('/');
    let path_part = trimmed.strip_prefix("file://").unwrap_or(trimmed);
    let name = Path::new(path_part)
        .file_name()
        .and_then(|name| name.to_str())
        .or_else(|| trimmed.split('/').next_back())
        .unwrap_or("");

    name.trim_end_matches(".git").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_github_url_https() {
        let (owner, repo) = parse_github_url("https://github.com/rust-lang/rust").unwrap();
        assert_eq!(owner, "rust-lang");
        assert_eq!(repo, "rust");
    }

    #[test]
    fn test_parse_github_url_http() {
        let (owner, repo) = parse_github_url("http://github.com/rust-lang/rust").unwrap();
        assert_eq!(owner, "rust-lang");
        assert_eq!(repo, "rust");
    }

    #[test]
    fn test_parse_github_url_trailing_slash() {
        let (owner, repo) = parse_github_url("https://github.com/rust-lang/rust/").unwrap();
        assert_eq!(owner, "rust-lang");
        assert_eq!(repo, "rust");
    }

    #[test]
    fn test_parse_github_url_invalid() {
        assert!(parse_github_url("https://gitlab.com/rust-lang/rust").is_err());
        assert!(parse_github_url("not-a-url").is_err());
        assert!(parse_github_url("https://github.com/").is_err());
        assert!(parse_github_url("https://github.com/rust-lang").is_err());
    }

    #[test]
    fn test_gh_available() {
        // This test will pass or fail depending on the environment
        // We just ensure it doesn't panic
        let _ = gh_available();
    }

    #[test]
    fn test_git_available() {
        // This test will pass or fail depending on the environment
        // We just ensure it doesn't panic
        let _ = git_available();
    }

    #[test]
    fn test_get_repo_path() {
        use std::fs;

        let temp_dir = TempDir::new().unwrap();
        let repo_url = "https://github.com/owner/repo";

        // Create the expected directory
        fs::create_dir_all(temp_dir.path().join("repo")).unwrap();

        let path = get_repo_path(&temp_dir, repo_url).unwrap();
        assert_eq!(path, temp_dir.path().join("repo"));
    }

    #[test]
    fn test_get_repo_path_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let repo_url = "https://github.com/owner/repo";

        // Don't create the directory
        let result = get_repo_path(&temp_dir, repo_url);
        assert!(result.is_err());
    }
}
