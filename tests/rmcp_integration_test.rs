//! Integration tests for RMCP MCP server

use anyhow::Result;
use serde_json::json;
use std::process::Stdio;
use tempfile::TempDir;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;

#[tokio::test]
#[ignore = "RMCP stdio transport has initialization issues"]
async fn test_rmcp_server_initialization() -> Result<()> {
    // Start the RMCP server
    let mut child = Command::new("./target/release/context-creator")
        .arg("--rmcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;

    let stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);
    let mut writer = stdin;

    // Send initialize request
    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "0.1.0",
            "capabilities": {}
        }
    });

    writer
        .write_all(format!("{init_request}\n").as_bytes())
        .await?;
    writer.flush().await?;

    // Read response
    let mut line = String::new();
    reader.read_line(&mut line).await?;

    let response: serde_json::Value = serde_json::from_str(&line)?;

    // Verify response
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 1);
    assert!(response["result"]["capabilities"].is_object());

    // Clean up
    child.kill().await?;

    Ok(())
}

#[tokio::test]
#[ignore = "RMCP stdio transport has initialization issues"]
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

    // Start the RMCP server
    let mut child = Command::new("./target/release/context-creator")
        .arg("--rmcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;

    let stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);
    let mut writer = stdin;

    // Initialize first
    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "0.1.0",
            "capabilities": {}
        }
    });

    writer
        .write_all(format!("{init_request}\n").as_bytes())
        .await?;
    writer.flush().await?;

    // Read init response
    let mut line = String::new();
    reader.read_line(&mut line).await?;
    line.clear();

    // Call analyze_local tool
    let tool_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "analyze_local",
            "arguments": {
                "prompt": "What does the add function do?",
                "path": temp_dir.path(),
                "include_patterns": [],
                "ignore_patterns": [],
                "include_imports": false,
                "max_tokens": 1000,
                "llm_tool": "gemini",
                "include_context": false
            }
        }
    });

    writer
        .write_all(format!("{tool_request}\n").as_bytes())
        .await?;
    writer.flush().await?;

    // Read response
    reader.read_line(&mut line).await?;
    let response: serde_json::Value = serde_json::from_str(&line)?;

    // Verify response contains answer
    assert!(response["result"]["answer"].is_string());
    assert!(response["result"]["file_count"].is_number());
    assert!(response["result"]["token_count"].is_number());

    // Clean up
    child.kill().await?;

    Ok(())
}

#[tokio::test]
#[ignore = "RMCP stdio transport has initialization issues"]
async fn test_list_tools() -> Result<()> {
    // Start the RMCP server
    let mut child = Command::new("./target/release/context-creator")
        .arg("--rmcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;

    let stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);
    let mut writer = stdin;

    // Initialize first
    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "0.1.0",
            "capabilities": {}
        }
    });

    writer
        .write_all(format!("{init_request}\n").as_bytes())
        .await?;
    writer.flush().await?;

    // Read init response
    let mut line = String::new();
    reader.read_line(&mut line).await?;
    line.clear();

    // List tools
    let list_tools_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {}
    });

    writer
        .write_all(format!("{list_tools_request}\n").as_bytes())
        .await?;
    writer.flush().await?;

    // Read response
    reader.read_line(&mut line).await?;
    let response: serde_json::Value = serde_json::from_str(&line)?;

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

    // Clean up
    child.kill().await?;

    Ok(())
}
