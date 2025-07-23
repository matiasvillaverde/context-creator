//! Unit tests for core search functionality

use context_creator::core::search::{find_files_with_matches, SearchConfig};
use tempfile::TempDir;

#[test]
fn test_basic_search() {
    let temp_dir = TempDir::new().unwrap();

    // Create test files
    std::fs::write(
        temp_dir.path().join("file1.rs"),
        "fn authenticate_user() {}",
    )
    .unwrap();
    std::fs::write(temp_dir.path().join("file2.rs"), "// No matches here").unwrap();
    std::fs::write(temp_dir.path().join("file3.rs"), "use AuthenticateService;").unwrap();

    let config = SearchConfig {
        pattern: "authenticate",
        path: temp_dir.path(),
        case_insensitive: true,
        include_globs: &[],
        exclude_globs: &[],
    };

    let matches = find_files_with_matches(&config).unwrap();

    assert_eq!(matches.len(), 2);
    assert!(matches.iter().any(|p| p.ends_with("file1.rs")));
    assert!(matches.iter().any(|p| p.ends_with("file3.rs")));
}

#[test]
fn test_case_insensitive_search() {
    let temp_dir = TempDir::new().unwrap();

    std::fs::write(temp_dir.path().join("file1.rs"), "AuthService").unwrap();
    std::fs::write(temp_dir.path().join("file2.rs"), "authservice").unwrap();
    std::fs::write(temp_dir.path().join("file3.rs"), "AUTHSERVICE").unwrap();

    let config = SearchConfig {
        pattern: "authservice",
        path: temp_dir.path(),
        case_insensitive: true,
        include_globs: &[],
        exclude_globs: &[],
    };

    let matches = find_files_with_matches(&config).unwrap();
    assert_eq!(matches.len(), 3);
}

#[test]
fn test_search_with_include_patterns() {
    let temp_dir = TempDir::new().unwrap();

    // Create files in different directories
    std::fs::create_dir(temp_dir.path().join("src")).unwrap();
    std::fs::create_dir(temp_dir.path().join("tests")).unwrap();

    std::fs::write(
        temp_dir.path().join("src/main.rs"),
        "fn main() { authenticate(); }",
    )
    .unwrap();
    std::fs::write(temp_dir.path().join("tests/test.rs"), "authenticate();").unwrap();
    std::fs::write(temp_dir.path().join("README.md"), "authenticate users").unwrap();

    let include_patterns = vec!["**/*.rs".to_string()];
    let config = SearchConfig {
        pattern: "authenticate",
        path: temp_dir.path(),
        case_insensitive: true,
        include_globs: &include_patterns,
        exclude_globs: &[],
    };

    let matches = find_files_with_matches(&config).unwrap();

    assert_eq!(matches.len(), 2);
    assert!(matches.iter().all(|p| p.extension().unwrap() == "rs"));
}

#[test]
fn test_search_with_exclude_patterns() {
    let temp_dir = TempDir::new().unwrap();

    std::fs::create_dir(temp_dir.path().join("src")).unwrap();
    std::fs::create_dir(temp_dir.path().join("target")).unwrap();

    std::fs::write(temp_dir.path().join("src/main.rs"), "authenticate").unwrap();
    std::fs::write(temp_dir.path().join("target/debug.rs"), "authenticate").unwrap();

    let exclude_patterns = vec!["target/**".to_string()];
    let config = SearchConfig {
        pattern: "authenticate",
        path: temp_dir.path(),
        case_insensitive: true,
        include_globs: &[],
        exclude_globs: &exclude_patterns,
    };

    let matches = find_files_with_matches(&config).unwrap();

    assert_eq!(matches.len(), 1);
    assert!(matches[0].ends_with("src/main.rs"));
}

#[test]
fn test_no_matches() {
    let temp_dir = TempDir::new().unwrap();

    std::fs::write(temp_dir.path().join("file.rs"), "no matches here").unwrap();

    let config = SearchConfig {
        pattern: "nonexistent",
        path: temp_dir.path(),
        case_insensitive: true,
        include_globs: &[],
        exclude_globs: &[],
    };

    let matches = find_files_with_matches(&config).unwrap();
    assert_eq!(matches.len(), 0);
}

#[test]
fn test_special_characters_in_pattern() {
    let temp_dir = TempDir::new().unwrap();

    std::fs::write(temp_dir.path().join("file.rs"), "user_auth_handler").unwrap();

    let config = SearchConfig {
        pattern: "user_auth",
        path: temp_dir.path(),
        case_insensitive: true,
        include_globs: &[],
        exclude_globs: &[],
    };

    let matches = find_files_with_matches(&config).unwrap();
    assert_eq!(matches.len(), 1);
}
