use context_creator::{
    agent::{
        AgentService, Confidence, ContextRequest, ContextResponse, RecipeBook, RecipeDefinition,
        RecipeKind, Representation,
    },
    core::token::TokenCounter,
};
use std::{fs, path::Path};
use tempfile::TempDir;

fn write(root: &Path, path: &str, content: &str) {
    let target = root.join(path);
    fs::create_dir_all(target.parent().unwrap()).unwrap();
    fs::write(target, content).unwrap();
}
fn project() -> TempDir {
    let root = TempDir::new().unwrap();
    write(
        root.path(),
        "pyproject.toml",
        "[project]\nname = 'session-app'\nversion = '0.1.0'\n",
    );
    write(root.path(), "src/__init__.py", "");
    write(
        root.path(),
        "src/models.py",
        "class Session:\n    expires_at: int\n",
    );
    write(root.path(), "src/expiry.py", "from .models import Session\n\ndef session_expired(session: Session, now: int) -> bool:\n    return now >= session.expires_at\n\ndef unrelated_payment(amount: int) -> int:\n    return amount * 100\n");
    write(root.path(), "src/handler.py", "from .expiry import session_expired\nfrom .models import Session\n\ndef authorize(session: Session, now: int) -> bool:\n    return not session_expired(session, now)\n");
    write(root.path(), "tests/test_expiry.py", "from src.expiry import session_expired\n\ndef test_expiration():\n    assert session_expired(type('Session', (), {'expires_at': 5})(), 6)\n");
    write(
        root.path(),
        "config/session.toml",
        "[session]\nexpiration_seconds = 3600\n",
    );
    write(
        root.path(),
        "payments.py",
        "def charge_credit_card():\n    return 'not related'\n",
    );
    root
}
fn assert_budget(response: &ContextResponse, budget: usize) {
    let actual = TokenCounter::new()
        .unwrap()
        .count_tokens(&serde_json::to_string(response).unwrap())
        .unwrap();
    assert_eq!(actual, response.token_count);
    assert!(actual <= budget, "{actual} > {budget}");
}

#[test]
fn task_context_includes_source_types_tests_config_and_explanations() {
    let root = project();
    let service = AgentService::new();
    let response = service
        .build_context(ContextRequest::new(root.path(), "Fix session expiration"))
        .unwrap();
    assert_budget(&response, 8000);
    for path in [
        "src/expiry.py",
        "src/models.py",
        "src/handler.py",
        "tests/test_expiry.py",
        "config/session.toml",
    ] {
        assert!(
            response.snippets.iter().any(|snippet| snippet.path == path),
            "missing {path}: {}",
            serde_json::to_string_pretty(&response).unwrap()
        );
    }
    assert!(!response
        .snippets
        .iter()
        .any(|snippet| snippet.path == "payments.py"));
    assert!(response
        .snippets
        .iter()
        .all(|snippet| !snippet.reasons.is_empty()
            && snippet.start_line >= 1
            && snippet.end_line >= snippet.start_line));
    assert!(response
        .gaps
        .iter()
        .any(|gap| gap.kind == "static_analysis"));
}

#[test]
fn budgets_keep_complete_functions_or_signatures_with_exact_json_accounting() {
    let root = TempDir::new().unwrap();
    let source = format!(
        "pub fn session_expired(now: u64) -> bool {{\n{}    now > 10\n}}\n",
        "    let _padding = 12345;\n".repeat(2000)
    );
    write(root.path(), "expiry.rs", &source);
    let service = AgentService::new();
    for budget in [256, 400, 700, 1200, 8000] {
        let mut request = ContextRequest::new(root.path(), "session expiration");
        request.max_tokens = budget;
        match service.build_context(request) {
            Ok(response) => {
                assert_budget(&response, budget);
                for snippet in &response.snippets {
                    if snippet.representation == Representation::Full {
                        assert_eq!(snippet.content.as_deref(), Some(source.trim_end()));
                    } else {
                        assert!(snippet
                            .content
                            .as_deref()
                            .unwrap()
                            .starts_with("pub fn session_expired"));
                        assert!(!snippet.content.as_deref().unwrap().contains("padding"));
                    }
                }
                if budget >= 700 {
                    assert!(response
                        .snippets
                        .iter()
                        .any(|snippet| snippet.representation == Representation::Signature));
                }
            }
            Err(error) => assert!(budget < 400, "{budget}: {error:#}"),
        }
    }
}

#[test]
fn sessions_reuse_analysis_deliver_deltas_remove_deleted_symbols_and_reject_stale_cursors() {
    let root = project();
    let service = AgentService::new();
    let mut request = ContextRequest::new(root.path(), "session expiration");
    request.session_id = Some("agent-1".into());
    let first = service.build_context(request.clone()).unwrap();
    assert!(first.reset);
    request.expected_revision = Some(first.revision.clone());
    let second = service.build_context(request.clone()).unwrap();
    assert!(!second.reset);
    assert!(!second.stale);
    assert!(second.index.analysis_reused);
    assert_eq!(second.index.declarations_reparsed, 0);
    assert!(second
        .snippets
        .iter()
        .all(|snippet| snippet.content.is_none()));
    request.expected_revision = Some(second.revision.clone());
    write(root.path(), "src/expiry.py", "from .models import Session\n\ndef session_expired(session: Session, now: int) -> bool:\n    return now > session.expires_at + 30\n");
    let third = service.build_context(request.clone()).unwrap();
    assert!(third.stale);
    assert!(!third.reset);
    assert_eq!(third.index.declarations_reparsed, 1);
    assert!(third
        .snippets
        .iter()
        .any(|snippet| snippet.path == "src/expiry.py"
            && snippet
                .content
                .as_ref()
                .is_some_and(|text| text.contains("+ 30"))));
    assert!(third
        .snippets
        .iter()
        .any(|snippet| snippet.path == "src/models.py" && snippet.content.is_none()));
    assert!(service
        .build_context(request.clone())
        .unwrap_err()
        .to_string()
        .contains("conflict"));
    request.expected_revision = Some(third.revision.clone());
    let expiry_ids = third
        .snippets
        .iter()
        .filter(|snippet| snippet.path == "src/expiry.py")
        .map(|snippet| snippet.id.clone())
        .collect::<Vec<_>>();
    fs::remove_file(root.path().join("src/expiry.py")).unwrap();
    let fourth = service.build_context(request).unwrap();
    assert!(expiry_ids.iter().all(|id| fourth.removed.contains(id)));
    assert!(fourth.stale);
    assert_budget(&fourth, 8000);
}

#[test]
fn custom_recipes_share_validated_catalog_and_selection_policy() {
    let root = project();
    let mut book = RecipeBook::default();
    book.recipes.insert(
        "auth_review".into(),
        RecipeDefinition {
            extends: RecipeKind::DebugFailure,
            include_tests: Some(false),
            include_config: Some(false),
            max_snippets: Some(3),
            ..RecipeDefinition::default()
        },
    );
    assert_eq!(
        context_creator::agent::list_recipes(&book).unwrap().len(),
        4
    );
    let mut request = ContextRequest::new(root.path(), "session expiration");
    request.recipe = "auth_review".into();
    request.custom_recipes = book.recipes;
    let response = AgentService::new().build_context(request).unwrap();
    assert_eq!(response.recipe, "auth_review");
    assert!(response.snippets.len() <= 3);
    let mut invalid = RecipeBook::default();
    invalid
        .recipes
        .insert("review_change".into(), RecipeDefinition::default());
    assert!(context_creator::agent::list_recipes(&invalid).is_err());
}

#[test]
fn ignores_hidden_binary_symlinks_and_secret_policies_apply_before_selection() {
    let root = project();
    let secret = format!("ghp_{}", "a".repeat(36));
    write(
        root.path(),
        "secret.py",
        &format!("def session_secret():\n    return '{secret}'\n"),
    );
    write(
        root.path(),
        "private.py",
        "def session_private():\n    return 1\n",
    );
    write(root.path(), ".contextignore", "private.py\n");
    write(root.path(), ".hidden.py", "def session_hidden(): pass\n");
    write(root.path(), "binary.py", "session\0secret");
    let service = AgentService::new();
    let request = ContextRequest::new(root.path(), "session");
    let response = service.build_context(request.clone()).unwrap();
    let encoded = serde_json::to_string(&response).unwrap();
    assert!(!encoded.contains(&secret));
    assert!(encoded.contains("REDACTED:github-token"));
    for path in ["private.py", ".hidden.py", "binary.py", "../private.py"] {
        let mut seeded = request.clone();
        seeded.seeds.push(path.into());
        assert!(service.build_context(seeded).is_err(), "{path}");
    }
    let mut strict = request;
    strict.secret_scan = context_creator::core::secrets::SecretPolicy::Error;
    assert!(service.build_context(strict).is_err());
}

#[test]
fn checkpoint_survives_process_lifetime_without_persisting_source() {
    let root = project();
    let checkpoints = TempDir::new().unwrap();
    let path = checkpoints.path().join("agent.json");
    let mut request = ContextRequest::new(root.path(), "session expiration");
    request.session_id = Some("agent".into());
    let first = AgentService::new()
        .build_context_with_checkpoint(request.clone(), &path)
        .unwrap();
    let data = fs::read_to_string(&path).unwrap();
    assert!(!data.contains("def session_expired"));
    assert!(!data.contains("expires_at"));
    request.expected_revision = Some(first.revision);
    let second = AgentService::new()
        .build_context_with_checkpoint(request.clone(), &path)
        .unwrap();
    assert!(second
        .snippets
        .iter()
        .all(|snippet| snippet.content.is_none()));
    assert!(AgentService::new()
        .build_context_with_checkpoint(request, &root.path().join("state.json"))
        .is_err());
}

fn commit(repository: &git2::Repository, message: &str) -> git2::Oid {
    let mut index = repository.index().unwrap();
    index
        .add_all(["*"], git2::IndexAddOption::DEFAULT, None)
        .unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repository.find_tree(tree_id).unwrap();
    let signature = git2::Signature::now("Tester", "test@example.com").unwrap();
    let parent = repository
        .head()
        .ok()
        .and_then(|head| head.peel_to_commit().ok());
    repository
        .commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            &parent.iter().collect::<Vec<_>>(),
        )
        .unwrap()
}

#[test]
fn change_recipe_includes_worktree_impact_tests_and_deleted_symbols() {
    let root = project();
    let repository = git2::Repository::init(root.path()).unwrap();
    let baseline = commit(&repository, "baseline");
    write(root.path(), "src/expiry.py", "from .models import Session\n\ndef session_expired(session: Session, now: int) -> bool:\n    return now >= session.expires_at + 60\n");
    let mut request = ContextRequest::new(root.path(), "");
    request.recipe = "review_change".into();
    request.base_ref = Some(baseline.to_string());
    let response = AgentService::new().build_context(request).unwrap();
    assert!(response
        .changes
        .iter()
        .any(|change| change.path == "src/expiry.py"
            && change
                .removed_symbols
                .contains(&"unrelated_payment".to_string())));
    assert!(response
        .snippets
        .iter()
        .any(
            |snippet| snippet.symbol.as_deref() == Some("session_expired")
                && snippet
                    .reasons
                    .iter()
                    .any(|reason| reason.kind == "changed")
        ));
    assert!(response
        .snippets
        .iter()
        .any(|snippet| snippet.path == "src/handler.py"));
    assert!(response
        .snippets
        .iter()
        .any(|snippet| snippet.path == "tests/test_expiry.py"));
}

#[test]
fn traces_ts_client_openapi_python_handler_and_schema_types_with_uncertainty() {
    let root = TempDir::new().unwrap();
    write(root.path(), "web/client.ts", "export async function refreshSession() { return fetch('/api/session/refresh', {method: 'POST'}); }\n");
    write(root.path(), "backend/api.py", "@app.post('/api/session/refresh')\ndef refresh_session(payload: RefreshRequest):\n    return {'ok': True}\n");
    write(
        root.path(),
        "backend/models.py",
        "class RefreshRequest:\n    token: str\n",
    );
    write(
        root.path(),
        "backend/wrong_method.py",
        "@app.get('/api/session/refresh')\ndef other():\n    return {}\n",
    );
    write(
        root.path(),
        "api/openapi.json",
        r##"{"openapi":"3.1.0","paths":{"/api/session/refresh":{"post":{"requestBody":{"content":{"application/json":{"schema":{"$ref":"#/components/schemas/RefreshRequest"}}}}}}}}"##,
    );
    let mut request = ContextRequest::new(root.path(), "");
    request.seeds.push("web/client.ts".into());
    let response = AgentService::new().build_context(request).unwrap();
    for path in ["backend/api.py", "backend/models.py", "api/openapi.json"] {
        assert!(
            response.snippets.iter().any(|snippet| snippet.path == path),
            "{path}: {}",
            serde_json::to_string_pretty(&response).unwrap()
        );
    }
    assert!(!response
        .relations
        .iter()
        .any(|relation| relation.kind == "http_contract"
            && (relation.from.path == "backend/wrong_method.py"
                || relation.to.path == "backend/wrong_method.py")));
    assert!(response
        .relations
        .iter()
        .any(|relation| relation.kind == "http_contract"
            && relation.confidence == Confidence::Inferred));
    assert!(response
        .relations
        .iter()
        .any(|relation| relation.kind == "schema_type"));
}

#[test]
fn cli_returns_budgeted_json_and_discovers_custom_recipes() {
    let root = project();
    let output = assert_cmd::Command::cargo_bin("context-creator")
        .unwrap()
        .args(["context", "--root"])
        .arg(root.path())
        .args(["--task", "session expiration", "--max-tokens", "3000"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let response: ContextResponse = serde_json::from_slice(&output).unwrap();
    assert_budget(&response, 3000);
    let output = assert_cmd::Command::cargo_bin("context-creator")
        .unwrap()
        .arg("recipes")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let recipes: Vec<context_creator::agent::Recipe> = serde_json::from_slice(&output).unwrap();
    assert_eq!(recipes.len(), 3);
}

#[test]
fn whole_declarations_and_locations_work_across_thirteen_language_families() {
    let fixtures = [
        (
            "sample.rs",
            "pub fn session_expired(now: u64) -> bool { now > 10 }\n",
            "session_expired",
        ),
        (
            "sample.py",
            "def session_expired(now: int) -> bool:\n    return now > 10\n",
            "session_expired",
        ),
        (
            "sample.js",
            "export const sessionExpired = (now) => now > 10;\n",
            "sessionExpired",
        ),
        (
            "sample.ts",
            "export const sessionExpired = (now: number): boolean => { return now > 10; };\n",
            "sessionExpired",
        ),
        (
            "sample.go",
            "package sample\nfunc SessionExpired(now int) bool { return now > 10 }\n",
            "SessionExpired",
        ),
        (
            "sample.swift",
            "func sessionExpired(now: Int) -> Bool { return now > 10 }\n",
            "sessionExpired",
        ),
        (
            "Sample.java",
            "class Sample { boolean sessionExpired(int now) { return now > 10; } }\n",
            "sessionExpired",
        ),
        (
            "sample.c",
            "int session_expired(int now) { return now > 10; }\n",
            "session_expired",
        ),
        (
            "sample.cpp",
            "bool session_expired(int now) { return now > 10; }\n",
            "session_expired",
        ),
        (
            "Sample.cs",
            "class Sample { bool SessionExpired(int now) { return now > 10; } }\n",
            "SessionExpired",
        ),
        (
            "sample.kt",
            "fun sessionExpired(now: Int): Boolean { return now > 10 }\n",
            "sessionExpired",
        ),
        (
            "sample.rb",
            "def session_expired(now)\n  now > 10\nend\n",
            "session_expired",
        ),
        (
            "sample.php",
            "<?php function session_expired($now) { return $now > 10; }\n",
            "session_expired",
        ),
    ];
    for (filename, code, symbol) in fixtures {
        let root = TempDir::new().unwrap();
        write(root.path(), filename, code);
        let response = AgentService::new()
            .build_context(ContextRequest::new(root.path(), "session expiration"))
            .unwrap();
        let found = response
            .snippets
            .iter()
            .find(|snippet| snippet.symbol.as_deref() == Some(symbol))
            .unwrap_or_else(|| {
                panic!(
                    "{filename}: {}",
                    serde_json::to_string_pretty(&response).unwrap()
                )
            });
        assert_eq!(found.representation, Representation::Full, "{filename}");
        assert!(
            code.contains(found.content.as_deref().unwrap()),
            "{filename}"
        );
        assert_eq!(
            found.content.as_deref().unwrap().lines().count(),
            found.end_line - found.start_line + 1,
            "{filename}"
        );
        assert_budget(&response, 8000);
    }
}

#[test]
fn recipe_exclusions_do_not_reintroduce_tests_or_configuration() {
    let root = project();
    let mut request = ContextRequest::new(root.path(), "session expiration");
    request.recipe = "implementation".into();
    request.custom_recipes.insert(
        request.recipe.clone(),
        RecipeDefinition {
            include_tests: Some(false),
            include_config: Some(false),
            ..RecipeDefinition::default()
        },
    );
    let response = AgentService::new().build_context(request).unwrap();
    assert!(response
        .snippets
        .iter()
        .all(|snippet| !snippet.path.starts_with("tests/")
            && !snippet.path.starts_with("config/")
            && !snippet.path.ends_with("toml")));
    assert!(response
        .snippets
        .iter()
        .all(|snippet| snippet.symbol.as_deref() != Some("unrelated_payment")));
}

#[test]
fn malformed_source_and_unresolved_local_imports_have_explicit_gaps() {
    let root = project();
    write(
        root.path(),
        "src/broken.py",
        "from .does_not_exist import Session\n\ndef session_broken(:\n    return Session(\n",
    );
    let response = AgentService::new()
        .build_context(ContextRequest::new(root.path(), "session broken"))
        .unwrap();
    assert!(response
        .gaps
        .iter()
        .any(|gap| gap.kind == "parse_error" && gap.path.as_deref() == Some("src/broken.py")));
    assert!(response
        .gaps
        .iter()
        .any(|gap| gap.kind == "unresolved_import"));
}

#[test]
fn literal_contracts_reject_comments_dynamic_paths_and_ambiguous_handlers() {
    let root = TempDir::new().unwrap();
    write(root.path(), "client.ts", "export function sessionRefresh() { return fetch('/session/refresh', {method: 'POST'}); }\n");
    write(
        root.path(),
        "handler.py",
        "@app.post('/session/refresh')\ndef refresh_session():\n    return {}\n",
    );
    write(
        root.path(),
        "other.py",
        "@app.post('/session/refresh')\ndef another_service():\n    return {}\n",
    );
    write(
        root.path(),
        "comments.ts",
        "// fetch('/session/refresh', {method: 'POST'});\nexport const unrelated = 5;\n",
    );
    write(root.path(), "dynamic.ts", "export function dynamic() { return fetch(/* '/session/refresh' */ route, {method: 'POST'}); }\n");
    let mut request = ContextRequest::new(root.path(), "");
    request.seeds.push("client.ts".into());
    let response = AgentService::new().build_context(request).unwrap();
    assert!(response
        .gaps
        .iter()
        .any(|gap| gap.kind == "ambiguous_contract"));
    assert!(response
        .relations
        .iter()
        .all(
            |relation| !["comments.ts", "dynamic.ts"].contains(&relation.from.path.as_str())
                && !["comments.ts", "dynamic.ts"].contains(&relation.to.path.as_str())
        ));
}

#[test]
fn concurrent_requests_share_a_revision_checked_session() {
    let root = project();
    let service = std::sync::Arc::new(AgentService::new());
    let mut request = ContextRequest::new(root.path(), "session expiration");
    request.session_id = Some("concurrent".into());
    let first = service.build_context(request.clone()).unwrap();
    request.expected_revision = Some(first.revision);
    let threads = (0..2)
        .map(|_| {
            let service = service.clone();
            let request = request.clone();
            std::thread::spawn(move || service.build_context(request))
        })
        .collect::<Vec<_>>();
    let results = threads
        .into_iter()
        .map(|thread| thread.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|result| result
                .as_ref()
                .is_err_and(|error| error.to_string().contains("conflict")))
            .count(),
        1
    );
}

#[test]
fn reset_and_budget_changes_never_retain_content_the_client_was_not_given() {
    let root = project();
    let service = AgentService::new();
    let mut request = ContextRequest::new(root.path(), "session expiration");
    request.session_id = Some("budget".into());
    let first = service.build_context(request.clone()).unwrap();
    request.expected_revision = Some(first.revision);
    request.max_tokens = 600;
    let small = service.build_context(request.clone()).unwrap();
    assert_budget(&small, 600);
    if small.reset {
        assert!(small
            .snippets
            .iter()
            .all(|snippet| snippet.content.is_some()));
    }
    request.expected_revision = None;
    request.reset_session = true;
    request.max_tokens = 8000;
    let reset = service.build_context(request).unwrap();
    assert!(reset.reset);
    assert!(reset
        .snippets
        .iter()
        .all(|snippet| snippet.content.is_some()));
}

#[test]
fn recipe_cli_and_checkpoint_cli_match_library_contracts() {
    let root = project();
    let external = TempDir::new().unwrap();
    let recipe_file = external.path().join("recipes.toml");
    fs::write(
        &recipe_file,
        "[recipes.auth]\nextends = 'debug_failure'\nmax_snippets = 3\ninclude_tests = false\n",
    )
    .unwrap();
    let catalog = assert_cmd::Command::cargo_bin("context-creator")
        .unwrap()
        .args(["recipes", "--recipe-file"])
        .arg(&recipe_file)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let recipes: Vec<context_creator::agent::Recipe> = serde_json::from_slice(&catalog).unwrap();
    assert!(recipes
        .iter()
        .any(|recipe| recipe.name == "auth" && recipe.max_snippets == 3));
    let state = external.path().join("agent.json");
    let mut command = assert_cmd::Command::cargo_bin("context-creator").unwrap();
    command
        .args(["context", "--root"])
        .arg(root.path())
        .args([
            "--task",
            "session expiration",
            "--session-id",
            "cli",
            "--session-file",
        ])
        .arg(&state);
    let first: ContextResponse =
        serde_json::from_slice(&command.assert().success().get_output().stdout).unwrap();
    command.args(["--expected-revision", &first.revision]);
    let second: ContextResponse =
        serde_json::from_slice(&command.assert().success().get_output().stdout).unwrap();
    assert!(second
        .snippets
        .iter()
        .all(|snippet| snippet.content.is_none()));
}

#[cfg(unix)]
#[test]
fn seeds_cannot_follow_symlinks() {
    let root = project();
    let external = TempDir::new().unwrap();
    write(
        external.path(),
        "outside.py",
        "def session_private(): pass\n",
    );
    std::os::unix::fs::symlink(
        external.path().join("outside.py"),
        root.path().join("linked.py"),
    )
    .unwrap();
    let mut request = ContextRequest::new(root.path(), "session");
    request.seeds.push("linked.py".into());
    assert!(AgentService::new().build_context(request).is_err());
}

#[test]
fn change_context_handles_staged_untracked_deleted_and_renamed_paths() {
    let root = project();
    let repository = git2::Repository::init(root.path()).unwrap();
    let baseline = commit(&repository, "baseline");
    fs::rename(
        root.path().join("src/expiry.py"),
        root.path().join("src/expiration.py"),
    )
    .unwrap();
    fs::remove_file(root.path().join("payments.py")).unwrap();
    write(
        root.path(),
        "tests/test_new_session.py",
        "def test_session_refresh():\n    assert True\n",
    );
    write(
        root.path(),
        "config/session.toml",
        "[session]\nexpiration_seconds = 1200\n",
    );
    let mut index = repository.index().unwrap();
    index.add_path(Path::new("config/session.toml")).unwrap();
    index.write().unwrap();
    let mut request = ContextRequest::new(root.path(), "");
    request.recipe = "review_change".into();
    request.base_ref = Some(baseline.to_string());
    let response = AgentService::new().build_context(request.clone()).unwrap();
    for path in [
        "src/expiration.py",
        "payments.py",
        "tests/test_new_session.py",
        "config/session.toml",
    ] {
        assert!(
            response.changes.iter().any(|change| change.path == path),
            "{path}: {:?}",
            response.changes
        );
    }
    assert!(response
        .changes
        .iter()
        .any(|change| change.status == "renamed"
            && change.old_path.as_deref() == Some("src/expiry.py")));
    request.base_ref = Some("--output=/tmp/evil".into());
    assert!(AgentService::new().build_context(request).is_err());
}

#[test]
fn snapshots_and_sessions_are_isolated_by_repository_policy_and_tokenizer() {
    let root = project();
    let other = project();
    let service = AgentService::new();
    let mut request = ContextRequest::new(root.path(), "session expiration");
    request.session_id = Some("same-name".into());
    let first = service.build_context(request.clone()).unwrap();
    let mut foreign = request.clone();
    foreign.root = other.path().into();
    foreign.expected_revision = Some(first.revision.clone());
    assert!(service.build_context(foreign).is_err());
    let mut foreign = request.clone();
    foreign.secret_scan = context_creator::core::secrets::SecretPolicy::Off;
    foreign.expected_revision = Some(first.revision.clone());
    assert!(service.build_context(foreign).is_err());
    let mut foreign = request;
    foreign.tokenizer = context_creator::core::token::TokenEncoding::Cl100kBase;
    foreign.expected_revision = Some(first.revision);
    assert!(service.build_context(foreign).is_err());
}

#[test]
fn contracts_follow_rust_route_attributes_and_go_clients_without_language_name_matching() {
    let root = TempDir::new().unwrap();
    write(
        root.path(),
        "server.rs",
        "#[post(\"/api/session\")]\nasync fn session_refresh() -> bool { true }\n",
    );
    write(root.path(), "client.go", "package client\nfunc Refresh() { http.Post(\"/api/session\", \"application/json\", nil) }\n");
    let mut request = ContextRequest::new(root.path(), "");
    request.seeds.push("client.go".into());
    let response = AgentService::new().build_context(request).unwrap();
    assert!(
        response
            .relations
            .iter()
            .any(|relation| relation.kind == "http_contract"
                && [relation.from.path.as_str(), relation.to.path.as_str()].contains(&"server.rs")),
        "{}",
        serde_json::to_string_pretty(&response).unwrap()
    );
    let server = response
        .snippets
        .iter()
        .find(|snippet| snippet.symbol.as_deref() == Some("session_refresh"))
        .unwrap();
    assert_eq!(server.start_line, 1);
    assert!(server.content.as_deref().unwrap().starts_with("#[post"));
}

#[test]
fn changed_contract_methods_and_new_dependencies_invalidate_warm_relationships() {
    let root = TempDir::new().unwrap();
    write(
        root.path(),
        "client.ts",
        "export function sessionRefresh() { return fetch('/session', {method: 'POST'}); }\n",
    );
    write(
        root.path(),
        "handler.py",
        "@app.post('/session')\ndef refresh_session():\n    return {}\n",
    );
    let service = AgentService::new();
    let mut request = ContextRequest::new(root.path(), "");
    request.seeds.push("client.ts".into());
    let first = service.build_context(request.clone()).unwrap();
    assert!(first
        .relations
        .iter()
        .any(|relation| relation.kind == "http_contract"));
    write(
        root.path(),
        "handler.py",
        "@app.get('/session')\ndef read_session():\n    return {}\n",
    );
    let second = service.build_context(request).unwrap();
    assert!(!second.index.analysis_reused);
    assert!(!second
        .relations
        .iter()
        .any(|relation| relation.kind == "http_contract"));
}
