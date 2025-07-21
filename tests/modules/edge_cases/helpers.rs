//! Helper utilities for edge case testing

use assert_cmd::Command;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Standard assertion for checking that a command failed with an error
pub fn assert_error_contains(output: &std::process::Output, error_substring: &str) {
    assert!(
        !output.status.success(),
        "Expected command to fail but it succeeded"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stderr.contains(error_substring) || stdout.contains(error_substring),
        "Expected error containing '{error_substring}' but got:\nSTDERR: {stderr}\nSTDOUT: {stdout}"
    );
}

/// Create a symlink with cross-platform support
#[cfg(unix)]
pub fn create_symlink(target: &Path, link: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

#[cfg(windows)]
pub fn create_symlink(target: &Path, link: &Path) -> std::io::Result<()> {
    // On Windows, we need to determine if target is a file or directory
    if target.is_dir() {
        std::os::windows::fs::symlink_dir(target, link)
    } else {
        std::os::windows::fs::symlink_file(target, link)
    }
}

/// Create a circular symlink chain
pub fn create_circular_symlinks(temp_dir: &Path) -> std::io::Result<()> {
    let link_a = temp_dir.join("link_a");
    let link_b = temp_dir.join("link_b");

    create_symlink(&link_b, &link_a)?;
    create_symlink(&link_a, &link_b)?;

    Ok(())
}

/// Create a file with specific content patterns
pub struct PathologicalFileBuilder {
    content: Vec<u8>,
}

impl PathologicalFileBuilder {
    pub fn new() -> Self {
        Self {
            content: Vec::new(),
        }
    }

    /// Add null bytes to the content
    pub fn with_null_bytes(mut self, count: usize) -> Self {
        self.content.extend(vec![0u8; count]);
        self
    }

    /// Add mixed line endings
    pub fn with_mixed_line_endings(mut self) -> Self {
        self.content.extend_from_slice(b"line1\r\n");
        self.content.extend_from_slice(b"line2\n");
        self.content.extend_from_slice(b"line3\r");
        self
    }

    /// Add UTF-8 BOM
    pub fn with_utf8_bom(mut self) -> Self {
        let mut new_content = vec![0xEF, 0xBB, 0xBF]; // UTF-8 BOM
        new_content.extend(self.content);
        self.content = new_content;
        self
    }

    /// Add extremely long line
    pub fn with_long_line(mut self, length: usize) -> Self {
        self.content.extend(vec![b'a'; length]);
        self.content.push(b'\n');
        self
    }

    /// Add only whitespace
    pub fn with_only_whitespace(mut self) -> Self {
        self.content.extend_from_slice(b"\n\t \r\n    \n");
        self
    }

    /// Add text content
    pub fn with_text(mut self, text: &str) -> Self {
        self.content.extend_from_slice(text.as_bytes());
        self
    }

    /// Write to file
    pub fn write_to_file(self, path: &Path) -> std::io::Result<()> {
        let mut file = fs::File::create(path)?;
        file.write_all(&self.content)?;
        Ok(())
    }
}

/// Create a file with extremely large size (filled with pattern)
pub fn create_large_file(path: &Path, size_mb: usize) -> std::io::Result<()> {
    let mut file = fs::File::create(path)?;
    let pattern = b"This is a repeating pattern for the large file. ";
    let chunk_size = pattern.len();
    let total_bytes = size_mb * 1024 * 1024;
    let iterations = total_bytes / chunk_size;

    for _ in 0..iterations {
        file.write_all(pattern)?;
    }

    Ok(())
}

/// Create a file with invalid permissions (Unix only)
#[cfg(unix)]
pub fn create_readonly_file(path: &Path, content: &str) -> std::io::Result<()> {
    fs::write(path, content)?;

    use std::os::unix::fs::PermissionsExt;
    let permissions = fs::Permissions::from_mode(0o444); // Read-only
    fs::set_permissions(path, permissions)?;

    Ok(())
}

#[cfg(not(unix))]
pub fn create_readonly_file(path: &Path, content: &str) -> std::io::Result<()> {
    fs::write(path, content)?;

    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_readonly(true);
    fs::set_permissions(path, permissions)?;

    Ok(())
}

/// Create a deeply nested directory structure
pub fn create_deep_directory(base: &Path, depth: usize) -> std::io::Result<PathBuf> {
    let mut current = base.to_path_buf();

    for i in 0..depth {
        current = current.join(format!("level_{i}"));
    }

    fs::create_dir_all(&current)?;
    Ok(current)
}

/// Generate a very long file path
#[allow(dead_code)]
pub fn generate_long_path(base: &Path, target_length: usize) -> PathBuf {
    let mut path = base.to_path_buf();
    let segment = "very_long_directory_name_component_";

    while path.to_string_lossy().len() < target_length {
        path = path.join(segment);
    }

    path
}

/// Helper to run context-creator with specific arguments
pub fn run_context_creator(args: &[&str]) -> std::process::Output {
    Command::cargo_bin("context-creator")
        .unwrap()
        .args(args)
        .output()
        .expect("Failed to execute context-creator")
}

/// Helper to create a file with specific name patterns
pub fn create_file_with_special_name(
    dir: &Path,
    name: &str,
    content: &str,
) -> std::io::Result<PathBuf> {
    let path = dir.join(name);
    fs::write(&path, content)?;
    Ok(path)
}

/// Platform-specific test runner
#[allow(dead_code)]
pub struct PlatformTest;

#[allow(dead_code)]
impl PlatformTest {
    /// Run test only on Unix platforms
    #[cfg(unix)]
    pub fn unix_only<F>(test_fn: F)
    where
        F: FnOnce(),
    {
        test_fn();
    }

    #[cfg(not(unix))]
    pub fn unix_only<F>(_test_fn: F)
    where
        F: FnOnce(),
    {
        println!("Skipping Unix-only test on current platform");
    }

    /// Run test only on Windows
    #[cfg(windows)]
    pub fn windows_only<F>(test_fn: F)
    where
        F: FnOnce(),
    {
        test_fn();
    }

    #[cfg(not(windows))]
    pub fn windows_only<F>(_test_fn: F)
    where
        F: FnOnce(),
    {
        println!("Skipping Windows-only test on current platform");
    }
}

/// Helper to check if error message indicates a graceful failure
pub fn assert_graceful_failure(output: &std::process::Output) {
    assert!(!output.status.success(), "Expected command to fail");

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should not contain panic messages
    assert!(
        !stderr.contains("panic"),
        "Tool panicked instead of failing gracefully: {stderr}"
    );
    assert!(
        !stderr.contains("RUST_BACKTRACE"),
        "Tool showed backtrace instead of user-friendly error: {stderr}"
    );
}
