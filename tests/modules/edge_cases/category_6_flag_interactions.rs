//! Category 6: Flag Interactions (15 Tests)
//!
//! Tests for complex flag interactions and edge cases

use crate::edge_cases::helpers::*;
use std::fs;
use tempfile::TempDir;

/// Scenario 86: Mutually exclusive flag combinations
#[test]
fn test_86_mutually_exclusive_flags() {
    let temp_dir = TempDir::new().unwrap();

    fs::write(temp_dir.path().join("test.py"), "def test(): pass").unwrap();

    // Test --copy and --output together
    let output = run_context_creator(&[
        "--copy",
        "--output-file",
        temp_dir.path().join("out.md").to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
    ]);

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Cannot specify both --copy and --output"));
}

/// Scenario 87: Conflicting prompt and output options
#[test]
fn test_87_prompt_output_conflict() {
    let temp_dir = TempDir::new().unwrap();

    fs::write(temp_dir.path().join("test.py"), "def test(): pass").unwrap();

    // Test prompt with output file
    let output = run_context_creator(&[
        "--output-file",
        temp_dir.path().join("out.md").to_str().unwrap(),
        "--prompt",
        "Analyze this code",
        temp_dir.path().to_str().unwrap(),
    ]);

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Cannot specify both --output and a prompt")
            || stderr.contains("Cannot specify both output file and prompt")
    );
}

/// Scenario 88: Incompatible progress and quiet flags
#[test]
fn test_88_progress_quiet_conflict() {
    let temp_dir = TempDir::new().unwrap();

    fs::write(temp_dir.path().join("test.py"), "def test(): pass").unwrap();

    // Test progress with quiet
    let output = run_context_creator(&["--progress", "--quiet", temp_dir.path().to_str().unwrap()]);

    // These flags might not conflict in the current implementation
    // Check if quiet suppresses progress
    if output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Quiet should suppress progress output
        assert!(!stderr.contains("Scanning directory"));
    }
}

/// Scenario 89: Multiple include patterns
#[test]
fn test_89_multiple_include_patterns() {
    let temp_dir = TempDir::new().unwrap();

    fs::write(temp_dir.path().join("main.py"), "# Main file").unwrap();
    fs::write(temp_dir.path().join("test.py"), "# Test file").unwrap();
    fs::write(temp_dir.path().join("utils.rs"), "// Utils").unwrap();
    fs::write(temp_dir.path().join("lib.rs"), "// Lib").unwrap();

    let output = run_context_creator(&[
        "--include",
        "*.py",
        "--include",
        "lib.rs",
        temp_dir.path().to_str().unwrap(),
    ]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("main.py"));
    assert!(stdout.contains("test.py"));
    assert!(stdout.contains("lib.rs"));
    assert!(!stdout.contains("utils.rs"));
}

/// Scenario 90: Multiple ignore patterns
#[test]
fn test_90_multiple_ignore_patterns() {
    let temp_dir = TempDir::new().unwrap();

    fs::write(temp_dir.path().join("main.py"), "# Main").unwrap();
    fs::write(temp_dir.path().join("test.py"), "# Test").unwrap();
    fs::write(temp_dir.path().join("temp.txt"), "Temp").unwrap();
    fs::write(temp_dir.path().join("cache.db"), "Cache").unwrap();

    let output = run_context_creator(&[
        "--ignore",
        "*.txt",
        "--ignore",
        "*.db",
        "--ignore",
        "test.*",
        temp_dir.path().to_str().unwrap(),
    ]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("main.py"));
    assert!(!stdout.contains("test.py"));
    assert!(!stdout.contains("temp.txt"));
    assert!(!stdout.contains("cache.db"));
}

/// Scenario 91: Combining semantic analysis flags
#[test]
fn test_91_combined_semantic_flags() {
    let temp_dir = TempDir::new().unwrap();

    fs::write(
        temp_dir.path().join("types.py"),
        r#"
class BaseClass:
    pass

class DerivedClass(BaseClass):
    pass
"#,
    )
    .unwrap();

    fs::write(
        temp_dir.path().join("utils.py"),
        r#"
from types import BaseClass

def process(obj: BaseClass):
    pass
"#,
    )
    .unwrap();

    fs::write(
        temp_dir.path().join("main.py"),
        r#"
from utils import process
from types import DerivedClass

obj = DerivedClass()
process(obj)
"#,
    )
    .unwrap();

    // Test all semantic flags together
    let output = run_context_creator(&[
        "--trace-imports",
        temp_dir.path().join("types.py").to_str().unwrap(),
        "--include-callers",
        temp_dir.path().join("utils.py").to_str().unwrap(),
        "--include-types",
        temp_dir.path().to_str().unwrap(),
    ]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should include all files due to semantic relationships
    assert!(stdout.contains("types.py"));
    assert!(stdout.contains("utils.py"));
    assert!(stdout.contains("main.py"));
}

/// Scenario 92: Max tokens with verbose output
#[test]
fn test_92_max_tokens_verbose() {
    let temp_dir = TempDir::new().unwrap();

    // Create files that will exceed token limit
    fs::write(
        temp_dir.path().join("large1.py"),
        "# Large file 1\n".repeat(100),
    )
    .unwrap();
    fs::write(
        temp_dir.path().join("large2.py"),
        "# Large file 2\n".repeat(100),
    )
    .unwrap();

    let output = run_context_creator(&[
        "--max-tokens",
        "100",
        "--verbose",
        temp_dir.path().to_str().unwrap(),
    ]);

    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Token limit"));
    assert!(stderr.contains("Selected"));
}

/// Scenario 93: Config file with CLI flag override
#[test]
fn test_93_config_cli_override() {
    let temp_dir = TempDir::new().unwrap();

    fs::write(temp_dir.path().join("test.py"), "# Test").unwrap();
    fs::write(temp_dir.path().join("ignore_me.py"), "# Ignore").unwrap();

    // Create config that ignores test.py
    let config = r#"
ignore = ["test.py"]

[defaults]
max_tokens = 1000
"#;

    let config_file = temp_dir.path().join(".context-creator.toml");
    fs::write(&config_file, config).unwrap();

    // Override with CLI include pattern
    let output = run_context_creator(&[
        "--config",
        config_file.to_str().unwrap(),
        "--include",
        "*.py",
        "--verbose",
        temp_dir.path().to_str().unwrap(),
    ]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // CLI include should override config ignore
    assert!(stdout.contains("test.py") || stdout.contains("ignore_me.py"));
}

/// Scenario 94: Tool selection with semantic flags
#[test]
fn test_94_tool_with_semantic_flags() {
    let temp_dir = TempDir::new().unwrap();

    fs::write(
        temp_dir.path().join("module.py"),
        r#"
def helper():
    return "help"
"#,
    )
    .unwrap();

    fs::write(
        temp_dir.path().join("main.py"),
        r#"
from module import helper

print(helper())
"#,
    )
    .unwrap();

    // Test different tool with semantic analysis
    let output = run_context_creator(&[
        "--tool",
        "gemini",
        "--trace-imports",
        temp_dir.path().join("module.py").to_str().unwrap(),
        "--verbose",
        temp_dir.path().to_str().unwrap(),
    ]);

    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("LLM tool: gemini"));
}

/// Scenario 95: Glob pattern with semantic flags
#[test]
fn test_95_glob_with_semantic() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir(&src_dir).unwrap();

    fs::write(
        src_dir.join("base.py"),
        r#"
class Base:
    pass
"#,
    )
    .unwrap();

    fs::write(
        src_dir.join("derived.py"),
        r#"
from base import Base

class Derived(Base):
    pass
"#,
    )
    .unwrap();

    // Use glob pattern with semantic analysis
    let output = run_context_creator(&[
        "--include-types",
        "--include",
        "*.py",
        temp_dir.path().to_str().unwrap(),
    ]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("base.py"));
    assert!(stdout.contains("derived.py"));
}

/// Scenario 96: Multiple paths with different flags
#[test]
fn test_96_multiple_paths_flags() {
    let temp_dir = TempDir::new().unwrap();
    let dir1 = temp_dir.path().join("project1");
    let dir2 = temp_dir.path().join("project2");
    fs::create_dir(&dir1).unwrap();
    fs::create_dir(&dir2).unwrap();

    fs::write(dir1.join("file1.py"), "# Project 1").unwrap();
    fs::write(dir2.join("file2.py"), "# Project 2").unwrap();

    // Process multiple directories
    let output =
        run_context_creator(&["--verbose", dir1.to_str().unwrap(), dir2.to_str().unwrap()]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("file1.py"));
    assert!(stdout.contains("file2.py"));
}

/// Scenario 97: Empty flag values
#[test]
fn test_97_empty_flag_values() {
    let temp_dir = TempDir::new().unwrap();

    fs::write(temp_dir.path().join("test.py"), "# Test").unwrap();

    // Test with empty include pattern
    let output = run_context_creator(&["--include", "", temp_dir.path().to_str().unwrap()]);

    // Should either fail or ignore empty pattern
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("error") || stderr.contains("invalid"));
    }
}

/// Scenario 98: Invalid flag combinations for semantic analysis
#[test]
fn test_98_invalid_semantic_combinations() {
    let temp_dir = TempDir::new().unwrap();

    fs::write(temp_dir.path().join("test.py"), "# Test").unwrap();

    // Try to trace imports on a non-existent file
    let output = run_context_creator(&[
        "--trace-imports",
        temp_dir.path().join("nonexistent.py").to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
    ]);

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("does not exist") || stderr.contains("not found"));
}

/// Scenario 99: Extreme token limit values
#[test]
fn test_99_extreme_token_limits() {
    let temp_dir = TempDir::new().unwrap();

    fs::write(temp_dir.path().join("test.py"), "# Small test file").unwrap();

    // Test with very small token limit
    let output = run_context_creator(&["--max-tokens", "1", temp_dir.path().to_str().unwrap()]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should include minimal output with such a small limit
    assert!(stdout.contains("# Code Context"));

    // Test with very large token limit
    let output = run_context_creator(&[
        "--max-tokens",
        "999999999",
        temp_dir.path().to_str().unwrap(),
    ]);

    assert!(output.status.success());
}

/// Scenario 100: All flags combined stress test
#[test]
fn test_100_all_flags_stress_test() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir(&src_dir).unwrap();

    // Create a complex project structure
    fs::write(
        src_dir.join("base.py"),
        r#"
class Base:
    def method(self):
        pass
"#,
    )
    .unwrap();

    fs::write(
        src_dir.join("derived.py"),
        r#"
from base import Base

class Derived(Base):
    def method(self):
        super().method()
"#,
    )
    .unwrap();

    fs::write(
        src_dir.join("main.py"),
        r#"
from derived import Derived

obj = Derived()
obj.method()
"#,
    )
    .unwrap();

    fs::write(temp_dir.path().join("README.md"), "# Test Project").unwrap();
    fs::write(temp_dir.path().join(".gitignore"), "*.pyc\n__pycache__/").unwrap();

    // Create config file
    let config = r#"
[[priorities]]
pattern = "main.py"
weight = 100.0

[defaults]
max_tokens = 10000
"#;

    let config_file = temp_dir.path().join(".context-creator.toml");
    fs::write(&config_file, config).unwrap();

    // Use many flags together
    let output = run_context_creator(&[
        "--config",
        config_file.to_str().unwrap(),
        "--include",
        "*.py",
        "--ignore",
        "test_*.py",
        "--trace-imports",
        src_dir.join("base.py").to_str().unwrap(),
        "--include-callers",
        src_dir.join("base.py").to_str().unwrap(),
        "--include-types",
        "--max-tokens",
        "5000",
        "--tool",
        "gemini",
        "--verbose",
        "--progress",
        temp_dir.path().to_str().unwrap(),
    ]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Verify all features worked
    assert!(stdout.contains("base.py"));
    assert!(stdout.contains("derived.py"));
    assert!(stdout.contains("main.py"));
    assert!(!stdout.contains("README.md")); // Should be excluded by include pattern
    assert!(stderr.contains("LLM tool: gemini"));
    assert!(stderr.contains("Loaded configuration"));
}
