use context_creator::core::token::TokenCounter;
use serde_json::Value;
use std::fs;
use tempfile::TempDir;

#[test]
fn accounting_matches_the_actual_output_and_repeat_runs_exclude_artifacts() {
    let project = TempDir::new().unwrap();
    fs::write(
        project.path().join("main.rs"),
        "pub fn greet() -> &'static str { \"你好\" }\n",
    )
    .unwrap();
    for _ in 0..2 {
        assert_cmd::Command::cargo_bin("context-creator")
            .unwrap()
            .current_dir(project.path())
            .args([
                "--quiet",
                "--report",
                "report.json",
                "--output",
                "context.md",
            ])
            .assert()
            .success();
        let content = fs::read_to_string(project.path().join("context.md")).unwrap();
        let report: Value =
            serde_json::from_slice(&fs::read(project.path().join("report.json")).unwrap()).unwrap();
        assert_eq!(report["files_discovered"], 1);
        assert_eq!(report["files_selected"], 1);
        assert_eq!(report["output_bytes"], content.len());
        assert_eq!(
            report["context_tokens"],
            TokenCounter::new().unwrap().count_tokens(&content).unwrap()
        );
        assert_eq!(report["tokenizer"], "o200k-base");
        assert!(report["native"].is_null());
        assert!(report["timings_ms"]["rendering"].as_f64().unwrap() >= 0.0);
        assert!(!content.contains("report.json"));
        assert!(!content.contains("## context.md"));
    }
}

#[test]
fn report_and_context_cannot_overwrite_each_other() {
    let project = TempDir::new().unwrap();
    assert_cmd::Command::cargo_bin("context-creator")
        .unwrap()
        .current_dir(project.path())
        .args(["--report", "result", "--output", "./result"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("different paths"));
    assert!(!project.path().join("result").exists());
}

#[test]
fn all_content_formats_apply_secret_policy_before_writing_output() {
    let project = TempDir::new().unwrap();
    let secret = format!("ghp_{}", "a".repeat(36));
    fs::write(
        project.path().join("main.rs"),
        format!("pub fn run() {{ let credential = \"{secret}\"; }}\n"),
    )
    .unwrap();
    for style in ["markdown", "xml", "plain", "map"] {
        let output = assert_cmd::Command::cargo_bin("context-creator")
            .unwrap()
            .current_dir(project.path())
            .args(["--quiet", "--style", style, "--secret-scan", "redact"])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        let output = String::from_utf8(output).unwrap();
        assert!(!output.contains(&secret), "{style}");
        if style != "map" {
            assert!(output.contains("REDACTED:github-token"), "{style}");
        }
    }
    assert_cmd::Command::cargo_bin("context-creator")
        .unwrap()
        .current_dir(project.path())
        .args([
            "--quiet",
            "--secret-scan",
            "error",
            "--output",
            "context.md",
            "--report",
            "report.json",
        ])
        .assert()
        .failure()
        .stdout("")
        .stderr(predicates::str::contains("Secret detected"));
    assert!(!project.path().join("context.md").exists());
    assert!(!project.path().join("report.json").exists());
}

#[test]
fn documented_ignore_and_keep_files_affect_selection_without_unignoring_secrets() {
    let project = TempDir::new().unwrap();
    fs::write(project.path().join("public.rs"), "pub fn public() {}\n").unwrap();
    fs::write(project.path().join("chosen.rs"), "pub fn chosen() {}\n").unwrap();
    fs::write(project.path().join("secret.txt"), "private data\n").unwrap();
    fs::write(project.path().join(".contextignore"), "secret.txt\n").unwrap();
    fs::write(
        project.path().join(".contextkeep"),
        "chosen.rs\nsecret.txt\n",
    )
    .unwrap();
    let output = assert_cmd::Command::cargo_bin("context-creator")
        .unwrap()
        .current_dir(project.path())
        .args(["--quiet", "--style", "paths"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let output = String::from_utf8(output).unwrap();
    assert!(output.find("chosen.rs").unwrap() < output.find("public.rs").unwrap());
    assert!(!output.contains("secret.txt"));
}

#[test]
fn secret_scan_defaults_can_be_overridden_explicitly() {
    let project = TempDir::new().unwrap();
    let secret = format!("ghp_{}", "a".repeat(36));
    fs::write(project.path().join("main.rs"), format!("// {secret}\n")).unwrap();
    fs::write(
        project.path().join(".context-creator.toml"),
        "[defaults]\nsecret_scan = 'error'\n",
    )
    .unwrap();
    assert_cmd::Command::cargo_bin("context-creator")
        .unwrap()
        .current_dir(project.path())
        .arg("--quiet")
        .assert()
        .failure();
    assert_cmd::Command::cargo_bin("context-creator")
        .unwrap()
        .current_dir(project.path())
        .args(["--quiet", "--secret-scan", "off"])
        .assert()
        .success()
        .stdout(predicates::str::contains(secret));
}

#[test]
fn semantic_expansion_respects_contextignore_for_discovered_dependencies() {
    let project = TempDir::new().unwrap();
    fs::create_dir(project.path().join(".git")).unwrap();
    fs::write(
        project.path().join("main.rs"),
        "mod private;\nfn main() { private::run(); }\n",
    )
    .unwrap();
    fs::write(
        project.path().join("private.rs"),
        "pub fn run() { let private_marker = 42; }\n",
    )
    .unwrap();
    fs::write(project.path().join(".contextignore"), "private.rs\n").unwrap();
    let output = assert_cmd::Command::cargo_bin("context-creator")
        .unwrap()
        .current_dir(project.path())
        .args(["--quiet", "--include", "main.rs", "--trace-imports"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let output = String::from_utf8(output).unwrap();
    assert!(output.contains("main.rs"));
    assert!(!output.contains("private_marker"));
}

#[test]
fn strict_secret_policy_also_covers_semantically_discovered_files() {
    let project = TempDir::new().unwrap();
    fs::create_dir(project.path().join(".git")).unwrap();
    let secret = format!("ghp_{}", "a".repeat(36));
    fs::write(
        project.path().join("main.rs"),
        "mod helper;\nfn main() { helper::run(); }\n",
    )
    .unwrap();
    fs::write(
        project.path().join("helper.rs"),
        format!("// {secret}\npub fn run() {{}}\n"),
    )
    .unwrap();
    assert_cmd::Command::cargo_bin("context-creator")
        .unwrap()
        .current_dir(project.path())
        .args([
            "--quiet",
            "--include",
            "main.rs",
            "--trace-imports",
            "--secret-scan",
            "error",
        ])
        .assert()
        .failure()
        .stdout("")
        .stderr(predicates::str::contains("Secret detected"));
}

#[test]
fn semantic_expansion_uses_each_input_repository_root() {
    let project = TempDir::new().unwrap();
    let roots = [project.path().join("first"), project.path().join("second")];
    for root in &roots {
        fs::create_dir(root).unwrap();
        fs::create_dir(root.join(".git")).unwrap();
        fs::write(
            root.join("main.rs"),
            "mod helper;\nfn main() { helper::run(); }\n",
        )
        .unwrap();
        fs::write(root.join("helper.rs"), "pub fn run() {}\n").unwrap();
    }
    for (feature, include) in [
        ("--trace-imports", "**/main.rs"),
        ("--include-callers", "**/helper.rs"),
    ] {
        let output = assert_cmd::Command::cargo_bin("context-creator")
            .unwrap()
            .args(&roots)
            .args(["--quiet", "--style", "paths", "--include", include, feature])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        let paths = String::from_utf8(output).unwrap();
        for path in [
            "1-first/main.rs",
            "1-first/helper.rs",
            "2-second/main.rs",
            "2-second/helper.rs",
        ] {
            assert!(paths.contains(path), "{feature}: {paths}");
        }
    }
}

#[test]
fn disabled_secret_scanning_in_config_does_not_block_search() {
    let project = TempDir::new().unwrap();
    fs::write(project.path().join("main.rs"), "pub fn sentinel() {}\n").unwrap();
    let config = project.path().join(".context-creator.toml");
    fs::write(&config, "[defaults]\nsecret_scan = 'off'\n").unwrap();
    assert_cmd::Command::cargo_bin("context-creator")
        .unwrap()
        .current_dir(project.path())
        .args(["--quiet", "search", "sentinel", "."])
        .assert()
        .success();
    fs::write(config, "[defaults]\nsecret_scan = 'redact'\n").unwrap();
    assert_cmd::Command::cargo_bin("context-creator")
        .unwrap()
        .current_dir(project.path())
        .args(["--quiet", "search", "sentinel", "."])
        .assert()
        .failure()
        .stderr(predicates::str::contains("apply to context generation"));
    assert_cmd::Command::cargo_bin("context-creator")
        .unwrap()
        .current_dir(project.path())
        .args(["--quiet", "--secret-scan", "off", "search", "sentinel", "."])
        .assert()
        .success();
}
