//! Tests for multiple output format support

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

#[test]
fn test_default_style_is_markdown() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.rs");
    std::fs::write(&test_file, "fn main() {}").unwrap();

    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg("--output-file")
        .arg(temp_dir.path().join("output.txt"))
        .arg(temp_dir.path());

    cmd.assert().success();

    let output = std::fs::read_to_string(temp_dir.path().join("output.txt")).unwrap();
    assert!(output.contains("# Code Context"));
    assert!(output.contains("```rust"));
}

#[test]
fn test_explicit_markdown_style() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.rs");
    std::fs::write(&test_file, "fn main() {}").unwrap();

    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg("--style")
        .arg("markdown")
        .arg("--output-file")
        .arg(temp_dir.path().join("output.txt"))
        .arg(temp_dir.path());

    cmd.assert().success();

    let output = std::fs::read_to_string(temp_dir.path().join("output.txt")).unwrap();
    assert!(output.contains("# Code Context"));
    assert!(output.contains("```rust"));
}

#[test]
fn test_paths_style_only_outputs_paths() {
    let temp_dir = TempDir::new().unwrap();
    std::fs::write(temp_dir.path().join("file1.rs"), "fn main() {}").unwrap();
    std::fs::write(temp_dir.path().join("file2.rs"), "fn test() {}").unwrap();

    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg("--style")
        .arg("paths")
        .arg("--output-file")
        .arg(temp_dir.path().join("output.txt"))
        .arg(temp_dir.path());

    cmd.assert().success();

    let output = std::fs::read_to_string(temp_dir.path().join("output.txt")).unwrap();
    assert!(output.contains("file1.rs"));
    assert!(output.contains("file2.rs"));
    assert!(!output.contains("fn main()"));
    assert!(!output.contains("fn test()"));
    assert!(!output.contains("```"));
}

#[test]
fn test_xml_style_outputs_valid_xml() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.rs");
    std::fs::write(&test_file, "fn main() {}").unwrap();

    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg("--style")
        .arg("xml")
        .arg("--output-file")
        .arg(temp_dir.path().join("output.xml"))
        .arg(temp_dir.path());

    cmd.assert().success();

    let output = std::fs::read_to_string(temp_dir.path().join("output.xml")).unwrap();
    assert!(output.contains("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
    assert!(output.contains("<context_creator>"));
    assert!(output.contains("<file_summary>"));
    assert!(output.contains("<files>"));
    assert!(output.contains("<![CDATA["));
}

#[test]
fn test_plain_style_outputs_plain_text() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.rs");
    std::fs::write(&test_file, "fn main() {}").unwrap();

    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg("--style")
        .arg("plain")
        .arg("--output-file")
        .arg(temp_dir.path().join("output.txt"))
        .arg(temp_dir.path());

    cmd.assert().success();

    let output = std::fs::read_to_string(temp_dir.path().join("output.txt")).unwrap();
    assert!(output.contains("================================================================"));
    assert!(output.contains("Code Digest"));
    assert!(output.contains("File Summary:"));
    assert!(!output.contains("```"));
    assert!(!output.contains("#"));
}

#[test]
fn test_invalid_style_shows_error() {
    let temp_dir = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg("--style").arg("invalid").arg(temp_dir.path());

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("possible values"));
}
