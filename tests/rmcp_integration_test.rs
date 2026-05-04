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

        let mut child = Command::new(binary)
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

        wait_for_port(port).await?;

        Ok(Self { child, port })
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

async fn open_sse(port: u16) -> Result<(BufReader<TcpStream>, String)> {
    let mut stream = TcpStream::connect(("127.0.0.1", port))
        .await
        .context("failed to connect to RMCP SSE endpoint")?;
    let request = format!(
        "GET /sse HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nAccept: text/event-stream\r\nConnection: keep-alive\r\n\r\n"
    );
    stream.write_all(request.as_bytes()).await?;
    stream.flush().await?;

    let mut reader = BufReader::new(stream);
    let status = read_http_headers(&mut reader).await?;
    if !status.starts_with("HTTP/1.1 200") {
        bail!("unexpected SSE status line: {status}");
    }

    let endpoint = read_sse_endpoint(&mut reader).await?;
    Ok((reader, endpoint))
}

async fn read_http_headers(reader: &mut BufReader<TcpStream>) -> Result<String> {
    let mut status = String::new();
    reader.read_line(&mut status).await?;

    let mut line = String::new();
    loop {
        line.clear();
        let bytes = reader.read_line(&mut line).await?;
        if bytes == 0 || line == "\r\n" {
            break;
        }
    }

    Ok(status)
}

async fn read_sse_endpoint(reader: &mut BufReader<TcpStream>) -> Result<String> {
    loop {
        let (event, data) = read_sse_event(reader).await?;
        if event.as_deref() == Some("endpoint") {
            return Ok(data);
        }
    }
}

async fn read_sse_message(reader: &mut BufReader<TcpStream>) -> Result<Value> {
    loop {
        let (event, data) = read_sse_event(reader).await?;
        if event.as_deref() == Some("message") {
            return serde_json::from_str(&data).context("invalid SSE JSON-RPC message");
        }
    }
}

async fn read_sse_event(reader: &mut BufReader<TcpStream>) -> Result<(Option<String>, String)> {
    let mut event = None;
    let mut data_lines = Vec::new();
    let mut line = String::new();

    loop {
        line.clear();
        let bytes = tokio::time::timeout(Duration::from_secs(30), reader.read_line(&mut line))
            .await
            .context("timed out waiting for SSE event")??;
        if bytes == 0 {
            bail!("SSE stream closed before an event was received");
        }

        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            if event.is_some() || !data_lines.is_empty() {
                return Ok((event, data_lines.join("\n")));
            }
            continue;
        }

        if let Some(value) = trimmed.strip_prefix("event:") {
            event = Some(value.trim().to_string());
        } else if let Some(value) = trimmed.strip_prefix("data:") {
            data_lines.push(value.trim().to_string());
        }
    }
}

async fn post_json_rpc(port: u16, endpoint: &str, message: &Value) -> Result<()> {
    let body = message.to_string();
    let mut stream = TcpStream::connect(("127.0.0.1", port))
        .await
        .context("failed to connect to RMCP message endpoint")?;
    let request = format!(
        "POST {endpoint} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(request.as_bytes()).await?;
    stream.flush().await?;

    let mut reader = BufReader::new(stream);
    let status = read_http_headers(&mut reader).await?;
    if !status.starts_with("HTTP/1.1 202") {
        bail!("unexpected RMCP POST status line: {status}");
    }

    Ok(())
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
async fn test_rmcp_http_sse_list_tools() -> Result<()> {
    let server = RmcpHttpServer::spawn().await?;
    let (mut sse_reader, endpoint) = open_sse(server.port).await?;

    post_json_rpc(
        server.port,
        &endpoint,
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/list",
            "params": {}
        }),
    )
    .await?;

    let response = read_sse_message(&mut sse_reader).await?;
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 1);

    let tools = response["result"]["tools"]
        .as_array()
        .context("tools/list HTTP/SSE response did not include tools")?;
    let tool_names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();

    assert!(tool_names.contains(&"analyze_local"));
    assert!(tool_names.contains(&"search"));
    assert!(tool_names.contains(&"semantic_search"));

    server.shutdown().await?;

    Ok(())
}
