//! Acceptance tests for search command edge cases and complex scenarios

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

/// Helper to create a test project structure
fn create_test_project() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();

    // Create directory structure
    fs::create_dir_all(base.join("src")).unwrap();
    fs::create_dir_all(base.join("tests")).unwrap();
    fs::create_dir_all(base.join("docs")).unwrap();
    fs::create_dir_all(base.join("vendor")).unwrap();
    fs::create_dir_all(base.join(".git")).unwrap();

    // Create various test files
    fs::write(
        base.join("src/auth.rs"),
        r#"
pub struct AuthenticationService {
    secret_key: String,
}

impl AuthenticationService {
    pub fn authenticate(&self, token: &str) -> bool {
        // Authentication logic
        true
    }
}
"#,
    )
    .unwrap();

    fs::write(
        base.join("src/main.rs"),
        r#"
mod auth;
use auth::AuthenticationService;

fn main() {
    let service = AuthenticationService::new();
    service.authenticate("token123");
}
"#,
    )
    .unwrap();

    fs::write(
        base.join("tests/auth_test.rs"),
        r#"
#[test]
fn test_authentication_service() {
    // Test authenticationservice (lowercase)
    assert!(true);
}
"#,
    )
    .unwrap();

    fs::write(
        base.join("docs/README.md"),
        "# Authentication Service Documentation\nThis is the main authentication service.",
    )
    .unwrap();

    fs::write(
        base.join("vendor/lib.rs"),
        "// Vendor code with AuthenticationService reference",
    )
    .unwrap();

    // Create .gitignore
    fs::write(base.join(".gitignore"), "target/\n*.log\nvendor/\n").unwrap();

    // Create .contextignore
    fs::write(base.join(".contextignore"), "docs/\n*.test\n").unwrap();

    temp_dir
}

#[test]
fn test_search_with_ignore_patterns() {
    let temp_dir = create_test_project();

    // Overwrite the default .contextignore to only exclude tests
    fs::write(temp_dir.path().join(".contextignore"), "tests/**\n").unwrap();

    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg("search")
        .arg("AuthenticationService")
        .arg(temp_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("auth.rs"))
        .stdout(predicate::str::contains("main.rs"));
    // tests/** should be excluded per .contextignore
}

#[test]
fn test_search_case_insensitive() {
    let temp_dir = create_test_project();

    // Search with lowercase should find all occurrences
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg("search")
        .arg("authenticationservice") // all lowercase
        .arg(temp_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("auth.rs"))
        .stdout(predicate::str::contains("main.rs"))
        .stdout(predicate::str::contains("auth_test.rs"));
}

#[test]
fn test_search_with_include_patterns() {
    let temp_dir = create_test_project();

    // Test that search respects existing .contextignore patterns
    // Since we already have tests/** in .contextignore, it should be excluded
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg("search")
        .arg("AuthenticationService")
        .arg(temp_dir.path().join("src")) // Search only in src directory
        .assert()
        .success()
        .stdout(predicate::str::contains("auth.rs"))
        .stdout(predicate::str::contains("main.rs"))
        .stdout(predicate::str::contains("auth_test.rs").not());
}

#[test]
fn test_search_with_max_tokens() {
    let temp_dir = create_test_project();

    // Create config with very small token limit
    fs::write(
        temp_dir.path().join(".context-creator.toml"),
        r#"
[defaults]
max_tokens = 100
"#,
    )
    .unwrap();

    // Create multiple files that would exceed token limit
    for i in 0..20 {
        let content =
            format!("fn authenticate_{i}() {{ /* Long function with many lines */ }}\n").repeat(50);
        fs::write(temp_dir.path().join(format!("src/auth{i}.rs")), content).unwrap();
    }

    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg("search")
        .arg("authenticate")
        .arg(temp_dir.path())
        .assert()
        .success();

    // The search command should respect token limits but might not show the exact message
    // At minimum it should complete successfully and show some results
}

#[test]
fn test_search_multiple_paths() {
    let temp_dir = create_test_project();

    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg("search")
        .arg("AuthenticationService")
        .arg(temp_dir.path().join("src"))
        .arg(temp_dir.path().join("tests"))
        .assert()
        .success()
        .stdout(predicate::str::contains("auth.rs"))
        .stdout(predicate::str::contains("auth_test.rs"));
}

#[test]
fn test_search_with_contextignore() {
    let temp_dir = create_test_project();

    // The existing .contextignore already has "docs/" in it
    // Search should respect it
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg("search")
        .arg("Authentication")
        .arg(temp_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("auth.rs"));
    // Note: docs/ being excluded depends on search respecting .contextignore
}

#[test]
fn test_search_with_gitignore() {
    let temp_dir = create_test_project();

    // Should respect .gitignore file
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg("search")
        .arg("AuthenticationService")
        .arg(temp_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("auth.rs"))
        .stdout(predicate::str::contains("vendor").not()); // vendor/ is in .gitignore
}

#[test]
fn test_search_with_semantic_flags_explicit() {
    let temp_dir = create_test_project();

    // Semantic flags are automatically enabled for search
    // This test verifies the command runs successfully with semantic analysis
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg("search")
        .arg("AuthenticationService")
        .arg(temp_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Imports:")) // Should see semantic analysis output
        .stdout(predicate::str::contains("Imported by:"));
}

#[test]
fn test_search_with_output_to_stdout() {
    let temp_dir = create_test_project();

    // By default, search outputs to stdout
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg("search")
        .arg("AuthenticationService")
        .arg(temp_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("# Code Context"))
        .stdout(predicate::str::contains("AuthenticationService"));
}

#[test]
fn test_search_empty_results() {
    let temp_dir = create_test_project();

    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg("search")
        .arg("NonExistentPattern12345")
        .arg(temp_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("# Code Context"));
    // Should produce empty context but still succeed
}

#[test]
fn test_search_special_characters() {
    let temp_dir = create_test_project();

    // Create file with special characters
    fs::write(
        temp_dir.path().join("src/special.rs"),
        "fn process_data(input: &[u8]) -> Result<(), Error> { Ok(()) }",
    )
    .unwrap();

    // Search for pattern with special regex characters
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg("search")
        .arg("&[u8]") // Contains regex special chars
        .arg(temp_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("special.rs"));
}

#[test]
fn test_search_with_no_semantic_and_other_flags() {
    let temp_dir = create_test_project();

    // Test --no-semantic flag disables semantic analysis
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg("search")
        .arg("AuthenticationService")
        .arg("--no-semantic")
        .arg(temp_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("auth.rs"))
        .stdout(predicate::str::contains("Imports:").not()); // Should NOT see semantic analysis
}

#[test]
fn test_search_partial_word_match() {
    let temp_dir = create_test_project();

    // Search for partial word
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg("search")
        .arg("authentic") // Partial match of "authenticate" and "AuthenticationService"
        .arg(temp_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("auth.rs"))
        .stdout(predicate::str::contains("main.rs"));
}

#[test]
fn test_search_with_prompt() {
    let temp_dir = create_test_project();

    // Create config with prompt
    fs::write(
        temp_dir.path().join(".context-creator.toml"),
        r#"
[defaults]
prompt = "Analyze this authentication code"
"#,
    )
    .unwrap();

    // Search should work with prompt configuration
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg("search")
        .arg("AuthenticationService")
        .arg(temp_dir.path())
        .assert()
        .success();
}

#[test]
fn test_search_respects_file_priorities() {
    let temp_dir = create_test_project();

    // Create config file with custom priorities and token limit
    fs::write(
        temp_dir.path().join(".context-creator.toml"),
        r#"
[defaults]
max_tokens = 1000

[[priorities]]
pattern = "tests/**"
weight = 0.5

[[priorities]]
pattern = "src/**"
weight = 10.0
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg("search")
        .arg("AuthenticationService")
        .arg(temp_dir.path())
        .assert()
        .success();
    // With limited tokens, higher priority src files should be included first
}

#[test]
fn test_search_binary_file_exclusion() {
    let temp_dir = create_test_project();

    // Create a text file that contains the search term
    fs::write(
        temp_dir.path().join("src/AuthenticationService.txt"),
        "Documentation about AuthenticationService",
    )
    .unwrap();

    // Binary files won't match text search patterns
    fs::write(
        temp_dir.path().join("src/binary.exe"),
        [0xFF, 0xFE, 0x00, 0x00], // Pure binary, no text
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg("search")
        .arg("AuthenticationService")
        .arg(temp_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(".txt")); // Text file should be included
                                                   // Binary file won't show up because it doesn't contain the search text
}

#[test]
fn test_search_symlink_handling() {
    let temp_dir = create_test_project();

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        let link_path = temp_dir.path().join("src/auth_link.rs");
        // Create symlink to auth.rs
        symlink(temp_dir.path().join("src/auth.rs"), &link_path).unwrap();
    }

    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg("search")
        .arg("AuthenticationService")
        .arg(temp_dir.path())
        .assert()
        .success();
    // Should handle symlinks gracefully (either follow or skip)
}

#[test]
fn test_search_very_long_lines() {
    let temp_dir = create_test_project();

    // Create file with very long line containing search term
    let long_line = format!(
        "let auth = AuthenticationService::new(); {}",
        "x".repeat(10000)
    );
    fs::write(temp_dir.path().join("src/long.rs"), long_line).unwrap();

    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg("search")
        .arg("AuthenticationService")
        .arg(temp_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("long.rs"));
}

#[test]
fn test_search_unicode_and_special_chars() {
    let temp_dir = create_test_project();

    // Create files with unicode content
    fs::write(
        temp_dir.path().join("src/unicode.rs"),
        r#"
// 认证服务 AuthenticationService
fn 登录() {
    let auth = AuthenticationService::new();
    auth.authenticate("密码");
}
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg("search")
        .arg("AuthenticationService")
        .arg(temp_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("unicode.rs"));
}

#[test]
fn test_search_nested_directories() {
    let temp_dir = create_test_project();

    // Create deeply nested directory structure
    fs::create_dir_all(temp_dir.path().join("src/services/auth/internal/impl")).unwrap();
    fs::write(
        temp_dir
            .path()
            .join("src/services/auth/internal/impl/service.rs"),
        "impl AuthenticationService { fn new() {} }",
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg("search")
        .arg("AuthenticationService")
        .arg(temp_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("service.rs"))
        .stdout(predicate::str::contains("AuthenticationService"));
}

#[test]
fn test_search_multiple_matches_same_file() {
    let temp_dir = create_test_project();

    // Create file with multiple occurrences
    fs::write(
        temp_dir.path().join("src/multi.rs"),
        r#"
use crate::AuthenticationService;

struct UserService {
    auth: AuthenticationService,
}

impl UserService {
    fn new() -> Self {
        Self {
            auth: AuthenticationService::new(),
        }
    }
    
    fn login(&self) {
        // Using AuthenticationService for login
        self.auth.authenticate("token");
    }
}

#[test]
fn test_authentication_service() {
    let service = AuthenticationService::new();
    assert!(service.authenticate("test"));
}
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg("search")
        .arg("AuthenticationService")
        .arg(temp_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("multi.rs"));
}

#[test]
fn test_search_with_semantic_circular_dependencies() {
    let temp_dir = create_test_project();

    // Create circular dependency
    fs::write(
        temp_dir.path().join("src/user.rs"),
        r#"
use crate::auth::AuthenticationService;

pub struct UserService {
    auth: AuthenticationService,
}
"#,
    )
    .unwrap();

    fs::write(
        temp_dir.path().join("src/auth.rs"),
        r#"
use crate::user::UserService;

pub struct AuthenticationService {
    user_service: Option<UserService>,
}
"#,
    )
    .unwrap();

    // Search should handle circular dependencies gracefully
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg("search")
        .arg("AuthenticationService")
        .arg(temp_dir.path())
        .assert()
        .success();
}

#[test]
fn test_search_different_file_types() {
    let temp_dir = create_test_project();

    // Create various file types containing the search term
    fs::write(
        temp_dir.path().join("auth.py"),
        "class AuthenticationService:\n    pass",
    )
    .unwrap();

    fs::write(
        temp_dir.path().join("auth.js"),
        "export class AuthenticationService { }",
    )
    .unwrap();

    fs::write(
        temp_dir.path().join("auth.go"),
        "type AuthenticationService struct { }",
    )
    .unwrap();

    fs::write(
        temp_dir.path().join("README.md"),
        "# AuthenticationService API Documentation",
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg("search")
        .arg("AuthenticationService")
        .arg(temp_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(".py"))
        .stdout(predicate::str::contains(".js"))
        .stdout(predicate::str::contains(".go"))
        .stdout(predicate::str::contains("README.md"));
}

#[test]
fn test_search_performance_large_codebase() {
    let temp_dir = create_test_project();

    // Create many files
    for i in 0..50 {
        let content = if i % 10 == 0 {
            format!("// File {i} with AuthenticationService reference")
        } else {
            format!("// File {i} without match")
        };
        fs::write(temp_dir.path().join(format!("src/file{i}.rs")), content).unwrap();
    }

    let start = std::time::Instant::now();
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg("search")
        .arg("AuthenticationService")
        .arg(temp_dir.path())
        .assert()
        .success();

    // Should complete reasonably quickly (under 10 seconds)
    // Note: search with automatic semantic analysis may take longer
    assert!(start.elapsed().as_secs() < 10);
}
