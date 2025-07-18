//! Remote repository fetching functionality

use crate::utils::error::ContextCreatorError;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

#[cfg(unix)]
use std::fs;

/// Check if gh CLI is available
pub fn gh_available() -> bool {
    Command::new("gh")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Check if git is available
pub fn git_available() -> bool {
    Command::new("git")
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

    Ok((parts[0].to_string(), parts[1].to_string()))
}

/// Fetch a repository from GitHub
pub fn fetch_repository(repo_url: &str, verbose: bool) -> Result<TempDir, ContextCreatorError> {
    let (owner, repo) = parse_github_url(repo_url)?;
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

    if verbose {
        eprintln!("📥 Fetching repository: {owner}/{repo}");
    }

    // Try gh first, then fall back to git
    let success = if gh_available() {
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
    let mut cmd = Command::new("gh");
    let _ = cmd
        .arg("repo")
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
    let repo_name = repo_url.split('/').next_back().ok_or_else(|| {
        ContextCreatorError::InvalidConfiguration("Invalid repository URL".to_string())
    })?;

    let mut cmd = Command::new("git");
    let _ = cmd
        .arg("clone")
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
    let (_, repo) = parse_github_url(repo_url)?;
    let repo_path = temp_dir.path().join(&repo);

    if !repo_path.exists() {
        return Err(ContextCreatorError::RemoteFetchError(format!(
            "Repository directory not found after cloning: {}",
            repo_path.display()
        )));
    }

    Ok(repo_path)
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
