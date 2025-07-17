use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
#[cfg_attr(windows, ignore = "Mock gh command has issues on Windows CI")]
fn test_github_repo_parsing_with_gh() {
    let temp_dir = TempDir::new().unwrap();
    let mock_bin_dir = temp_dir.path().join("bin");
    fs::create_dir(&mock_bin_dir).unwrap();

    // Create the mock gh script
    let _mock_repo_path = temp_dir.path().join("repo");

    #[cfg(unix)]
    {
        let mock_gh_path = mock_bin_dir.join("gh");
        let script = r#"#!/bin/sh
# Mock gh command
if [ "$1" = "repo" ] && [ "$2" = "clone" ]; then
    # The 4th argument is the target directory path (e.g., /tmp/xyz/repo)
    target_dir="$4"
    # Simulate cloning by creating the directory structure
    mkdir -p "$target_dir/src"
    echo 'fn main() {{}}' > "$target_dir/src/main.rs"
    echo '# Mock Repo' > "$target_dir/README.md"
    echo 'name = "mock-repo"' > "$target_dir/Cargo.toml"
    echo "Cloned successfully"
    exit 0
fi
if [ "$1" = "--version" ]; then
    echo "gh version 2.40.0"
    exit 0
fi
exit 1
"#;
        fs::write(&mock_gh_path, script).unwrap();

        // Make the mock script executable
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&mock_gh_path, fs::Permissions::from_mode(0o755)).unwrap();
    }

    #[cfg(windows)]
    {
        // On Windows, create gh.cmd which will be found by Command::new("gh")
        let script = r#"@echo off
echo Arguments: %* >> gh_debug.log
if "%1" == "--version" (
    echo gh version 2.40.0
    exit /b 0
)
if "%1" == "repo" if "%2" == "clone" (
    rem Extract the target directory from arguments
    rem Arguments are: repo clone fake/repo C:\path\to\temp\repo -- --depth 1
    set "target_dir=%~4"
    echo Target dir: %target_dir% >> gh_debug.log
    
    rem Create the directory structure
    if not exist "%target_dir%" mkdir "%target_dir%"
    if not exist "%target_dir%\src" mkdir "%target_dir%\src"
    
    rem Create mock files
    echo fn main() {} > "%target_dir%\src\main.rs"
    echo # Mock Repo > "%target_dir%\README.md"
    echo name = "mock-repo" > "%target_dir%\Cargo.toml"
    
    echo Cloned successfully
    exit /b 0
)
echo Command not recognized >> gh_debug.log
exit /b 1
"#;
        fs::write(mock_bin_dir.join("gh.cmd"), script).unwrap();
    }

    let mut cmd = Command::cargo_bin("context-creator").unwrap();

    // Prepend the mock bin directory to the PATH
    let original_path = std::env::var("PATH").unwrap_or_default();
    #[cfg(windows)]
    let new_path = format!("{};{}", mock_bin_dir.display(), original_path);
    #[cfg(not(windows))]
    let new_path = format!("{}:{}", mock_bin_dir.display(), original_path);

    cmd.env("PATH", new_path);
    cmd.arg("--repo").arg("https://github.com/fake/repo");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("main.rs"))
        .stdout(predicate::str::contains("README.md"));
}

#[test]
#[ignore = "Git fallback test has issues with mock script"]
fn test_github_repo_parsing_fallback_to_git() {
    let temp_dir = TempDir::new().unwrap();
    let mock_bin_dir = temp_dir.path().join("bin");
    fs::create_dir(&mock_bin_dir).unwrap();

    // Create mock git script (no gh available)
    let mock_git_path = mock_bin_dir.join("git");
    let _mock_repo_path = temp_dir.path().join("repo");

    #[cfg(unix)]
    {
        let script = r#"#!/bin/sh
# Mock git command
if [ "$1" = "clone" ]; then
    # For git clone, the last argument is the target directory
    # Find the last argument
    for last; do true; done
    target_dir="$last"
    # Simulate cloning by creating a directory with files
    mkdir -p "$target_dir/src"
    echo 'fn main() {{}}' > "$target_dir/src/main.rs"
    echo '# Mock Repo' > "$target_dir/README.md"
    echo Cloned successfully
    exit 0
fi
if [ "$1" = "--version" ]; then
    echo "git version 2.40.0"
    exit 0
fi
exit 1
"#;
        fs::write(&mock_git_path, script).unwrap();

        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&mock_git_path, fs::Permissions::from_mode(0o755)).unwrap();
    }

    #[cfg(windows)]
    {
        let script = r#"@echo off
if "%1" == "clone" (
    rem For git clone, the last argument is the target directory
    rem Get the last argument using a simple approach
    for %%a in (%*) do set "target_dir=%%a"
    mkdir "%target_dir%\src" 2>nul
    echo fn main() {} > "%target_dir%\src\main.rs"
    echo # Mock Repo > "%target_dir%\README.md"
    echo Cloned successfully
    exit /b 0
)
if "%1" == "--version" (
    echo git version 2.40.0
    exit /b 0
)
exit /b 1
"#;
        fs::write(mock_git_path.with_extension("bat"), script).unwrap();
    }

    let mut cmd = Command::cargo_bin("context-creator").unwrap();

    // Set PATH with only our mock bin (no gh available)
    cmd.env("PATH", mock_bin_dir.display().to_string());
    cmd.arg("--repo").arg("https://github.com/fake/repo");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("main.rs"));
}

#[test]
fn test_invalid_repo_url() {
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg("--repo").arg("https://gitlab.com/fake/repo");

    cmd.assert().failure().stderr(predicate::str::contains(
        "Repository URL must be a GitHub URL",
    ));
}

#[test]
fn test_repo_and_directory_mutually_exclusive_cli() {
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg("--repo")
        .arg("https://github.com/fake/repo")
        .arg(".");

    cmd.assert().failure().stderr(predicate::str::contains(
        "Cannot specify both --repo and local paths",
    ));
}

#[test]
fn test_no_git_or_gh_available() {
    let temp_dir = TempDir::new().unwrap();
    let empty_bin_dir = temp_dir.path().join("bin");
    fs::create_dir(&empty_bin_dir).unwrap();

    let mut cmd = Command::cargo_bin("context-creator").unwrap();

    // Set PATH to empty directory (no commands available)
    cmd.env("PATH", empty_bin_dir.display().to_string());
    cmd.arg("--repo").arg("https://github.com/fake/repo");

    cmd.assert().failure().stderr(predicate::str::contains(
        "Neither gh CLI nor git is available",
    ));
}

#[test]
#[ignore = "Real repository test - requires network and git/gh CLI"]
fn test_parse_own_repository() {
    // This test requires gh or git to be available and network access
    // Use our own repository as the test case
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg("--repo")
        .arg("https://github.com/matiasvillaverde/context-creator");

    let assert = cmd.assert();

    // Should succeed and contain our key source files
    assert
        .success()
        .stdout(predicate::str::contains("src/main.rs"))
        .stdout(predicate::str::contains("src/lib.rs"))
        .stdout(predicate::str::contains("Cargo.toml"));
}
