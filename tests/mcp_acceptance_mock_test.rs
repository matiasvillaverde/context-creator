//! Acceptance tests for MCP server (with mocked LLM responses)
//! These tests validate the MCP server functionality without calling actual LLMs

use anyhow::Result;
use jsonrpsee::http_client::{HttpClient, HttpClientBuilder};
use jsonrpsee::core::client::ClientT;
use jsonrpsee::rpc_params;
use serde_json::json;
use std::process::{Child, Command};
use std::time::Duration;
use tempfile::TempDir;
use tokio::time::sleep;

/// Test server configuration
struct TestServer {
    process: Child,
    port: u16,
    client: HttpClient,
}

impl TestServer {
    /// Start a test MCP server
    async fn start() -> Result<Self> {
        let port = 8124; // Use a different port to avoid conflicts
        
        // Build the binary in release mode for realistic performance testing
        let output = Command::new("cargo")
            .args(&["build", "--release"])
            .output()?;
        
        if !output.status.success() {
            anyhow::bail!("Failed to build binary: {}", String::from_utf8_lossy(&output.stderr));
        }
        
        // Start the server
        let process = Command::new("./target/release/context-creator")
            .arg("--mcp")
            .arg("--mcp-port")
            .arg(port.to_string())
            .spawn()?;
        
        // Wait for server to start
        sleep(Duration::from_millis(500)).await;
        
        // Create client
        let client = HttpClientBuilder::default()
            .build(format!("http://127.0.0.1:{}", port))?;
        
        Ok(Self {
            process,
            port,
            client,
        })
    }
    
    /// Stop the test server
    fn stop(mut self) -> Result<()> {
        self.process.kill()?;
        Ok(())
    }
}

#[tokio::test]
async fn test_health_check() -> Result<()> {
    let server = TestServer::start().await?;
    
    let response: serde_json::Value = server.client
        .request("health_check", rpc_params![])
        .await?;
    
    assert_eq!(response["status"], "healthy");
    assert!(response["timestamp"].is_number());
    assert!(response["version"].is_string());
    
    server.stop()?;
    Ok(())
}

#[tokio::test]
async fn test_get_file_metadata_basic() -> Result<()> {
    let server = TestServer::start().await?;
    let temp_dir = TempDir::new()?;
    
    // Create test files
    let rust_file = temp_dir.path().join("test.rs");
    std::fs::write(&rust_file, "fn main() { println!(\"Hello\"); }")?;
    
    let response: serde_json::Value = server.client
        .request("get_file_metadata", json!({
            "file_path": rust_file
        }))
        .await?;
    
    assert!(response["size"].as_u64().unwrap() > 0);
    assert!(response["modified"].is_number());
    assert_eq!(response["is_symlink"], false);
    assert_eq!(response["language"], "rust");
    
    server.stop()?;
    Ok(())
}

#[tokio::test]
async fn test_search_codebase_basic() -> Result<()> {
    let server = TestServer::start().await?;
    let temp_dir = TempDir::new()?;
    
    // Create searchable content
    std::fs::write(
        temp_dir.path().join("code.rs"),
        r#"
fn calculate_sum(a: i32, b: i32) -> i32 {
    a + b
}

fn calculate_product(a: i32, b: i32) -> i32 {
    a * b
}

#[test]
fn test_calculate() {
    assert_eq!(calculate_sum(2, 3), 5);
    assert_eq!(calculate_product(2, 3), 6);
}
"#,
    )?;
    
    let response: serde_json::Value = server.client
        .request("search_codebase", json!({
            "path": temp_dir.path(),
            "query": "calculate",
            "max_results": 10
        }))
        .await?;
    
    assert!(response["total_matches"].as_u64().unwrap() >= 4);
    assert!(!response["results"].as_array().unwrap().is_empty());
    
    server.stop()?;
    Ok(())
}

#[tokio::test]
async fn test_diff_files_basic() -> Result<()> {
    let server = TestServer::start().await?;
    let temp_dir = TempDir::new()?;
    
    let file1 = temp_dir.path().join("v1.txt");
    std::fs::write(&file1, "Line 1\nLine 2\nLine 3\n")?;
    
    let file2 = temp_dir.path().join("v2.txt");
    std::fs::write(&file2, "Line 1\nLine 2 modified\nLine 3\nLine 4\n")?;
    
    let response: serde_json::Value = server.client
        .request("diff_files", json!({
            "file1_path": file1,
            "file2_path": file2,
            "context_lines": 1
        }))
        .await?;
    
    assert!(!response["hunks"].as_array().unwrap().is_empty());
    assert!(response["added_lines"].as_u64().unwrap() > 0);
    assert_eq!(response["is_binary"], false);
    
    server.stop()?;
    Ok(())
}

#[tokio::test]
async fn test_semantic_search_functions() -> Result<()> {
    let server = TestServer::start().await?;
    let temp_dir = TempDir::new()?;
    
    std::fs::write(
        temp_dir.path().join("lib.rs"),
        r#"
pub fn process_data(input: &str) -> String {
    input.to_uppercase()
}

pub fn validate_data(input: &str) -> bool {
    !input.is_empty()
}

impl DataProcessor {
    pub fn new() -> Self {
        Self {}
    }
    
    pub fn process(&self, data: &str) -> String {
        process_data(data)
    }
}
"#,
    )?;
    
    let response: serde_json::Value = server.client
        .request("semantic_search", json!({
            "path": temp_dir.path(),
            "query": "process",
            "search_type": "functions",
            "max_results": 10
        }))
        .await?;
    
    let results = response["results"].as_array().unwrap();
    assert!(results.len() >= 2); // Should find process_data and process method
    
    server.stop()?;
    Ok(())
}

#[tokio::test]
async fn test_security_path_validation() -> Result<()> {
    let server = TestServer::start().await?;
    
    // Test path traversal protection
    let result = server.client
        .request::<_, serde_json::Value>("get_file_metadata", json!({
            "file_path": "../../../etc/passwd"
        }))
        .await;
    
    assert!(result.is_err());
    
    server.stop()?;
    Ok(())
}

#[tokio::test]
async fn test_concurrent_metadata_requests() -> Result<()> {
    let server = TestServer::start().await?;
    let temp_dir = TempDir::new()?;
    
    // Create multiple files
    for i in 0..5 {
        std::fs::write(
            temp_dir.path().join(format!("file{}.txt", i)),
            format!("Content {}", i),
        )?;
    }
    
    // Send concurrent requests
    let mut handles = vec![];
    
    for i in 0..5 {
        let client = server.client.clone();
        let file_path = temp_dir.path().join(format!("file{}.txt", i));
        
        let handle = tokio::spawn(async move {
            let response: serde_json::Value = client
                .request("get_file_metadata", json!({
                    "file_path": file_path
                }))
                .await?;
            Ok::<_, anyhow::Error>(response)
        });
        
        handles.push(handle);
    }
    
    // Collect results
    for handle in handles {
        let result = handle.await?;
        assert!(result.is_ok());
    }
    
    server.stop()?;
    Ok(())
}