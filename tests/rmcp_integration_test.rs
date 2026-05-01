//! Integration tests for RMCP MCP server

use anyhow::{bail, Context, Result};
use serde_json::json;
use serde_json::Value;
use std::process::Stdio;
use std::time::Duration;
use tempfile::TempDir;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
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
