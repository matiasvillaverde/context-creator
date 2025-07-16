//! Test specifically for semantic information in markdown output

use std::fs;
use tempfile::TempDir;

#[test]
fn test_semantic_imports_shown_in_markdown() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();

    // Create a simple module structure
    fs::write(
        src_dir.join("main.rs"),
        r#"mod lib;
use lib::hello;

fn main() {
    hello();
}"#,
    )
    .unwrap();

    fs::write(
        src_dir.join("lib.rs"),
        r#"pub fn hello() {
    println!("Hello");
}"#,
    )
    .unwrap();

    // Run with semantic analysis
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_code-digest"))
        .arg(&src_dir)
        .arg("--trace-imports")
        .arg("--include-callers")
        .arg("--progress")
        .output()
        .expect("Failed to run code-digest");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Verify semantic analysis ran
    assert!(stderr.contains("Analyzing semantic dependencies"));
    assert!(stderr.contains("Found") && stderr.contains("import"));

    // The bug: semantic information is not in the markdown
    println!("=== STDERR (progress output) ===");
    println!("{stderr}");
    println!("\n=== STDOUT (markdown output) ===");
    println!("{stdout}");

    // These assertions SHOULD pass but currently FAIL
    assert!(
        stdout.contains("main.rs")
            && (stdout.contains("imports: lib")
                || stdout.contains("Imports: lib")
                || stdout.contains("uses: lib::hello")
                || stdout.contains("Dependencies:")),
        "main.rs should show it imports lib module"
    );

    assert!(
        stdout.contains("lib.rs")
            && (stdout.contains("imported by: main.rs")
                || stdout.contains("Imported by: main.rs")
                || stdout.contains("Referenced by: main.rs")),
        "lib.rs should show it's imported by main.rs"
    );

    // TODO: This would require tracking function callers in the semantic analyzer
    // Currently we only track what functions a file calls, not who calls functions in a file
    // assert!(
    //     stdout.contains("hello()") &&
    //     (stdout.contains("called by: main") ||
    //      stdout.contains("Called by: main") ||
    //      stdout.contains("Callers: main")),
    //     "Function hello() should show it's called by main"
    // );
}
