use context_creator::core::{secrets::SecretPolicy, token::TokenCounter};
use context_creator::mcp_server::{
    progressive::{read_file_sync, repository_map_sync},
    rmcp_server::ContextCreatorServer,
    ReadFileRequest, ReadFileResponse, RepositoryMapRequest, RepositoryMapResponse,
};
use jsonrpsee::{core::client::ClientT, http_client::HttpClientBuilder, rpc_params};
use rmcp::handler::server::wrapper::Parameters;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

fn read_request(root: &Path, path: &str) -> ReadFileRequest {
    ReadFileRequest {
        root: root.into(),
        path: path.into(),
        start_line: None,
        max_lines: None,
        max_tokens: None,
        tokenizer: None,
        secret_scan: None,
        dependency_tokens: None,
    }
}
fn map_request(root: &Path) -> RepositoryMapRequest {
    RepositoryMapRequest {
        path: root.into(),
        include_patterns: vec![],
        ignore_patterns: vec![],
        max_tokens: Some(256),
        tokenizer: None,
        secret_scan: None,
    }
}

#[tokio::test]
async fn dependency_aware_reads_match_across_both_servers_and_reuse_current_analysis() {
    let project = TempDir::new().unwrap();
    for index in 0..7 {
        let content = if index < 6 {
            format!("const next = @import(\"level{}.zig\");\npub fn step{index}() u32 {{ return next.step{}(); }}\n", index + 1, index + 1)
        } else {
            "pub fn step6() u32 { return 1; }\n".into()
        };
        fs::write(project.path().join(format!("level{index}.zig")), content).unwrap();
    }
    let modern = ContextCreatorServer::new();
    let legacy = context_creator::mcp_server::start_server("127.0.0.1:0")
        .await
        .unwrap();
    let client = HttpClientBuilder::default()
        .build(format!("http://{}", legacy.local_addr().unwrap()))
        .unwrap();
    let mut request = read_request(project.path(), "level0.zig");
    request.dependency_tokens = Some(16000);
    for phase in 0..3 {
        if phase == 2 {
            fs::write(
                project.path().join("level6.zig"),
                "pub fn step6() u32 { return 42; }\n",
            )
            .unwrap();
        }
        let modern_response = modern
            .read_file(Parameters(request.clone()))
            .await
            .unwrap()
            .0;
        let legacy_response: ReadFileResponse = client
            .request("read_file", rpc_params![request.clone()])
            .await
            .unwrap();
        assert_eq!(
            serde_json::to_value(&modern_response).unwrap(),
            serde_json::to_value(&legacy_response).unwrap()
        );
        let context = modern_response.dependency_context.unwrap();
        assert_eq!(
            context
                .snippets
                .iter()
                .map(|snippet| &snippet.path)
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            7
        );
        assert!(context.token_count <= 16000);
        if phase == 1 {
            assert!(context.index.analysis_reused);
            assert_eq!(context.index.declarations_reparsed, 0);
        }
        if phase == 2 {
            assert_eq!(context.index.declarations_reparsed, 1);
            assert!(context
                .snippets
                .iter()
                .any(|snippet| snippet.path == "level6.zig"
                    && snippet
                        .content
                        .as_ref()
                        .is_some_and(|content| content.contains("return 42"))));
        }
    }
    legacy.stop().unwrap();
}

#[test]
fn dependency_reads_validate_budgets_and_apply_secret_policy_to_dependencies() {
    let project = TempDir::new().unwrap();
    fs::write(
        project.path().join("main.zig"),
        "const next = @import(\"private.zig\");\npub fn run() void {}\n",
    )
    .unwrap();
    let secret = format!("ghp_{}", "z".repeat(36));
    fs::write(
        project.path().join("private.zig"),
        format!("// {secret}\npub fn value() u32 {{ return 1; }}\n"),
    )
    .unwrap();
    let mut request = read_request(project.path(), "main.zig");
    for budget in [0, 255, 65537] {
        request.dependency_tokens = Some(budget);
        assert!(read_file_sync(request.clone())
            .unwrap_err()
            .to_string()
            .contains("dependency_tokens"));
    }
    request.dependency_tokens = Some(8000);
    let response = read_file_sync(request.clone()).unwrap();
    assert!(!serde_json::to_string(&response).unwrap().contains(&secret));
    assert!(response
        .dependency_context
        .unwrap()
        .snippets
        .iter()
        .any(|snippet| snippet
            .content
            .as_ref()
            .is_some_and(|content| content.contains("REDACTED"))));
    request.secret_scan = Some(SecretPolicy::Error);
    assert!(read_file_sync(request)
        .unwrap_err()
        .to_string()
        .contains("Secret detected"));
}

#[test]
fn paginated_reads_preserve_every_source_byte_and_make_progress_under_budget() {
    let project = TempDir::new().unwrap();
    let source = (1..=80)
        .map(|line| format!("// line {line}: 你好 世界\n"))
        .collect::<String>();
    fs::write(project.path().join("main.rs"), &source).unwrap();
    let mut reconstructed = String::new();
    let mut start = 1;
    loop {
        let mut request = read_request(project.path(), "main.rs");
        request.start_line = Some(start);
        request.max_lines = Some(7);
        request.max_tokens = Some(35);
        let page = read_file_sync(request).unwrap();
        assert!(page.token_count <= 35);
        assert_eq!(
            page.token_count,
            TokenCounter::new()
                .unwrap()
                .count_tokens(&page.content)
                .unwrap()
        );
        reconstructed.push_str(&page.content);
        match page.next_line {
            Some(next) => {
                assert!(next > start);
                start = next;
            }
            None => break,
        }
    }
    assert_eq!(reconstructed, source);
    fs::write(project.path().join("main.rs"), "changed\n").unwrap();
    assert_eq!(
        read_file_sync(read_request(project.path(), "main.rs"))
            .unwrap()
            .content,
        "changed\n"
    );
}

#[test]
fn reads_validate_ranges_limits_hidden_paths_and_ignore_files() {
    let project = TempDir::new().unwrap();
    fs::create_dir(project.path().join("src")).unwrap();
    fs::write(project.path().join("src/main.rs"), "pub fn run() {}\n").unwrap();
    fs::write(project.path().join("secret.txt"), "secret\n").unwrap();
    fs::write(project.path().join(".env"), "private\n").unwrap();
    fs::write(project.path().join(".contextignore"), "secret.txt\n").unwrap();
    fs::write(project.path().join("src/.gitignore"), "main.rs\n").unwrap();
    for path in ["../escape", ".env", "secret.txt", "src/main.rs"] {
        assert!(
            read_file_sync(read_request(project.path(), path)).is_err(),
            "{path}"
        );
    }
    fs::write(project.path().join("normal.rs"), "pub fn run() {}\n").unwrap();
    for (start, lines, tokens) in [
        (0, 10, 100),
        (1, 0, 100),
        (1, 1001, 100),
        (1, 10, 0),
        (1, 10, 32769),
        (1, 10, 1),
    ] {
        let mut request = read_request(project.path(), "normal.rs");
        request.start_line = Some(start);
        request.max_lines = Some(lines);
        request.max_tokens = Some(tokens);
        assert!(read_file_sync(request).is_err());
    }
    let mut beyond = read_request(project.path(), "normal.rs");
    beyond.start_line = Some(100);
    let page = read_file_sync(beyond).unwrap();
    assert!(page.content.is_empty());
    assert!(page.end_line.is_none());
    assert!(page.next_line.is_none());
}

#[test]
fn a_child_negation_cannot_bypass_an_ignored_parent_directory() {
    let project = TempDir::new().unwrap();
    fs::create_dir(project.path().join("blocked")).unwrap();
    fs::write(project.path().join("blocked/visible.rs"), "source\n").unwrap();
    fs::write(
        project.path().join(".gitignore"),
        "blocked/\n!blocked/visible.rs\n",
    )
    .unwrap();
    assert!(read_file_sync(read_request(project.path(), "blocked/visible.rs")).is_err());
    fs::write(
        project.path().join(".gitignore"),
        "blocked/*\n!blocked/visible.rs\n",
    )
    .unwrap();
    assert!(read_file_sync(read_request(project.path(), "blocked/visible.rs")).is_ok());
}

#[cfg(unix)]
#[test]
fn reads_deny_inside_and_outside_symlinks_including_directory_aliases() {
    let project = TempDir::new().unwrap();
    let outside = TempDir::new().unwrap();
    fs::write(project.path().join("main.rs"), "inside\n").unwrap();
    fs::write(outside.path().join("main.rs"), "outside\n").unwrap();
    std::os::unix::fs::symlink(
        project.path().join("main.rs"),
        project.path().join("alias.rs"),
    )
    .unwrap();
    std::os::unix::fs::symlink(outside.path(), project.path().join("external")).unwrap();
    for path in ["alias.rs", "external/main.rs"] {
        assert!(read_file_sync(read_request(project.path(), path)).is_err());
    }
}

#[test]
fn reads_redact_by_default_and_scan_before_returning_any_range() {
    let project = TempDir::new().unwrap();
    let secret = format!("ghp_{}", "a".repeat(36));
    fs::write(
        project.path().join("main.rs"),
        format!("pub fn run() {{}}\n// {secret}\n"),
    )
    .unwrap();
    let page = read_file_sync(read_request(project.path(), "main.rs")).unwrap();
    assert!(!page.content.contains(&secret));
    assert!(page.content.contains("REDACTED"));
    let mut strict = read_request(project.path(), "main.rs");
    strict.secret_scan = Some(SecretPolicy::Error);
    strict.max_lines = Some(1);
    let error = read_file_sync(strict).unwrap_err().to_string();
    assert!(error.contains("Secret detected"));
    assert!(!error.contains(&secret));
}

#[tokio::test]
async fn both_mcp_servers_expose_the_same_bounded_exploration_contract() {
    let project = TempDir::new().unwrap();
    fs::write(
        project.path().join("main.rs"),
        "pub fn authenticate() -> bool { true }\n",
    )
    .unwrap();
    let request = map_request(project.path());
    let expected = repository_map_sync(request.clone()).unwrap();
    assert!(expected.map.contains("authenticate() -> bool"));
    assert!(!expected.map.contains("true"));
    assert!(expected.token_count <= 256);
    let rmcp = ContextCreatorServer::new();
    let actual = rmcp
        .repository_map(Parameters(request.clone()))
        .await
        .unwrap()
        .0;
    assert_eq!(expected.map, actual.map);
    let page = rmcp
        .read_file(Parameters(read_request(project.path(), "main.rs")))
        .await
        .unwrap()
        .0;
    assert!(page.content.contains("true"));

    let server = context_creator::mcp_server::start_server("127.0.0.1:0")
        .await
        .unwrap();
    let client = HttpClientBuilder::default()
        .build(format!("http://{}", server.local_addr().unwrap()))
        .unwrap();
    let actual: RepositoryMapResponse = client
        .request("repository_map", rpc_params![request])
        .await
        .unwrap();
    assert_eq!(actual.map, expected.map);
    let page: ReadFileResponse = client
        .request(
            "read_file",
            rpc_params![read_request(project.path(), "main.rs")],
        )
        .await
        .unwrap();
    assert!(page.content.contains("true"));
    let denied = client
        .request::<ReadFileResponse, _>(
            "read_file",
            rpc_params![read_request(project.path(), "../escape")],
        )
        .await;
    assert!(denied.is_err());
    server.stop().unwrap();
}

#[test]
fn map_ranks_shared_dependencies_before_unrelated_files() {
    let project = TempDir::new().unwrap();
    fs::create_dir(project.path().join(".git")).unwrap();
    fs::write(
        project.path().join("a_unrelated.py"),
        "def unrelated(): pass\n",
    )
    .unwrap();
    fs::write(
        project.path().join("z_shared.py"),
        "def authenticate(): return True\n",
    )
    .unwrap();
    fs::write(
        project.path().join("client.py"),
        "from z_shared import authenticate\ndef run(): return authenticate()\n",
    )
    .unwrap();
    let map = repository_map_sync(map_request(project.path()))
        .unwrap()
        .map;
    assert!(
        map.find("z_shared.py").unwrap() < map.find("a_unrelated.py").unwrap(),
        "{map}"
    );
}

#[test]
fn reads_reject_binary_and_oversized_files_without_returning_partial_content() {
    let project = TempDir::new().unwrap();
    fs::write(project.path().join("binary.txt"), b"some\0binary\0content").unwrap();
    fs::File::create(project.path().join("large.txt"))
        .unwrap()
        .set_len(10 * 1024 * 1024 + 1)
        .unwrap();
    for path in ["binary.txt", "large.txt"] {
        assert!(read_file_sync(read_request(project.path(), path)).is_err());
    }
}

#[tokio::test]
async fn analyze_cache_distinguishes_secret_policies() {
    let project = TempDir::new().unwrap();
    let secret = format!("ghp_{}", "a".repeat(36));
    fs::write(
        project.path().join("main.rs"),
        format!("// {secret}\npub fn run() {{}}\n"),
    )
    .unwrap();
    let server = ContextCreatorServer::new();
    for (policy, visible) in [("off", true), ("redact", false)] {
        let request = serde_json::from_value(serde_json::json!({
            "path": project.path(), "include_context": true, "secret_scan": policy
        }))
        .unwrap();
        let result = server.analyze_local(Parameters(request)).await.unwrap().0;
        assert_eq!(result.context.unwrap().contains(&secret), visible);
    }
}

#[tokio::test]
async fn task_context_and_recipes_match_across_legacy_and_rmcp_servers() {
    use context_creator::agent::{ContextRequest, ContextResponse, RecipeBook, RecipeDefinition};
    let root = TempDir::new().unwrap();
    fs::write(
        root.path().join("expiry.rs"),
        "pub fn session_expired(now: u64) -> bool { now > 10 }\n",
    )
    .unwrap();
    let mut book = RecipeBook::default();
    book.recipes.insert(
        "auth".into(),
        RecipeDefinition {
            max_snippets: Some(2),
            ..RecipeDefinition::default()
        },
    );
    let expected = context_creator::agent::list_recipes(&book).unwrap();
    let rmcp = ContextCreatorServer::new();
    let catalog = rmcp.list_recipes(Parameters(book.clone())).await.unwrap().0;
    assert_eq!(
        serde_json::to_value(&catalog).unwrap(),
        serde_json::to_value(&expected).unwrap()
    );
    let legacy = context_creator::mcp_server::start_server("127.0.0.1:0")
        .await
        .unwrap();
    let client = HttpClientBuilder::default()
        .build(format!("http://{}", legacy.local_addr().unwrap()))
        .unwrap();
    let catalog: Vec<context_creator::agent::Recipe> = client
        .request("list_recipes", rpc_params![book.clone()])
        .await
        .unwrap();
    assert_eq!(
        serde_json::to_value(catalog).unwrap(),
        serde_json::to_value(&expected).unwrap()
    );
    let mut request = ContextRequest::new(root.path(), "session expiration");
    request.recipe = "auth".into();
    request.custom_recipes = book.recipes;
    request.session_id = Some("mcp".into());
    let modern = rmcp
        .build_context(Parameters(request.clone()))
        .await
        .unwrap()
        .0;
    let old: ContextResponse = client
        .request("build_context", rpc_params![request.clone()])
        .await
        .unwrap();
    assert_eq!(
        serde_json::to_value(&modern).unwrap(),
        serde_json::to_value(&old).unwrap()
    );
    request.expected_revision = Some(modern.revision);
    let modern_delta = rmcp
        .build_context(Parameters(request.clone()))
        .await
        .unwrap()
        .0;
    let old_delta: ContextResponse = client
        .request("build_context", rpc_params![request.clone()])
        .await
        .unwrap();
    assert_eq!(
        serde_json::to_value(&modern_delta).unwrap(),
        serde_json::to_value(&old_delta).unwrap()
    );
    assert!(modern_delta
        .snippets
        .iter()
        .all(|snippet| snippet.content.is_none()));
    assert!(rmcp
        .build_context(Parameters(request.clone()))
        .await
        .is_err());
    assert!(client
        .request::<ContextResponse, _>("build_context", rpc_params![request])
        .await
        .is_err());
    legacy.stop().unwrap();
}

#[tokio::test]
async fn local_analysis_refreshes_edits_additions_ignores_deletions_and_secret_checks() {
    use context_creator::mcp_server::{ProcessLocalRequest, ProcessLocalResponse};
    let root = TempDir::new().unwrap();
    let modern = ContextCreatorServer::new();
    let legacy = context_creator::mcp_server::start_server("127.0.0.1:0")
        .await
        .unwrap();
    let client = HttpClientBuilder::default()
        .build(format!("http://{}", legacy.local_addr().unwrap()))
        .unwrap();
    let request: ProcessLocalRequest = serde_json::from_value(
        serde_json::json!({"path":root.path(),"include_context":true,"secret_scan":"error"}),
    )
    .unwrap();
    for (step, visible, absent, count) in [
        (0, "before_edit", "after_edit", 1),
        (1, "after_edit", "before_edit", 1),
        (2, "new_source", "before_edit", 2),
        (3, "after_edit", "new_source", 1),
        (4, "replacement_source", "after_edit", 1),
    ] {
        match step {
            0 => fs::write(root.path().join("main.rs"), "pub fn before_edit() {}\n").unwrap(),
            1 => fs::write(root.path().join("main.rs"), "pub fn after_edit() {}\n").unwrap(),
            2 => fs::write(root.path().join("added.rs"), "pub fn new_source() {}\n").unwrap(),
            3 => fs::write(root.path().join(".contextignore"), "added.rs\n").unwrap(),
            4 => {
                fs::remove_file(root.path().join("main.rs")).unwrap();
                fs::write(
                    root.path().join("replacement.rs"),
                    "pub fn replacement_source() {}\n",
                )
                .unwrap();
            }
            _ => unreachable!(),
        }
        let modern_response = modern
            .analyze_local(Parameters(request.clone()))
            .await
            .unwrap()
            .0;
        let legacy_response: ProcessLocalResponse = client
            .request("process_local_codebase", rpc_params![request.clone()])
            .await
            .unwrap();
        for response in [modern_response, legacy_response] {
            let content = response.context.unwrap();
            assert!(content.contains(visible), "step {step}: missing {visible}");
            assert!(!content.contains(absent), "step {step}: stale {absent}");
            assert_eq!(response.file_count, count, "step {step}");
        }
    }
    fs::write(
        root.path().join("replacement.rs"),
        format!(
            "// {}{}\npub fn replacement_source() {{}}\n",
            "ghp_",
            "c".repeat(36)
        ),
    )
    .unwrap();
    assert!(modern
        .analyze_local(Parameters(request.clone()))
        .await
        .err()
        .unwrap()
        .to_string()
        .contains("Secret detected"));
    let result: Result<ProcessLocalResponse, _> = client
        .request("process_local_codebase", rpc_params![request])
        .await;
    assert!(result.unwrap_err().to_string().contains("Secret detected"));
    legacy.stop().unwrap();
}

#[tokio::test]
async fn external_version_gaps_refresh_after_lock_edits_across_both_servers() {
    use context_creator::agent::{ContextRequest, ContextResponse};
    let root = TempDir::new().unwrap();
    fs::write(
        root.path().join("Cargo.toml"),
        "[package]\nname='app'\nversion='0.1.0'\n[dependencies]\nserde='1.0'\n",
    )
    .unwrap();
    fs::write(
        root.path().join("lib.rs"),
        "use serde::Serialize;\npub fn output<T: Serialize>(value: T) {}\n",
    )
    .unwrap();
    let modern = ContextCreatorServer::new();
    let legacy = context_creator::mcp_server::start_server("127.0.0.1:0")
        .await
        .unwrap();
    let client = HttpClientBuilder::default()
        .build(format!("http://{}", legacy.local_addr().unwrap()))
        .unwrap();
    let mut request = ContextRequest::new(root.path(), "output");
    request.seeds.push("lib.rs".into());
    request.session_id = Some("versions".into());
    let mut previous_revision = None;
    for version in ["1.0.197", "1.0.198"] {
        fs::write(
            root.path().join("Cargo.lock"),
            format!("[[package]]\nname='serde'\nversion='{version}'\n"),
        )
        .unwrap();
        request.expected_revision = previous_revision.clone();
        let result = modern
            .build_context(Parameters(request.clone()))
            .await
            .unwrap()
            .0;
        let old: ContextResponse = client
            .request("build_context", rpc_params![request.clone()])
            .await
            .unwrap();
        assert_eq!(
            serde_json::to_value(&result).unwrap(),
            serde_json::to_value(old).unwrap()
        );
        assert!(result
            .gaps
            .iter()
            .any(|gap| gap.kind == "external_type"
                && gap.detail.contains(&format!("serde v{version}"))));
        assert!(result.token_count <= 8000);
        if let Some(previous) = &previous_revision {
            assert_ne!(&result.revision, previous);
            assert!(result.stale);
        }
        previous_revision = Some(result.revision);
    }
    legacy.stop().unwrap();
}
