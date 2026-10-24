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
