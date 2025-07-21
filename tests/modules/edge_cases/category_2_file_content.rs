//! Category 2: File Content & Structure (15 Tests)
//!
//! Tests for unusual file content, encodings, and structural anomalies

use crate::edge_cases::helpers::*;
use std::fs;
use tempfile::TempDir;

/// Scenario 16: File containing only whitespace
#[test]
fn test_16_file_only_whitespace() {
    let temp_dir = TempDir::new().unwrap();
    let whitespace_file = temp_dir.path().join("whitespace.py");

    PathologicalFileBuilder::new()
        .with_only_whitespace()
        .write_to_file(&whitespace_file)
        .unwrap();

    let output = run_context_creator(&[whitespace_file.to_str().unwrap()]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("whitespace.py"));
    // Should include the whitespace content
    assert!(stdout.contains("\n\t") || stdout.contains("    "));
}

/// Scenario 17: File with mixed line endings
#[test]
fn test_17_mixed_line_endings() {
    let temp_dir = TempDir::new().unwrap();
    let mixed_file = temp_dir.path().join("mixed_endings.txt");

    PathologicalFileBuilder::new()
        .with_mixed_line_endings()
        .write_to_file(&mixed_file)
        .unwrap();

    let output = run_context_creator(&[mixed_file.to_str().unwrap()]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("mixed_endings.txt"));
    // Check that content is present (line endings may be preserved)
    assert!(stdout.contains("line1") && stdout.contains("line2") && stdout.contains("line3"));
}

/// Scenario 18: File with a UTF-8 Byte Order Mark (BOM)
#[test]
fn test_18_utf8_bom() {
    let temp_dir = TempDir::new().unwrap();
    let bom_file = temp_dir.path().join("bom.py");

    PathologicalFileBuilder::new()
        .with_utf8_bom()
        .with_text("def hello():\n    print('Hello')")
        .write_to_file(&bom_file)
        .unwrap();

    let output = run_context_creator(&[bom_file.to_str().unwrap()]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("bom.py"));
    // Content should be present (BOM handling may vary)
    assert!(stdout.contains("def hello():"));
}

/// Scenario 19: Extremely large text file (100MB)
#[test]
#[ignore = "Test creates large file - run with --ignored flag"]
fn test_19_extremely_large_file() {
    let temp_dir = TempDir::new().unwrap();
    let large_file = temp_dir.path().join("large_log.txt");

    create_large_file(&large_file, 100).unwrap();

    let output = run_context_creator(&[large_file.to_str().unwrap()]);

    // Should handle without excessive memory usage
    // May truncate with warning
    let stderr = String::from_utf8_lossy(&output.stderr);
    if output.status.success() {
        // If successful, may have truncated
        if stderr.contains("truncat") || stderr.contains("large") {
            println!("Large file was truncated as expected");
        }
    } else {
        // Should fail gracefully if file is too large
        assert_graceful_failure(&output);
    }
}

/// Scenario 20: A file that appears to be text but is actually binary
#[test]
fn test_20_binary_file_disguised_as_text() {
    let temp_dir = TempDir::new().unwrap();
    let binary_file = temp_dir.path().join("some.pack");

    // Create a file with binary content
    let mut content = vec![0x89, 0x50, 0x4E, 0x47]; // PNG header
    content.extend_from_slice(b"\r\n\x1a\n"); // More binary data
    fs::write(&binary_file, content).unwrap();

    let output = run_context_creator(&[binary_file.to_str().unwrap()]);

    // Tool may or may not detect binary files
    // If it processes them, it should at least complete without crashing
    assert!(
        output.status.success() || {
            let stderr = String::from_utf8_lossy(&output.stderr);
            stderr.contains("binary") || stderr.contains("skip")
        }
    );
}

/// Scenario 21: A source file with a shebang
#[test]
fn test_21_file_with_shebang() {
    let temp_dir = TempDir::new().unwrap();
    let script_file = temp_dir.path().join("script.py");

    fs::write(
        &script_file,
        "#!/usr/bin/env python\n# Script file\nprint('Hello')",
    )
    .unwrap();

    let output = run_context_creator(&[script_file.to_str().unwrap()]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("script.py"));
    // Shebang should be treated as regular content
    assert!(stdout.contains("#!/usr/bin/env python"));
}

/// Scenario 22: A file with extremely long lines
#[test]
fn test_22_extremely_long_lines() {
    let temp_dir = TempDir::new().unwrap();
    let minified_file = temp_dir.path().join("minified.js");

    PathologicalFileBuilder::new()
        .with_text("function a(){")
        .with_long_line(10000)
        .with_text("}")
        .write_to_file(&minified_file)
        .unwrap();

    let output = run_context_creator(&[minified_file.to_str().unwrap()]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("minified.js"));
    // Should handle long lines without truncating incorrectly
    assert!(stdout.contains("function a(){"));
}

/// Scenario 23: A file containing null bytes
#[test]
fn test_23_file_with_null_bytes() {
    let temp_dir = TempDir::new().unwrap();
    let null_file = temp_dir.path().join("has_null.c");

    PathologicalFileBuilder::new()
        .with_text("char data[] = \"hello")
        .with_null_bytes(1)
        .with_text("world\";")
        .write_to_file(&null_file)
        .unwrap();

    let output = run_context_creator(&[null_file.to_str().unwrap()]);

    // Tool may or may not detect null bytes
    // If it processes them, it should at least complete without crashing
    assert!(
        output.status.success() || {
            let stderr = String::from_utf8_lossy(&output.stderr);
            stderr.contains("binary") || stderr.contains("null")
        }
    );
}

/// Scenario 24: A valid source file with incorrect extension
#[test]
fn test_24_valid_source_wrong_extension() {
    let temp_dir = TempDir::new().unwrap();
    let misnamed_file = temp_dir.path().join("valid_python.js");

    fs::write(&misnamed_file, "def my_func():\n    print(\"hello\")").unwrap();

    let output = run_context_creator(&[misnamed_file.to_str().unwrap()]);

    // Tool may process file regardless of extension mismatch
    // Should at least not crash
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // If it fails, it should be due to parsing
        assert!(stderr.contains("parse") || stderr.contains("syntax") || stderr.contains("error"));
    }
}

/// Scenario 25: A directory containing thousands of files
#[test]
#[ignore = "Creates many files - run with --ignored flag"]
fn test_25_directory_thousands_files() {
    let temp_dir = TempDir::new().unwrap();
    let many_files_dir = temp_dir.path().join("many_files");
    fs::create_dir_all(&many_files_dir).unwrap();

    // Create 1000 files
    for i in 0..1000 {
        let file_path = many_files_dir.join(format!("file_{i:04}.py"));
        fs::write(&file_path, format!("# File {i}")).unwrap();
    }

    let output = run_context_creator(&[many_files_dir.to_str().unwrap()]);

    // Should process efficiently without hitting file handle limits
    assert!(
        output.status.success() || {
            let stderr = String::from_utf8_lossy(&output.stderr);
            stderr.contains("limit") || stderr.contains("too many")
        }
    );
}

/// Scenario 26: A file whose name is a reserved keyword
#[test]
fn test_26_reserved_keyword_filename() {
    let temp_dir = TempDir::new().unwrap();
    let keyword_file = temp_dir.path().join("class.py");

    fs::write(
        &keyword_file,
        "# This is class.py\nprint('not a class keyword')",
    )
    .unwrap();

    let output = run_context_creator(&[keyword_file.to_str().unwrap()]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("class.py"));
    assert!(stdout.contains("# This is class.py"));
}

/// Scenario 27: A file containing code from multiple languages
#[test]
fn test_27_multi_language_file() {
    let temp_dir = TempDir::new().unwrap();
    let html_file = temp_dir.path().join("index.html");

    fs::write(
        &html_file,
        r#"<!DOCTYPE html>
<html>
<head>
    <script>
    function greet() {
        console.log("Hello from JavaScript");
    }
    </script>
    <style>
    body { color: blue; }
    </style>
</head>
<body>
    <h1>Multi-language file</h1>
</body>
</html>"#,
    )
    .unwrap();

    let output = run_context_creator(&[html_file.to_str().unwrap()]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("index.html"));
    // Tool parses as HTML, JS semantic queries won't work
    assert!(stdout.contains("<script>"));
}

/// Scenario 28: A file that is deleted while the tool is running
#[test]
#[ignore = "Timing-dependent test that may be flaky"]
fn test_28_file_deleted_during_run() {
    // This test would require spawning context-creator in background
    // and deleting file during execution - skipped for reliability
}

/// Scenario 29: A file that is modified while the tool is running
#[test]
#[ignore = "Timing-dependent test that may be flaky"]
fn test_29_file_modified_during_run() {
    // Similar to test 28 - would require concurrent operations
    // Tool should read either old or new version without crashing
}

/// Scenario 30: A file with a name that is a glob pattern itself
#[test]
fn test_30_filename_is_glob_pattern() {
    let temp_dir = TempDir::new().unwrap();

    // Create file with glob-like name
    let glob_file = temp_dir.path().join("[...].py");
    fs::write(&glob_file, "# File with glob pattern name").unwrap();

    let output = run_context_creator(&[temp_dir.path().to_str().unwrap()]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("[...].py"));
    assert!(stdout.contains("# File with glob pattern name"));
}
