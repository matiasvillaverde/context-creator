use context_creator::{
    agent::{AgentService, ContextRequest, ContextResponse, Representation},
    core::token::{TokenCounter, TokenEncoding},
};
use std::{fs, path::Path};
use tempfile::TempDir;

fn project() -> TempDir {
    let project = TempDir::new().unwrap();
    fs::write(project.path().join(".context-creator.toml"), "[defaults]\n").unwrap();
    project
}
fn cli(project: &Path, before: &[&str], after: &[&str]) -> assert_cmd::Command {
    let mut command = assert_cmd::Command::cargo_bin("context-creator").unwrap();
    command
        .current_dir(project)
        .args(before)
        .arg("context")
        .args(after);
    command
}
fn response(command: &mut assert_cmd::Command) -> ContextResponse {
    serde_json::from_slice(&command.assert().success().get_output().stdout).unwrap()
}
fn assert_budget(response: &ContextResponse, budget: usize, encoding: TokenEncoding) {
    assert_eq!(
        response.token_count,
        TokenCounter::with_encoding(encoding)
            .unwrap()
            .count_tokens(&serde_json::to_string(response).unwrap())
            .unwrap()
    );
    assert!(response.token_count <= budget);
}

#[test]
fn context_cli_inherits_toml_filters_budget_and_tokenizer() {
    let project = project();
    fs::write(project.path().join(".context-creator.toml"), "include = ['session.py']\nignore = ['private.py']\n[defaults]\nmax_tokens = 1300\ntokenizer = 'cl100k-base'\nsecret_scan = 'error'\n").unwrap();
    fs::write(
        project.path().join("session.py"),
        format!(
            "def validate_session(now):\n{}    return now > 10\n",
            "    # 你好 世界 session\n".repeat(250)
        ),
    )
    .unwrap();
    fs::write(
        project.path().join("private.py"),
        format!(
            "def session_private():\n    return '{}{}'\n",
            "ghp_",
            "a".repeat(36)
        ),
    )
    .unwrap();
    fs::write(
        project.path().join("session_other.py"),
        "def session_other(): return True\n",
    )
    .unwrap();
    let result = response(&mut cli(
        project.path(),
        &[],
        &["--task", "session expiration"],
    ));
    assert!(result
        .snippets
        .iter()
        .all(|snippet| snippet.path == "session.py"));
    assert!(result
        .snippets
        .iter()
        .any(|snippet| snippet.symbol.as_deref() == Some("validate_session")));
    assert_budget(&result, 1300, TokenEncoding::Cl100kBase);
}

#[test]
fn context_cli_honors_explicit_config_and_top_level_flags() {
    let project = project();
    let external = TempDir::new().unwrap();
    let config = external.path().join("settings.toml");
    fs::write(
        &config,
        "ignore = ['private.py']\n[defaults]\nmax_tokens = 5000\n",
    )
    .unwrap();
    fs::write(
        project.path().join("private.py"),
        "def session_private(): return True\n",
    )
    .unwrap();
    fs::write(
        project.path().join("session.py"),
        format!(
            "def validate_session(now):\n{}    return now > 10\n",
            "    # session data\n".repeat(250)
        ),
    )
    .unwrap();
    let mut command = assert_cmd::Command::cargo_bin("context-creator").unwrap();
    command
        .current_dir(project.path())
        .arg("--config")
        .arg(&config)
        .args(["--max-tokens", "900", "context", "--task", "session"]);
    let result = response(&mut command);
    assert!(result
        .snippets
        .iter()
        .all(|snippet| snippet.path != "private.py"));
    assert_budget(&result, 900, TokenEncoding::O200kBase);
}

#[test]
fn top_level_secret_error_policy_is_not_silently_downgraded() {
    let project = project();
    fs::write(
        project.path().join("session.py"),
        format!(
            "def session_secret():\n    return '{}{}'\n",
            "ghp_",
            "b".repeat(36)
        ),
    )
    .unwrap();
    cli(
        project.path(),
        &["--secret-scan", "error"],
        &["--task", "session"],
    )
    .assert()
    .failure()
    .stdout("")
    .stderr(predicates::str::contains("Secret detected"));
}

#[test]
fn context_specific_flags_override_top_level_and_config_defaults() {
    let project = project();
    fs::write(
        project.path().join(".context-creator.toml"),
        "[defaults]\nmax_tokens = 700\nsecret_scan = 'error'\ntokenizer = 'cl100k-base'\n",
    )
    .unwrap();
    fs::write(
        project.path().join("session.py"),
        format!(
            "def session_secret():\n    return '{}{}'\n",
            "ghp_",
            "b".repeat(36)
        ),
    )
    .unwrap();
    let result = response(&mut cli(
        project.path(),
        &["--max-tokens", "900", "--secret-scan", "error"],
        &[
            "--task",
            "session",
            "--max-tokens",
            "1400",
            "--secret-scan",
            "redact",
            "--tokenizer",
            "o200k-base",
        ],
    ));
    assert!(result.snippets.iter().any(|snippet| snippet
        .content
        .as_ref()
        .is_some_and(|content| content.contains("REDACTED"))));
    assert_budget(&result, 1400, TokenEncoding::O200kBase);
}

#[test]
fn filename_matches_return_declarations_when_identifiers_use_other_terms() {
    let project = project();
    fs::write(
        project.path().join("session.py"),
        "def validate(now: int, deadline: int) -> bool:\n    return now > deadline\n",
    )
    .unwrap();
    let result = AgentService::new()
        .build_context(ContextRequest::new(
            project.path(),
            "Fix session expiration",
        ))
        .unwrap();
    let snippet = result
        .snippets
        .iter()
        .find(|snippet| snippet.symbol.as_deref() == Some("validate"))
        .expect("filename match lost");
    assert_eq!(snippet.representation, Representation::Full);
    assert!(snippet
        .reasons
        .iter()
        .any(|reason| reason.kind == "filename_match"));
}

fn commit(repository: &git2::Repository) {
    let mut index = repository.index().unwrap();
    index
        .add_all(["*"], git2::IndexAddOption::DEFAULT, None)
        .unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repository.find_tree(tree_id).unwrap();
    let signature = git2::Signature::now("Regression", "regression@example.com").unwrap();
    repository
        .commit(Some("HEAD"), &signature, &signature, "baseline", &tree, &[])
        .unwrap();
}
#[test]
fn deleted_snake_and_camel_case_symbols_retain_exact_remaining_references() {
    for symbol in ["refresh_token", "refreshToken", "RefreshToken", "refresh"] {
        let project = project();
        let repository = git2::Repository::init(project.path()).unwrap();
        fs::write(
            project.path().join("old.py"),
            format!("def {symbol}(): return 1\n"),
        )
        .unwrap();
        fs::write(
            project.path().join("consumer.py"),
            format!("from old import {symbol}\n\ndef use_it():\n    return {symbol}()\n"),
        )
        .unwrap();
        fs::write(
            project.path().join("unrelated.py"),
            format!("def {symbol}_other(): return 2\n"),
        )
        .unwrap();
        commit(&repository);
        fs::remove_file(project.path().join("old.py")).unwrap();
        let mut request = ContextRequest::new(project.path(), "");
        request.recipe = "review_change".into();
        request.base_ref = Some("HEAD".into());
        let result = AgentService::new().build_context(request).unwrap();
        assert!(
            result
                .snippets
                .iter()
                .any(|snippet| snippet.path == "consumer.py"),
            "missed {symbol}"
        );
        assert!(
            !result
                .snippets
                .iter()
                .any(|snippet| snippet.path == "unrelated.py"),
            "partial match {symbol}"
        );
    }
}

#[test]
fn type_context_keeps_fields_and_methods_without_duplicate_bodies() {
    for (path, code, name) in [
        ("models.py", "class Session:\n    ttl_seconds: int = 3600\n\n    def expired(self, now: int) -> bool:\n        return now > self.deadline\n", "Session"),
        ("models.ts", "export class Session {\n  ttl_seconds: number = 3600;\n  expired(now: number): boolean { return now > this.deadline; }\n}\n", "Session"),
        ("Session.java", "public class Session {\n  int ttl_seconds = 3600;\n  boolean expired(int now) { return now > this.deadline; }\n}\n", "Session"),
    ] {
        let project = project(); fs::write(project.path().join(path), code).unwrap();
        let mut request = ContextRequest::new(project.path(), ""); request.seeds.push(path.into());
        let result = AgentService::new().build_context(request).unwrap();
        let snippet = result.snippets.iter().find(|snippet| snippet.symbol.as_deref() == Some(name)).unwrap();
        assert_eq!(snippet.representation, Representation::Full, "{path}");
        assert!(snippet.content.as_ref().unwrap().contains("ttl_seconds"));
        let all = result.snippets.iter().filter_map(|snippet| snippet.content.as_ref()).cloned().collect::<String>();
        assert_eq!(all.matches("return now >").count(), 1, "{path}");
        assert_budget(&result, 8000, TokenEncoding::O200kBase);
    }
}

#[test]
fn configured_exclusions_preserve_hidden_and_repository_ignore_rules() {
    let project = project();
    fs::write(
        project.path().join(".context-creator.toml"),
        "ignore = ['unrelated.py']\n",
    )
    .unwrap();
    fs::write(
        project.path().join(".contextignore"),
        "session_private.py\n",
    )
    .unwrap();
    fs::write(
        project.path().join("session.py"),
        "def validate(now): return now > 10\n",
    )
    .unwrap();
    fs::write(
        project.path().join("session_private.py"),
        "def session_private(): return True\n",
    )
    .unwrap();
    fs::write(
        project.path().join(".hidden_session.py"),
        "def hidden_session(): return True\n",
    )
    .unwrap();
    let result = response(&mut cli(project.path(), &[], &["--task", "session"]));
    assert!(result
        .snippets
        .iter()
        .any(|snippet| snippet.path == "session.py"));
    assert!(result
        .snippets
        .iter()
        .all(|snippet| snippet.path == "session.py"));
}

#[test]
fn enclosing_type_replaces_methods_and_sessions_report_removed_snippet_ids() {
    let project = project();
    let file = project.path().join("session.py");
    let padding = (0..160)
        .map(|index| format!("    field_{index}: int = {index}\n"))
        .collect::<String>();
    fs::write(
        &file,
        format!(
            "class Session:\n{padding}\n    def expired(self, now):\n        return now > 10\n"
        ),
    )
    .unwrap();
    let service = AgentService::new();
    let mut request = ContextRequest::new(project.path(), "expired");
    request.session_id = Some("types".into());
    request.max_tokens = 1200;
    let small = service.build_context(request.clone()).unwrap();
    assert_budget(&small, 1200, TokenEncoding::O200kBase);
    assert!(small.snippets.iter().any(
        |snippet| snippet.kind == "type" && snippet.representation == Representation::Signature
    ));
    assert!(small.gaps.iter().any(|gap| gap.kind == "type_body_omitted"));
    let method = small
        .snippets
        .iter()
        .find(|snippet| snippet.symbol.as_deref() == Some("expired"))
        .unwrap()
        .id
        .clone();
    request.expected_revision = Some(small.revision);
    request.max_tokens = 8000;
    let full = service.build_context(request).unwrap();
    assert_budget(&full, 8000, TokenEncoding::O200kBase);
    assert!(full
        .snippets
        .iter()
        .any(|snippet| snippet.symbol.as_deref() == Some("Session")
            && snippet
                .content
                .as_ref()
                .is_some_and(|content| content.contains("field_159"))));
    assert!(full.removed.contains(&method));
    let content = full
        .snippets
        .iter()
        .filter_map(|snippet| snippet.content.as_ref())
        .cloned()
        .collect::<String>();
    assert_eq!(content.matches("return now > 10").count(), 1);
}

#[test]
fn command_filters_override_config_filters_and_explicit_off_remains_available() {
    let project = project();
    fs::write(
        project.path().join(".context-creator.toml"),
        "include = ['unused.py']\nignore = ['session.py']\n[defaults]\nsecret_scan = 'error'\n",
    )
    .unwrap();
    let secret = format!("ghp_{}", "d".repeat(36));
    fs::write(
        project.path().join("session.py"),
        format!("def session_secret(): return '{secret}'\n"),
    )
    .unwrap();
    fs::write(
        project.path().join("unused.py"),
        "def session_unused(): return True\n",
    )
    .unwrap();
    let result = response(&mut cli(
        project.path(),
        &[],
        &[
            "--task",
            "session",
            "--include",
            "session.py",
            "--ignore",
            "unused.py",
            "--secret-scan",
            "off",
        ],
    ));
    assert!(result.snippets.iter().any(|snippet| snippet
        .content
        .as_ref()
        .is_some_and(|content| content.contains(&secret))));
    assert!(result
        .snippets
        .iter()
        .all(|snippet| snippet.path == "session.py"));
}

#[test]
fn top_level_directory_and_tokenizer_are_inherited_and_root_can_override_them() {
    let other = project();
    let project = project();
    fs::write(
        project.path().join("session.py"),
        "def session_first(): return '你好 世界'\n",
    )
    .unwrap();
    fs::write(
        other.path().join("session.py"),
        "def session_second(): return '你好 世界'\n",
    )
    .unwrap();
    let mut command = assert_cmd::Command::cargo_bin("context-creator").unwrap();
    command
        .current_dir(project.path())
        .arg("--directory")
        .arg(other.path())
        .args(["--tokenizer", "cl100k-base", "context", "--task", "session"]);
    let inherited = response(&mut command);
    assert!(inherited
        .snippets
        .iter()
        .any(|snippet| snippet.symbol.as_deref() == Some("session_second")));
    assert_budget(&inherited, 8000, TokenEncoding::Cl100kBase);
    command.arg("--root").arg(project.path());
    let overridden = response(&mut command);
    assert!(overridden
        .snippets
        .iter()
        .any(|snippet| snippet.symbol.as_deref() == Some("session_first")));
}
