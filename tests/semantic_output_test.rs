//! Integration test to verify semantic analysis data is included in markdown output

use std::fs;
use tempfile::TempDir;

/// Test that semantic analysis data (imports, callers, types) is included in the markdown output
#[test]
fn test_semantic_analysis_included_in_output() {
    // Create a temporary directory with test files
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();

    // Create a simple project structure with imports
    let lib_dir = base_path.join("src");
    fs::create_dir_all(&lib_dir).unwrap();

    // Create main.rs that imports lib
    fs::write(
        lib_dir.join("main.rs"),
        r#"
use crate::lib::helper;
use crate::utils::format;

fn main() {
    helper::init();
    format::display("Hello");
}
"#,
    )
    .unwrap();

    // Create lib.rs with helper module
    fs::write(
        lib_dir.join("lib.rs"),
        r#"
pub mod helper;
pub mod utils;

use std::collections::HashMap;

pub fn process() {
    println!("Processing");
}
"#,
    )
    .unwrap();

    // Create helper.rs
    fs::write(
        lib_dir.join("helper.rs"),
        r#"
use std::fs;

pub fn init() {
    println!("Initialized");
}

pub fn cleanup() {
    println!("Cleanup");
}
"#,
    )
    .unwrap();

    // Create utils.rs
    fs::write(
        lib_dir.join("utils.rs"),
        r#"
pub mod format;

pub fn utility() {
    println!("Utility");
}
"#,
    )
    .unwrap();

    // Create format.rs
    fs::write(
        lib_dir.join("format.rs"),
        r#"
pub fn display(msg: &str) {
    println!("{}", msg);
}
"#,
    )
    .unwrap();

    // Run code-digest with semantic analysis flags
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_code-digest"))
        .arg(&lib_dir)
        .arg("--trace-imports")
        .arg("--include-callers")
        .arg("--include-types")
        .output()
        .expect("Failed to execute code-digest");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Print output for debugging
    eprintln!("STDOUT:\n{stdout}");
    eprintln!("STDERR:\n{stderr}");

    // Verify the command succeeded
    assert!(output.status.success(), "code-digest failed: {stderr}");

    // Check that semantic analysis was performed (from stderr progress messages)
    if stderr.contains("Analyzing semantic dependencies") {
        assert!(
            stderr.contains("Found") && stderr.contains("import relationships"),
            "Semantic analysis should report found imports"
        );
    }

    // Now check if the markdown output contains semantic information
    let markdown = stdout;

    // Test 1: Check if main.rs shows its imports
    assert!(
        markdown.contains("main.rs"),
        "Output should contain main.rs"
    );

    // Look for import information near main.rs
    let main_rs_section = markdown
        .split("## main.rs")
        .nth(1)
        .unwrap_or("")
        .split("##")
        .next()
        .unwrap_or("");

    // The output SHOULD include import information, but currently doesn't
    assert!(
        main_rs_section.contains("Imports:")
            || main_rs_section.contains("imports:")
            || main_rs_section.contains("Dependencies:")
            || main_rs_section.contains("lib::helper")
            || main_rs_section.contains("utils::format"),
        "main.rs section should show its imports (lib::helper, utils::format), but found:\n{main_rs_section}"
    );

    // Test 2: Check if helper.rs shows it's imported by main.rs
    let helper_section = markdown
        .split("## helper.rs")
        .nth(1)
        .unwrap_or("")
        .split("##")
        .next()
        .unwrap_or("");

    assert!(
        helper_section.contains("Imported by:")
            || helper_section.contains("imported by:")
            || helper_section.contains("Referenced by:")
            || helper_section.contains("main.rs"),
        "helper.rs section should show it's imported by main.rs, but found:\n{helper_section}"
    );

    // Test 3: Check if function calls are shown when --include-callers is used
    assert!(
        main_rs_section.contains("helper::init") ||
        main_rs_section.contains("format::display") ||
        main_rs_section.contains("Function calls:") ||
        main_rs_section.contains("Calls:"),
        "main.rs section should show function calls (helper::init, format::display), but found:\n{main_rs_section}"
    );

    // Test 4: Check if type references are shown when --include-types is used
    let lib_section = markdown
        .split("## lib.rs")
        .nth(1)
        .unwrap_or("")
        .split("##")
        .next()
        .unwrap_or("");

    assert!(
        lib_section.contains("HashMap")
            || lib_section.contains("Type references:")
            || lib_section.contains("Types used:"),
        "lib.rs section should show type references (HashMap), but found:\n{lib_section}"
    );
}

/// Test that semantic data is properly collected even if not displayed
#[test]
fn test_semantic_analysis_progress_messages() {
    // This test verifies that semantic analysis is actually running
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();
    let src_dir = base_path.join("src");
    fs::create_dir_all(&src_dir).unwrap();

    // Create files with clear import relationships
    fs::write(
        src_dir.join("main.rs"),
        r#"
mod utils;
use utils::helper;

fn main() {
    helper();
}
"#,
    )
    .unwrap();

    fs::write(
        src_dir.join("utils.rs"),
        r#"
pub fn helper() {
    println!("Helper function");
}
"#,
    )
    .unwrap();

    // Run with progress flag to see stderr messages
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_code-digest"))
        .arg(&src_dir)
        .arg("--trace-imports")
        .arg("--progress")
        .output()
        .expect("Failed to execute code-digest");

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Verify semantic analysis ran
    assert!(
        stderr.contains("Analyzing semantic dependencies"),
        "Should show semantic analysis progress"
    );

    assert!(
        stderr.contains("Found") && stderr.contains("import relationships"),
        "Should report number of import relationships found"
    );

    // The analysis found imports, but are they in the output?
    let stdout = String::from_utf8_lossy(&output.stdout);

    // This assertion will FAIL, proving the bug
    assert!(
        stdout.contains("utils")
            && (stdout.contains("Imports:")
                || stdout.contains("Dependencies:")
                || stdout.contains("imported by")),
        "The markdown output should contain semantic relationship information"
    );
}
