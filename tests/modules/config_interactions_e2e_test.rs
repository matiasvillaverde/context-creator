#![cfg(test)]

//! End-to-end coverage for configuration file behavior in the real CLI.

use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_config_defaults_patterns_semantic_expansion_and_priority_e2e() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path().join("app");
    fs::create_dir_all(project_dir.join("src")).unwrap();

    fs::write(
        project_dir.join("Cargo.toml"),
        r#"
[package]
name = "config-e2e"
version = "0.1.0"
edition = "2021"
"#,
    )
    .unwrap();

    fs::write(
        project_dir.join("src/main.rs"),
        r#"
mod settings;

use settings::Settings;

fn main() {
    let settings = Settings::default();
    println!("starting {}", settings.database_url);
}
"#,
    )
    .unwrap();

    fs::write(
        project_dir.join("src/settings.rs"),
        r#"
pub struct Settings {
    pub database_url: &'static str,
}

impl Settings {
    pub fn default() -> Self {
        Self {
            database_url: "postgres://localhost/config-e2e",
        }
    }
}
"#,
    )
    .unwrap();

    fs::write(
        project_dir.join("src/critical.rs"),
        r#"
pub fn critical_path() -> &'static str {
    "CRITICAL_CONFIG_PRIORITY"
}
"#,
    )
    .unwrap();

    fs::write(
        project_dir.join("src/ordinary.rs"),
        r#"
pub fn ordinary_path() -> &'static str {
    "ORDINARY_CONFIG_PRIORITY"
}
"#,
    )
    .unwrap();

    fs::write(
        project_dir.join("src/ignored.rs"),
        r#"
pub fn ignored_path() -> &'static str {
    "SHOULD_NOT_APPEAR_FROM_CONFIG_IGNORE"
}
"#,
    )
    .unwrap();
    fs::write(
        project_dir.join("README.md"),
        "SHOULD_NOT_APPEAR_FROM_README_IGNORE\n",
    )
    .unwrap();

    let config_path = temp_dir.path().join(".context-creator.toml");
    let output_path = temp_dir.path().join("configured-context.md");
    fs::write(
        &config_path,
        r#"
include = ["src/main.rs", "src/critical.rs", "src/ordinary.rs", "src/ignored.rs"]
ignore = ["src/ignored.rs", "README.md"]

[defaults]
directory = "app"
output_file = "configured-context.md"
verbose = true

[[priorities]]
pattern = "src/critical.rs"
weight = 50.0

[[priorities]]
pattern = "src/ordinary.rs"
weight = -5.0
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.current_dir(temp_dir.path())
        .arg("--config")
        .arg(&config_path)
        .arg("--trace-imports")
        .arg("--include-types")
        .assert()
        .success()
        .stdout(predicate::str::contains("configured-context.md"))
        .stderr(predicate::str::contains("Loaded configuration"));

    let output = fs::read_to_string(output_path).unwrap();
    assert!(output.contains("src/main.rs"), "{output}");
    assert!(output.contains("src/settings.rs"), "{output}");
    assert!(output.contains("pub struct Settings"), "{output}");
    assert!(output.contains("src/critical.rs"), "{output}");
    assert!(output.contains("CRITICAL_CONFIG_PRIORITY"), "{output}");
    assert!(output.contains("src/ordinary.rs"), "{output}");
    assert!(output.contains("ORDINARY_CONFIG_PRIORITY"), "{output}");
    assert!(!output.contains("src/ignored.rs"), "{output}");
    assert!(
        !output.contains("SHOULD_NOT_APPEAR_FROM_CONFIG_IGNORE"),
        "{output}"
    );
    assert!(
        !output.contains("SHOULD_NOT_APPEAR_FROM_README_IGNORE"),
        "{output}"
    );

    let critical_index = output.find("src/critical.rs").unwrap();
    let ordinary_index = output.find("src/ordinary.rs").unwrap();
    assert!(
        critical_index < ordinary_index,
        "config priority should order critical.rs before ordinary.rs:\n{output}"
    );
}

#[test]
fn test_cli_patterns_override_config_patterns_e2e() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path().join("override-app");
    fs::create_dir_all(project_dir.join("src")).unwrap();
    fs::write(
        project_dir.join("src/from_config.rs"),
        "pub const CONFIG: &str = \"config\";\n",
    )
    .unwrap();
    fs::write(
        project_dir.join("src/from_cli.rs"),
        "pub const CLI: &str = \"cli\";\n",
    )
    .unwrap();

    let config_path = temp_dir.path().join(".context-creator.toml");
    fs::write(
        &config_path,
        r#"
include = ["src/from_config.rs"]

[defaults]
directory = "override-app"
"#,
    )
    .unwrap();

    let output = Command::cargo_bin("context-creator")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("--config")
        .arg(&config_path)
        .arg("--include")
        .arg("src/from_cli.rs")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).unwrap();
    assert!(output.contains("src/from_cli.rs"), "{output}");
    assert!(output.contains("pub const CLI"), "{output}");
    assert!(!output.contains("src/from_config.rs"), "{output}");
    assert!(!output.contains("pub const CONFIG"), "{output}");
}
