#![cfg(test)]

//! Deterministic CLI-level reliability cases distilled from the disabled stress suites.

use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_cli_rejects_directory_traversal_include_pattern_e2e() {
    let temp_dir = TempDir::new().unwrap();
    fs::write(temp_dir.path().join("main.rs"), "fn main() {}\n").unwrap();

    Command::cargo_bin("context-creator")
        .unwrap()
        .arg(temp_dir.path())
        .arg("--include")
        .arg("../../../etc/passwd")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Directory traversal (..) not allowed in patterns",
        ));
}

#[test]
fn test_cli_handles_semantic_import_fanout_e2e() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();

    let module_count = 48;
    let mut main = String::new();
    for index in 0..module_count {
        main.push_str(&format!("mod module_{index:02};\n"));
        fs::write(
            src_dir.join(format!("module_{index:02}.rs")),
            format!(
                r#"
pub struct Type{index:02} {{
    pub value: usize,
}}

pub fn value_{index:02}() -> usize {{
    {index}
}}
"#
            ),
        )
        .unwrap();
    }

    main.push_str("\nfn main() {\n    let total = ");
    for index in 0..module_count {
        if index > 0 {
            main.push_str(" + ");
        }
        main.push_str(&format!("module_{index:02}::value_{index:02}()"));
    }
    main.push_str(";\n    println!(\"{}\", total);\n}\n");
    fs::write(src_dir.join("main.rs"), main).unwrap();

    let output = Command::cargo_bin("context-creator")
        .unwrap()
        .arg(&src_dir)
        .arg("--trace-imports")
        .arg("--include-callers")
        .arg("--include-types")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).unwrap();
    assert!(output.contains("main.rs"), "{output}");
    assert!(output.contains("module_00.rs"), "{output}");
    assert!(output.contains("module_47.rs"), "{output}");
    assert!(output.contains("Type00"), "{output}");
    assert!(output.contains("value_47"), "{output}");
}

#[test]
fn test_cli_handles_token_limited_large_project_e2e() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("large");
    fs::create_dir_all(&src_dir).unwrap();

    for index in 0..160 {
        fs::write(
            src_dir.join(format!("file_{index:03}.rs")),
            format!(
                r#"
pub fn file_{index:03}() -> &'static str {{
    "{}"
}}
"#,
                "x".repeat(240)
            ),
        )
        .unwrap();
    }

    let output = Command::cargo_bin("context-creator")
        .unwrap()
        .arg(&src_dir)
        .arg("--max-tokens")
        .arg("12000")
        .arg("--style")
        .arg("paths")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).unwrap();
    let selected_file_count = output
        .lines()
        .filter(|line| line.starts_with("file_") && line.ends_with(".rs"))
        .count();

    assert!(
        selected_file_count > 0,
        "token-limited run should keep at least one file:\n{output}"
    );
    assert!(
        selected_file_count < 160,
        "token-limited run should trim the generated project:\n{output}"
    );
}
