//! Category 1: Pathological Inputs & Environment (15 Tests)
//!
//! Tests for invalid inputs, environmental issues, and edge cases in path handling

use crate::edge_cases::helpers::*;
use std::fs;
use tempfile::TempDir;

/// Scenario 1: Non-existent path for `--include-callers`
#[test]
fn test_01_non_existent_path_include_callers() {
    let output = run_context_creator(&["--include-callers", "non_existent_dir/", "."]);

    assert_error_contains(&output, "does not exist");
    assert_graceful_failure(&output);
}

/// Scenario 2: Path is a file instead of a directory for positional arg
#[test]
fn test_02_file_as_positional_arg() {
    let temp_dir = TempDir::new().unwrap();
    let readme_path = temp_dir.path().join("README.md");
    fs::write(&readme_path, "Hello").unwrap();

    let output = run_context_creator(&[readme_path.to_str().unwrap()]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("README.md"));
    assert!(stdout.contains("Hello"));
}

/// Scenario 3: Path contains `../` to move up the directory tree
#[test]
fn test_03_relative_parent_path() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    let tests_dir = temp_dir.path().join("tests");

    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(&tests_dir).unwrap();

    fs::write(src_dir.join("main.py"), "# main file").unwrap();
    fs::write(tests_dir.join("test_main.py"), "# test file").unwrap();

    // Change to src directory and reference ../tests/
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_context-creator"))
        .current_dir(&src_dir)
        .arg("../tests/")
        .output()
        .expect("Failed to execute context-creator");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("test_main.py"));
}

/// Scenario 4: Broken symbolic link in the target directory
#[test]
fn test_04_broken_symbolic_link() {
    let temp_dir = TempDir::new().unwrap();
    let target_file = temp_dir.path().join("target.txt");
    let broken_link = temp_dir.path().join("broken_link");

    // Create and then delete the target to make a broken link
    fs::write(&target_file, "content").unwrap();
    create_symlink(&target_file, &broken_link).unwrap();
    fs::remove_file(&target_file).unwrap();

    let output = run_context_creator(&[temp_dir.path().to_str().unwrap()]);

    // Should skip the broken symlink without crashing
    assert!(output.status.success());
}

/// Scenario 5: Circular symbolic link dependency
#[test]
fn test_05_circular_symbolic_links() {
    let temp_dir = TempDir::new().unwrap();

    // Create circular symlinks
    if create_circular_symlinks(temp_dir.path()).is_err() {
        // Skip test if symlinks cannot be created (e.g., insufficient permissions)
        println!("Skipping circular symlink test - unable to create symlinks");
        return;
    }

    let output = run_context_creator(&[temp_dir.path().to_str().unwrap()]);

    // Should handle circular symlinks without crashing
    // Tool may either skip them or process limited depth
    assert!(
        output.status.success() || {
            let stderr = String::from_utf8_lossy(&output.stderr);
            stderr.contains("cycle") || stderr.contains("symlink") || stderr.contains("link")
        }
    );
}

/// Scenario 6: Glob pattern that matches both files and directories
#[test]
fn test_06_glob_matches_files_and_dirs() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    let api_dir = src_dir.join("api");

    fs::create_dir_all(&api_dir).unwrap();
    fs::write(src_dir.join("main.py"), "# main").unwrap();
    fs::write(api_dir.join("endpoints.py"), "# endpoints").unwrap();

    let output = run_context_creator(&["--include", "src/**", temp_dir.path().to_str().unwrap()]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("main.py"));
    assert!(stdout.contains("endpoints.py"));
}

/// Scenario 7: Shell expansion of `*` before it reaches the tool
#[test]
#[ignore = "This test is meant to demonstrate a documentation issue, not a bug"]
fn test_07_shell_expansion_without_quotes() {
    // This test demonstrates why quotes are needed in documentation
    // When running: context-creator --include *.py
    // The shell expands *.py BEFORE context-creator sees it
    // This test is ignored as it's meant for documentation purposes
}

/// Scenario 8: Invalid glob pattern in `.gitignore`
#[test]
fn test_08_invalid_gitignore_pattern() {
    let temp_dir = TempDir::new().unwrap();

    // Create .gitignore with invalid pattern
    fs::write(temp_dir.path().join(".gitignore"), "[").unwrap();
    fs::write(temp_dir.path().join("test.py"), "# test").unwrap();

    let output = run_context_creator(&[temp_dir.path().to_str().unwrap()]);

    // Tool may handle invalid gitignore patterns differently
    // Should either skip the pattern or fail gracefully
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("invalid")
                || stderr.contains("pattern")
                || stderr.contains("gitignore")
        );
    }
}

/// Scenario 9: `--output-file` points to an existing, read-only file
#[test]
fn test_09_output_file_readonly() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    let output_file = temp_dir.path().join("read_only.md");

    fs::create_dir_all(&src_dir).unwrap();
    fs::write(src_dir.join("main.py"), "# main").unwrap();

    // Create read-only file
    create_readonly_file(&output_file, "existing content").unwrap();

    let output = run_context_creator(&[
        src_dir.to_str().unwrap(),
        "--output-file",
        output_file.to_str().unwrap(),
    ]);

    // Should fail with permission or access error
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("permission") || stderr.contains("denied") || stderr.contains("read-only")
    );
}

/// Scenario 10: Very long file path (> 260 characters)
#[test]
fn test_10_very_long_file_path() {
    let temp_dir = TempDir::new().unwrap();

    // Create a deeply nested path
    let deep_path = create_deep_directory(temp_dir.path(), 20).unwrap();
    let long_file = deep_path.join("very_long_filename_that_contributes_to_path_length.py");
    fs::write(&long_file, "# content").unwrap();

    let output = run_context_creator(&[temp_dir.path().to_str().unwrap()]);

    // Should handle long paths correctly on target OS
    // Note: behavior may vary between Windows and Unix
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("# content"));
    } else {
        // On systems with path length limits, should fail gracefully
        assert_graceful_failure(&output);
    }
}

/// Scenario 11: File name with leading/trailing spaces
#[test]
fn test_11_filename_with_spaces() {
    let temp_dir = TempDir::new().unwrap();

    // Note: Some filesystems may not support leading/trailing spaces
    let filename = " file.py ";
    let file_path = create_file_with_special_name(temp_dir.path(), filename, "# space file");

    if file_path.is_ok() {
        let output = run_context_creator(&[temp_dir.path().to_str().unwrap()]);

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        // File should be included with preserved name
        assert!(stdout.contains("# space file"));
    } else {
        println!("Filesystem doesn't support filenames with leading/trailing spaces");
    }
}

/// Scenario 12: Case-sensitivity conflicts on case-insensitive filesystems
#[test]
fn test_12_case_sensitivity_conflicts() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();

    // Create file with lowercase name
    fs::write(src_dir.join("main.py"), "# lowercase").unwrap();

    let output = run_context_creator(&[
        "--include",
        "src/Main.py", // Note: uppercase M
        temp_dir.path().to_str().unwrap(),
    ]);

    // Tool behavior depends on glob implementation
    // The include pattern may be case-sensitive even on case-insensitive filesystems
    let stdout = String::from_utf8_lossy(&output.stdout);

    // If the tool found files, they should be the ones we created
    if stdout.contains("main.py") {
        assert!(stdout.contains("# lowercase"));
    }
    // Otherwise, the pattern didn't match (expected on case-sensitive matching)
}

/// Scenario 13: `--repo` with a branch that doesn't exist
#[test]
fn test_13_repo_nonexistent_branch() {
    let output = run_context_creator(&[
        "--repo",
        "https://github.com/rust-lang/rust#nonexistent-branch-xyz123",
    ]);

    // Should fail with some error (exact message may vary)
    assert!(!output.status.success());
}

/// Scenario 14: `--repo` with a repo that requires authentication
#[test]
fn test_14_repo_requires_auth() {
    // Using a private repo URL format
    let output = run_context_creator(&["--repo", "git@github.com:private-org/private-repo.git"]);

    // Should fail with some error (exact message may vary)
    assert!(!output.status.success());
}

/// Scenario 15: Flag with a missing required value
#[test]
fn test_15_flag_missing_value() {
    // Try to use --output-file without providing a path
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_context-creator"))
        .arg("--output-file")
        .output()
        .expect("Failed to execute context-creator");

    // Should fail with CLI parsing error
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("value") || stderr.contains("argument"));
}
