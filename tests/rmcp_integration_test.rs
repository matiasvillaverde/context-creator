//! Integration tests for RMCP MCP server

use anyhow::{bail, Context, Result};
use serde_json::json;
use serde_json::Value;
use std::net::TcpListener;
use std::process::Stdio;
use std::time::Duration;
use tempfile::TempDir;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

struct RmcpTestClient {
    child: Child,
    writer: ChildStdin,
    reader: BufReader<ChildStdout>,
}

impl RmcpTestClient {
    async fn spawn() -> Result<Self> {
        let binary = std::env::var("CARGO_BIN_EXE_context-creator")
            .unwrap_or_else(|_| "./target/debug/context-creator".to_string());

        Self::spawn_command(Command::new(binary)).await
    }

    async fn spawn_command(mut command: Command) -> Result<Self> {
        let mut child = command
            .arg("--rmcp")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .context("failed to spawn RMCP server")?;

        let writer = child.stdin.take().context("RMCP stdin was not piped")?;
        let stdout = child.stdout.take().context("RMCP stdout was not piped")?;

        Ok(Self {
            child,
            writer,
            reader: BufReader::new(stdout),
        })
    }

    async fn initialize(&mut self) -> Result<Value> {
        let response = self
            .request(
                1,
                "initialize",
                json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {},
                    "clientInfo": {
                        "name": "context-creator-rmcp-test",
                        "version": "0.0.0"
                    }
                }),
            )
            .await?;

        self.notify("notifications/initialized").await?;

        Ok(response)
    }

    async fn request(&mut self, id: u64, method: &str, params: Value) -> Result<Value> {
        let request = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });

        self.write_message(&request).await?;
        let response = self.read_message().await?;

        if response.get("error").is_some() {
            bail!("RMCP request {method} failed: {response}");
        }

        Ok(response)
    }

    async fn notify(&mut self, method: &str) -> Result<()> {
        let notification = json!({
            "jsonrpc": "2.0",
            "method": method,
        });

        self.write_message(&notification).await
    }

    async fn write_message(&mut self, message: &Value) -> Result<()> {
        self.writer
            .write_all(format!("{message}\n").as_bytes())
            .await
            .context("failed to write RMCP message")?;
        self.writer
            .flush()
            .await
            .context("failed to flush RMCP message")
    }

    async fn read_message(&mut self) -> Result<Value> {
        let mut line = String::new();
        let bytes_read =
            tokio::time::timeout(Duration::from_secs(30), self.reader.read_line(&mut line))
                .await
                .context("timed out waiting for RMCP response")?
                .context("failed to read RMCP response")?;

        if bytes_read == 0 {
            bail!("RMCP server closed stdout before sending a response");
        }

        serde_json::from_str(&line).with_context(|| format!("invalid RMCP JSON: {line}"))
    }

    async fn shutdown(mut self) -> Result<()> {
        self.child
            .kill()
            .await
            .context("failed to stop RMCP server")
    }
}

impl Drop for RmcpTestClient {
    fn drop(&mut self) {
        let _ = self.child.start_kill();
    }
}

struct RmcpHttpServer {
    child: Child,
    port: u16,
}

impl RmcpHttpServer {
    async fn spawn() -> Result<Self> {
        let binary = std::env::var("CARGO_BIN_EXE_context-creator")
            .unwrap_or_else(|_| "./target/debug/context-creator".to_string());
        let port = free_port()?;

        let child = Command::new(binary)
            .arg("--rmcp")
            .arg("--rmcp-transport")
            .arg("http")
            .arg("--mcp-port")
            .arg(port.to_string())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .context("failed to spawn RMCP HTTP server")?;

        let server = Self { child, port };
        wait_for_port(port).await?;
        Ok(server)
    }

    async fn shutdown(mut self) -> Result<()> {
        self.child
            .kill()
            .await
            .context("failed to stop RMCP HTTP server")
    }
}

impl Drop for RmcpHttpServer {
    fn drop(&mut self) {
        let _ = self.child.start_kill();
    }
}

fn free_port() -> Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    Ok(listener.local_addr()?.port())
}

async fn wait_for_port(port: u16) -> Result<()> {
    for _ in 0..50 {
        if TcpStream::connect(("127.0.0.1", port)).await.is_ok() {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    bail!("RMCP HTTP server did not listen on port {port}");
}

fn modern_params(mut params: Value) -> Value {
    params["_meta"] = json!({
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        "io.modelcontextprotocol/clientCapabilities": {}
    });
    params
}

async fn post_json_rpc(
    port: u16,
    message: Value,
    host: Option<&str>,
    origin: Option<&str>,
) -> Result<(u16, String)> {
    let stream = TcpStream::connect(("127.0.0.1", port)).await?;
    let (mut sender, connection) =
        hyper::client::conn::http1::handshake(hyper_util::rt::TokioIo::new(stream)).await?;
    tokio::spawn(async move {
        let _ = connection.await;
    });
    let mut request = hyper::Request::builder()
        .method("POST")
        .uri("/mcp")
        .header(
            "Host",
            host.map(str::to_string)
                .unwrap_or_else(|| format!("127.0.0.1:{port}")),
        )
        .header("Accept", "application/json, text/event-stream")
        .header("Content-Type", "application/json")
        .header("Mcp-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", message["method"].as_str().unwrap_or_default());
    if let Some(name) = message["params"]["name"].as_str() {
        request = request.header("Mcp-Name", name);
    }
    if let Some(origin) = origin {
        request = request.header("Origin", origin);
    }
    let response = tokio::time::timeout(
        Duration::from_secs(30),
        sender.send_request(request.body(axum::body::Body::from(message.to_string()))?),
    )
    .await??;
    let status = response.status().as_u16();
    let body = tokio::time::timeout(
        Duration::from_secs(30),
        axum::body::to_bytes(axum::body::Body::new(response.into_body()), 4 * 1024 * 1024),
    )
    .await??;
    Ok((status, String::from_utf8(body.to_vec())?))
}

#[tokio::test]
async fn test_rmcp_server_initialization() -> Result<()> {
    let mut client = RmcpTestClient::spawn().await?;
    let response = client.initialize().await?;

    // Verify response
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 1);
    assert!(response["result"]["capabilities"].is_object());
    assert_eq!(response["result"]["protocolVersion"], "2024-11-05");

    client.shutdown().await?;

    Ok(())
}

#[tokio::test]
async fn test_analyze_local_tool() -> Result<()> {
    // Create a test directory with some code
    let temp_dir = TempDir::new()?;
    std::fs::write(
        temp_dir.path().join("test.rs"),
        r#"
fn main() {
    println!("Hello, world!");
}

fn add(a: i32, b: i32) -> i32 {
    a + b
}
"#,
    )?;

    let mut client = RmcpTestClient::spawn().await?;
    client.initialize().await?;

    // Call analyze_local tool
    let response = client
        .request(
            2,
            "tools/call",
            json!({
                "name": "analyze_local",
                "arguments": {
                    "prompt": "",
                    "path": temp_dir.path(),
                "max_tokens": 10000,
                    "include_context": true
                }
            }),
        )
        .await?;

    let content = response["result"]["content"]
        .as_array()
        .context("tools/call response did not include content")?;
    let text = content
        .first()
        .and_then(|item| item["text"].as_str())
        .context("tools/call response did not include JSON text content")?;
    let payload: Value = serde_json::from_str(text)?;

    // Verify response contains context and metadata without requiring an external LLM.
    assert_eq!(payload["answer"], "");
    assert!(payload["file_count"].as_u64().unwrap_or_default() >= 1);
    assert!(payload["token_count"].as_u64().unwrap_or_default() > 0);
    assert!(payload["context"]
        .as_str()
        .unwrap_or_default()
        .contains("fn add"));
    assert!(payload["markdown"]
        .as_str()
        .unwrap_or_default()
        .contains("fn add"));

    client.shutdown().await?;

    Ok(())
}

#[tokio::test]
async fn test_list_tools() -> Result<()> {
    let mut client = RmcpTestClient::spawn().await?;
    client.initialize().await?;

    // List tools
    let response = client.request(2, "tools/list", json!({})).await?;

    // Verify tools are listed
    let tools = response["result"]["tools"].as_array().unwrap();
    assert!(tools.len() >= 6); // We should have at least 6 tools

    // Check that expected tools are present
    let tool_names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();

    assert!(tool_names.contains(&"analyze_local"));
    assert!(tool_names.contains(&"analyze_remote"));
    assert!(tool_names.contains(&"file_metadata"));
    assert!(tool_names.contains(&"search"));
    assert!(tool_names.contains(&"diff"));
    assert!(tool_names.contains(&"semantic_search"));

    client.shutdown().await?;

    Ok(())
}

#[tokio::test]
async fn test_rmcp_http_streamable_list_tools_and_call() -> Result<()> {
    let server = RmcpHttpServer::spawn().await?;
    let (status, body) = post_json_rpc(
        server.port,
        json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/list",
            "params": modern_params(json!({}))
        }),
        None,
        None,
    )
    .await?;
    assert_eq!(status, 200, "{body}");
    let response: Value = serde_json::from_str(&body)?;
    assert_eq!(response["result"]["resultType"], "complete");
    let tools = response["result"]["tools"]
        .as_array()
        .context("missing tools")?;
    assert!(tools.iter().any(|tool| tool["name"] == "analyze_local"));
    assert!(tools.iter().any(|tool| tool["name"] == "semantic_search"));
    assert!(tools.iter().any(|tool| tool["name"] == "repository_map"));
    assert!(tools.iter().any(|tool| tool["name"] == "read_file"));

    let project = TempDir::new()?;
    std::fs::write(
        project.path().join("main.rs"),
        "fn main() { println!(\"HTTP context\"); }",
    )?;
    let (status, body) = post_json_rpc(
        server.port,
        json!({
            "jsonrpc": "2.0", "id": 2, "method": "tools/call",
            "params": modern_params(json!({ "name": "analyze_local", "arguments": {
                "path": project.path(), "prompt": "", "include_context": true
            }}))
        }),
        None,
        None,
    )
    .await?;
    assert_eq!(status, 200, "{body}");
    let response: Value = serde_json::from_str(&body)?;
    assert!(response.get("error").is_none(), "{response}");
    assert!(body.contains("HTTP context"), "{body}");
    for (id, name, arguments) in [
        (
            3,
            "repository_map",
            json!({"path": project.path(), "max_tokens": 128}),
        ),
        (
            4,
            "read_file",
            json!({"root": project.path(), "path": "main.rs", "max_lines": 1}),
        ),
    ] {
        let (status, body) = post_json_rpc(
            server.port,
            json!({
                "jsonrpc": "2.0", "id": id, "method": "tools/call",
                "params": modern_params(json!({"name": name, "arguments": arguments}))
            }),
            None,
            None,
        )
        .await?;
        assert_eq!(status, 200, "{body}");
        let response: Value = serde_json::from_str(&body)?;
        assert!(response.get("error").is_none(), "{response}");
        assert_ne!(response["result"]["isError"], true, "{response}");
        assert!(body.contains("main.rs"), "{body}");
    }
    server.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn test_rmcp_http_rejects_untrusted_hosts_and_origins() -> Result<()> {
    let server = RmcpHttpServer::spawn().await?;
    for (host, origin) in [
        (Some("attacker.example"), None),
        (None, Some("https://attacker.example")),
    ] {
        let (status, _) = post_json_rpc(
            server.port,
            json!({
                "jsonrpc": "2.0", "id": 1, "method": "tools/list",
                "params": modern_params(json!({}))
            }),
            host,
            origin,
        )
        .await?;
        assert_eq!(status, 403, "host={host:?} origin={origin:?}");
    }
    server.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn test_rmcp_stdio_supports_stateless_discovery_and_tool_listing() -> Result<()> {
    let mut client = RmcpTestClient::spawn().await?;
    let discovery = client
        .request(1, "server/discover", modern_params(json!({})))
        .await?;
    assert!(discovery.get("error").is_none(), "{discovery}");
    let response = client
        .request(2, "tools/list", modern_params(json!({})))
        .await?;
    assert!(response["result"]["tools"]
        .as_array()
        .is_some_and(|tools| tools.len() >= 6));
    client.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn test_rmcp_ignores_context_generation_config() -> Result<()> {
    let project = TempDir::new()?;
    std::fs::write(
        project.path().join(".context-creator.toml"),
        "invalid TOML = [",
    )?;
    let mut command = Command::new(env!("CARGO_BIN_EXE_context-creator"));
    command.current_dir(project.path());
    let mut client = RmcpTestClient::spawn_command(command).await?;
    client.initialize().await?;
    client.shutdown().await?;
    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn test_rmcp_provider_models_headless_arguments_and_single_prompt_reservation() -> Result<()>
{
    use std::os::unix::fs::PermissionsExt;
    let project = TempDir::new()?;
    let tools = TempDir::new()?;
    let logs = TempDir::new()?;
    std::fs::write(
        project.path().join("main.rs"),
        "fn main() {}\n".to_string() + &"// context row\n".repeat(80),
    )?;
    std::fs::write(project.path().join("notes.md"), "## An embedded heading\n")?;
    for tool in ["gemini", "codex", "claude", "ollama"] {
        let path = tools.path().join(tool);
        std::fs::write(
            &path,
            format!(
                r#"#!/bin/sh
set -eu
printf '%s\n' "$@" > "$CONTEXT_CREATOR_MOCK_LOG_DIR/{tool}.args"
cat > "$CONTEXT_CREATOR_MOCK_LOG_DIR/{tool}.stdin"
printf '{tool} response\n'
"#
            ),
        )?;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))?;
    }
    let binary = env!("CARGO_BIN_EXE_context-creator");
    let mut command = Command::new(binary);
    let path = std::env::join_paths(std::iter::once(tools.path().to_path_buf()).chain(
        std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()),
    ))?;
    command
        .env("PATH", path)
        .env("CONTEXT_CREATOR_MOCK_LOG_DIR", logs.path());
    let mut client = RmcpTestClient::spawn_command(command).await?;
    client.initialize().await?;
    for (index, tool) in ["gemini", "codex", "claude", "ollama"]
        .into_iter()
        .enumerate()
    {
        let response = client
            .request(
                index as u64 + 2,
                "tools/call",
                json!({
                    "name": "analyze_local", "arguments": {
                        "path": project.path(), "prompt": "Explain", "llm_tool": tool,
                        "model": "test-current-model", "tokenizer": "cl100k-base",
                        "max_tokens": 2000, "include_context": true
                    }
                }),
            )
            .await?;
        let text = response["result"]["content"][0]["text"]
            .as_str()
            .context("missing tool text")?;
        let payload: Value = serde_json::from_str(text)?;
        assert_eq!(payload["answer"], format!("{tool} response\n"));
        assert_eq!(payload["file_count"], 2, "{payload}");
        let args = std::fs::read_to_string(logs.path().join(format!("{tool}.args")))?;
        let stdin = std::fs::read_to_string(logs.path().join(format!("{tool}.stdin")))?;
        assert!(args.contains("test-current-model"), "{args}");
        assert!(stdin.contains("fn main()"), "{stdin}");
        match tool {
            "codex" => assert!(args.starts_with("exec\n--skip-git-repo-check\n")),
            "gemini" => assert!(args.starts_with("--prompt\nExplain\n")),
            "claude" => assert!(args.starts_with("-p\nExplain\n")),
            "ollama" => assert!(args.starts_with("run\ntest-current-model\n")),
            _ => unreachable!(),
        }
    }
    client.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn task_context_tools_are_discoverable_and_sessions_work_over_stdio() -> Result<()> {
    let project = TempDir::new()?;
    std::fs::write(
        project.path().join("session.rs"),
        "pub fn session_expired(now: u64) -> bool { now > 10 }\n",
    )?;
    let mut client = RmcpTestClient::spawn().await?;
    client.initialize().await?;
    let tools = client.request(2, "tools/list", json!({})).await?;
    let tools = tools["result"]["tools"]
        .as_array()
        .context("missing tools")?;
    for name in ["build_context", "list_recipes"] {
        assert!(tools.iter().any(|tool| tool["name"] == name));
    }
    let catalog = client
        .request(
            3,
            "tools/call",
            json!({"name":"list_recipes", "arguments":{}}),
        )
        .await?;
    assert_ne!(catalog["result"]["isError"], true);
    let response = client.request(4, "tools/call", json!({"name":"build_context", "arguments":{"root":project.path(), "task":"session expiration", "session_id":"stdio-agent"}})).await?;
    let content = response["result"]["content"][0]["text"]
        .as_str()
        .context("missing context")?;
    let first: context_creator::agent::ContextResponse = serde_json::from_str(content)?;
    assert!(first
        .snippets
        .iter()
        .any(|snippet| snippet.symbol.as_deref() == Some("session_expired")));
    let response = client.request(5, "tools/call", json!({"name":"build_context", "arguments":{"root":project.path(), "task":"session expiration", "session_id":"stdio-agent", "expected_revision":first.revision}})).await?;
    let content = response["result"]["content"][0]["text"]
        .as_str()
        .context("missing delta")?;
    let delta: context_creator::agent::ContextResponse = serde_json::from_str(content)?;
    assert!(delta
        .snippets
        .iter()
        .all(|snippet| snippet.content.is_none()));
    assert!(delta.index.analysis_reused);
    client.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn analyze_local_returns_current_source_after_an_edit_over_stdio() -> Result<()> {
    let project = TempDir::new()?;
    let source = project.path().join("session.rs");
    let mut client = RmcpTestClient::spawn().await?;
    client.initialize().await?;
    for (id, name) in [(2, "before_edit"), (3, "after_edit")] {
        std::fs::write(&source, format!("pub fn {name}() {{}}\n"))?;
        let response = client.request(id, "tools/call", json!({
            "name": "analyze_local", "arguments": {"path":project.path(), "include_context":true}
        })).await?;
        let text = response["result"]["content"][0]["text"]
            .as_str()
            .context("missing local analysis")?;
        let payload: context_creator::mcp_server::ProcessLocalResponse =
            serde_json::from_str(text)?;
        assert!(payload
            .context
            .as_ref()
            .is_some_and(|context| context.contains(name)));
        if id == 3 {
            assert!(!payload
                .context
                .as_ref()
                .is_some_and(|context| context.contains("before_edit")));
        }
    }
    client.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn modern_stdio_reads_discoverable_transitive_context_and_refreshes_deep_edits() -> Result<()>
{
    let project = TempDir::new()?;
    for index in 0..7 {
        let code = if index < 6 {
            format!("const next = @import(\"level{}.zig\");\npub fn step{index}() u32 {{ return next.step{}(); }}\n", index + 1, index + 1)
        } else {
            "pub fn step6() u32 { return 1; }\n".into()
        };
        std::fs::write(project.path().join(format!("level{index}.zig")), code)?;
    }
    let mut client = RmcpTestClient::spawn().await?;
    client
        .request(1, "server/discover", modern_params(json!({})))
        .await?;
    let tools = client
        .request(2, "tools/list", modern_params(json!({})))
        .await?;
    let read = tools["result"]["tools"]
        .as_array()
        .context("missing tools")?
        .iter()
        .find(|tool| tool["name"] == "read_file")
        .context("missing read_file")?;
    assert!(read["inputSchema"]["properties"]
        .get("dependency_tokens")
        .is_some());
    let mut revision = None;
    for id in [3, 4] {
        if id == 4 {
            std::fs::write(
                project.path().join("level6.zig"),
                "pub fn step6() u32 { return 42; }\n",
            )?;
        }
        let response = client.request(id, "tools/call", modern_params(json!({"name":"read_file", "arguments":{"root":project.path(), "path":"level0.zig", "max_lines":1, "dependency_tokens":16000}}))).await?;
        let text = response["result"]["content"][0]["text"]
            .as_str()
            .context("missing read response")?;
        let payload: context_creator::mcp_server::ReadFileResponse = serde_json::from_str(text)?;
        let context = payload
            .dependency_context
            .context("missing dependency context")?;
        assert!(context.token_count <= 16000);
        assert_eq!(
            context
                .snippets
                .iter()
                .map(|snippet| &snippet.path)
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            7
        );
        if id == 4 {
            assert_ne!(revision, Some(context.source_revision.clone()));
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
        revision = Some(context.source_revision);
    }
    client.shutdown().await?;
    Ok(())
}
