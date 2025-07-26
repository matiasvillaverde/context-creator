//! Comprehensive acceptance tests for MCP server
//! These tests serve as both validation and documentation of the MCP server functionality

use anyhow::Result;
use jsonrpsee::core::client::ClientT;
use jsonrpsee::http_client::{HttpClient, HttpClientBuilder};
use jsonrpsee::rpc_params;
use serde_json::json;
use std::path::PathBuf;
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
        let port = 8123; // Use a fixed port for testing

        // Build the binary in release mode for realistic performance testing
        let output = Command::new("cargo")
            .args(&["build", "--release"])
            .output()?;

        if !output.status.success() {
            anyhow::bail!(
                "Failed to build binary: {}",
                String::from_utf8_lossy(&output.stderr)
            );
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
        let client = HttpClientBuilder::default().build(format!("http://127.0.0.1:{}", port))?;

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
async fn test_health_check_endpoint() -> Result<()> {
    let server = TestServer::start().await?;

    // Test: Basic health check
    let response: serde_json::Value = server.client.request("health_check", rpc_params![]).await?;

    assert_eq!(response["status"], "healthy");
    assert!(response["timestamp"].is_number());
    assert!(response["version"].is_string());

    // Document the response format
    println!(
        "Health check response: {}",
        serde_json::to_string_pretty(&response)?
    );

    server.stop()?;
    Ok(())
}

#[tokio::test]
async fn test_process_local_codebase_basic() -> Result<()> {
    let server = TestServer::start().await?;
    let temp_dir = TempDir::new()?;

    // Create a simple test project
    std::fs::write(
        temp_dir.path().join("main.rs"),
        r#"
fn main() {
    println!("Hello, world!");
}

fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_add() {
        assert_eq!(add(2, 2), 4);
    }
}
"#,
    )?;

    std::fs::write(
        temp_dir.path().join("lib.rs"),
        r#"
pub struct Calculator {
    value: i32,
}

impl Calculator {
    pub fn new() -> Self {
        Self { value: 0 }
    }
    
    pub fn add(&mut self, n: i32) {
        self.value += n;
    }
    
    pub fn get_value(&self) -> i32 {
        self.value
    }
}
"#,
    )?;

    // Test 1: Basic question about the codebase
    let response: serde_json::Value = server
        .client
        .request(
            "process_local_codebase",
            rpc_params![json!({
                "prompt": "What does this codebase do? List all the functions and their purposes.",
                "path": temp_dir.path(),
                "include_patterns": ["**/*.rs"],
                "ignore_patterns": [],
                "include_imports": false,
                "include_context": true
            })],
        )
        .await?;

    // Verify response structure
    assert!(response["answer"].is_string());
    assert!(response["context"].is_string());
    assert!(response["file_count"].as_u64().unwrap() >= 2);
    assert!(response["token_count"].is_number());
    assert!(response["processing_time_ms"].is_number());
    assert_eq!(response["llm_tool"], "gemini");

    // The answer should mention the functions
    let answer = response["answer"].as_str().unwrap();
    assert!(answer.contains("main") || answer.contains("add") || answer.contains("Calculator"));

    println!(
        "Basic codebase analysis response: {}",
        serde_json::to_string_pretty(&response)?
    );

    // Test 2: Specific query with file filtering
    let response: serde_json::Value = server
        .client
        .request(
            "process_local_codebase",
            rpc_params![json!({
                "prompt": "Find all test functions and explain what they test",
                "path": temp_dir.path(),
                "include_patterns": ["**/main.rs"],
                "ignore_patterns": [],
                "include_imports": false,
                "llm_tool": "gemini"
            })],
        )
        .await?;

    assert!(response["answer"].as_str().unwrap().contains("test_add"));
    assert_eq!(response["file_count"], 1);

    server.stop()?;
    Ok(())
}

#[tokio::test]
async fn test_process_local_codebase_with_imports() -> Result<()> {
    let server = TestServer::start().await?;
    let temp_dir = TempDir::new()?;

    // Create a project with imports
    std::fs::create_dir_all(temp_dir.path().join("src"))?;

    std::fs::write(
        temp_dir.path().join("src/main.rs"),
        r#"
mod utils;
mod config;

use utils::process_data;
use config::Config;

fn main() {
    let config = Config::load();
    let data = vec![1, 2, 3, 4, 5];
    let result = process_data(&data, &config);
    println!("Result: {:?}", result);
}
"#,
    )?;

    std::fs::write(
        temp_dir.path().join("src/utils.rs"),
        r#"
use crate::config::Config;

pub fn process_data(data: &[i32], config: &Config) -> Vec<i32> {
    data.iter()
        .map(|&x| x * config.multiplier)
        .collect()
}

pub fn validate_data(data: &[i32]) -> bool {
    !data.is_empty() && data.iter().all(|&x| x > 0)
}
"#,
    )?;

    std::fs::write(
        temp_dir.path().join("src/config.rs"),
        r#"
pub struct Config {
    pub multiplier: i32,
    pub debug: bool,
}

impl Config {
    pub fn load() -> Self {
        Self {
            multiplier: 2,
            debug: false,
        }
    }
}
"#,
    )?;

    // Test with import tracing enabled
    let response: serde_json::Value = server.client
        .request("process_local_codebase", rpc_params![json!({
            "prompt": "Explain how the data processing flow works, tracing through all the imports and dependencies",
            "path": temp_dir.path().join("src"),
            "include_patterns": ["**/*.rs"],
            "ignore_patterns": [],
            "include_imports": true,
            "include_context": true
        })])
        .await?;

    let answer = response["answer"].as_str().unwrap();
    let context = response["context"].as_str().unwrap();

    // Should explain the flow through imports
    assert!(answer.contains("process_data") || answer.contains("Config"));

    // Context should include all related files when imports are traced
    assert!(context.contains("main.rs"));
    assert!(context.contains("utils.rs"));
    assert!(context.contains("config.rs"));

    server.stop()?;
    Ok(())
}

#[tokio::test]
async fn test_process_remote_repo() -> Result<()> {
    let server = TestServer::start().await?;

    // Test with a small public repository
    let response: serde_json::Value = server
        .client
        .request(
            "process_remote_repo",
            rpc_params![json!({
                "prompt": "What is the main purpose of this repository? What are its key features?",
                "repo_url": "https://github.com/rust-lang/mdBook.git",
                "include_patterns": ["**/*.rs", "**/*.md"],
                "ignore_patterns": ["target/**", ".git/**"],
                "include_imports": false,
                "max_tokens": 50000,
                "include_context": false
            })],
        )
        .await?;

    assert!(response["answer"].is_string());
    assert!(response["repo_name"].is_string());
    assert!(response["file_count"].as_u64().unwrap() > 0);
    assert_eq!(response["llm_tool"], "gemini");

    println!(
        "Remote repo analysis: {}",
        serde_json::to_string_pretty(&response)?
    );

    server.stop()?;
    Ok(())
}

#[tokio::test]
async fn test_get_file_metadata() -> Result<()> {
    let server = TestServer::start().await?;
    let temp_dir = TempDir::new()?;

    // Create test files with different types
    let rust_file = temp_dir.path().join("test.rs");
    std::fs::write(&rust_file, "fn main() {}")?;

    let python_file = temp_dir.path().join("test.py");
    std::fs::write(&python_file, "def main():\n    pass")?;

    let binary_file = temp_dir.path().join("test.bin");
    std::fs::write(&binary_file, &[0u8, 1, 2, 3, 255])?;

    // Test Rust file
    let response: serde_json::Value = server
        .client
        .request(
            "get_file_metadata",
            rpc_params![json!({
                "file_path": rust_file
            })],
        )
        .await?;

    assert!(response["size"].as_u64().unwrap() > 0);
    assert!(response["modified"].is_number());
    assert_eq!(response["is_symlink"], false);
    assert_eq!(response["language"], "rust");

    // Test Python file
    let response: serde_json::Value = server
        .client
        .request(
            "get_file_metadata",
            rpc_params![json!({
                "file_path": python_file
            })],
        )
        .await?;

    assert_eq!(response["language"], "python");

    // Test binary file
    let response: serde_json::Value = server
        .client
        .request(
            "get_file_metadata",
            rpc_params![json!({
                "file_path": binary_file
            })],
        )
        .await?;

    assert_eq!(response["language"], json!(null));

    server.stop()?;
    Ok(())
}

#[tokio::test]
async fn test_search_codebase() -> Result<()> {
    let server = TestServer::start().await?;
    let temp_dir = TempDir::new()?;

    // Create files with searchable content
    std::fs::write(
        temp_dir.path().join("auth.rs"),
        r#"
use bcrypt::{hash, verify};

pub fn hash_password(password: &str) -> Result<String, Error> {
    hash(password, 10)
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool, Error> {
    verify(password, hash)
}

pub fn authenticate_user(username: &str, password: &str) -> bool {
    // TODO: Implement database lookup
    if username == "admin" && password == "secret" {
        return true;
    }
    false
}
"#,
    )?;

    std::fs::write(
        temp_dir.path().join("user.rs"),
        r#"
pub struct User {
    username: String,
    email: String,
    password_hash: String,
}

impl User {
    pub fn new(username: String, email: String, password: &str) -> Self {
        let password_hash = crate::auth::hash_password(password).unwrap();
        Self {
            username,
            email,
            password_hash,
        }
    }
    
    pub fn verify_password(&self, password: &str) -> bool {
        crate::auth::verify_password(password, &self.password_hash).unwrap_or(false)
    }
}
"#,
    )?;

    // Test 1: Search for password-related code
    let response: serde_json::Value = server
        .client
        .request(
            "search_codebase",
            rpc_params![json!({
                "path": temp_dir.path(),
                "query": "password",
                "max_results": 10
            })],
        )
        .await?;

    assert!(response["total_matches"].as_u64().unwrap() >= 4);
    assert!(response["results"].as_array().unwrap().len() > 0);

    let first_result = &response["results"][0];
    assert!(first_result["line_content"]
        .as_str()
        .unwrap()
        .contains("password"));

    // Test 2: Search with file pattern
    let response: serde_json::Value = server
        .client
        .request(
            "search_codebase",
            rpc_params![json!({
                "path": temp_dir.path(),
                "query": "username",
                "file_pattern": "**/user.rs"
            })],
        )
        .await?;

    assert!(response["results"]
        .as_array()
        .unwrap()
        .iter()
        .all(|r| r["file_path"].as_str().unwrap().ends_with("user.rs")));

    println!(
        "Search results: {}",
        serde_json::to_string_pretty(&response)?
    );

    server.stop()?;
    Ok(())
}

#[tokio::test]
async fn test_diff_files() -> Result<()> {
    let server = TestServer::start().await?;
    let temp_dir = TempDir::new()?;

    // Create two versions of a file
    let file1 = temp_dir.path().join("version1.rs");
    std::fs::write(
        &file1,
        r#"
fn calculate(x: i32, y: i32) -> i32 {
    x + y
}

fn main() {
    let result = calculate(5, 3);
    println!("Result: {}", result);
}
"#,
    )?;

    let file2 = temp_dir.path().join("version2.rs");
    std::fs::write(
        &file2,
        r#"
fn calculate(x: i32, y: i32, operation: &str) -> i32 {
    match operation {
        "add" => x + y,
        "subtract" => x - y,
        "multiply" => x * y,
        _ => 0,
    }
}

fn main() {
    let result = calculate(5, 3, "add");
    println!("Addition: {}", result);
    
    let result2 = calculate(10, 4, "multiply");
    println!("Multiplication: {}", result2);
}
"#,
    )?;

    // Get diff
    let response: serde_json::Value = server
        .client
        .request(
            "diff_files",
            rpc_params![json!({
                "file1_path": file1,
                "file2_path": file2,
                "context_lines": 3
            })],
        )
        .await?;

    assert!(!response["hunks"].as_array().unwrap().is_empty());
    assert!(response["added_lines"].as_u64().unwrap() > 0);
    assert!(response["removed_lines"].as_u64().unwrap() > 0);
    assert_eq!(response["is_binary"], false);

    // Check that diff shows the function signature change
    let hunks = response["hunks"].as_array().unwrap();
    let hunk_content = hunks[0]["content"].as_str().unwrap();
    assert!(hunk_content.contains("operation: &str"));

    println!("Diff output: {}", serde_json::to_string_pretty(&response)?);

    server.stop()?;
    Ok(())
}

#[tokio::test]
async fn test_semantic_search() -> Result<()> {
    let server = TestServer::start().await?;
    let temp_dir = TempDir::new()?;

    // Create a complex codebase for semantic search
    std::fs::write(
        temp_dir.path().join("main.rs"),
        r#"
mod database;
mod handlers;
mod models;

use handlers::{UserHandler, PostHandler};
use models::{User, Post};

fn main() {
    let user_handler = UserHandler::new();
    let post_handler = PostHandler::new();
    
    // Example usage
    let user = user_handler.create_user("john", "john@example.com");
    let post = post_handler.create_post(&user, "Hello World", "My first post");
}
"#,
    )?;

    std::fs::write(
        temp_dir.path().join("models.rs"),
        r#"
#[derive(Debug, Clone)]
pub struct User {
    pub id: u64,
    pub username: String,
    pub email: String,
}

#[derive(Debug)]
pub struct Post {
    pub id: u64,
    pub author: User,
    pub title: String,
    pub content: String,
}

pub trait Model {
    fn id(&self) -> u64;
}

impl Model for User {
    fn id(&self) -> u64 {
        self.id
    }
}

impl Model for Post {
    fn id(&self) -> u64 {
        self.id
    }
}
"#,
    )?;

    std::fs::write(
        temp_dir.path().join("handlers.rs"),
        r#"
use crate::models::{User, Post};

pub struct UserHandler {
    next_id: u64,
}

impl UserHandler {
    pub fn new() -> Self {
        Self { next_id: 1 }
    }
    
    pub fn create_user(&mut self, username: &str, email: &str) -> User {
        let user = User {
            id: self.next_id,
            username: username.to_string(),
            email: email.to_string(),
        };
        self.next_id += 1;
        user
    }
}

pub struct PostHandler {
    next_id: u64,
}

impl PostHandler {
    pub fn new() -> Self {
        Self { next_id: 1 }
    }
    
    pub fn create_post(&mut self, author: &User, title: &str, content: &str) -> Post {
        let post = Post {
            id: self.next_id,
            author: author.clone(),
            title: title.to_string(),
            content: content.to_string(),
        };
        self.next_id += 1;
        post
    }
}
"#,
    )?;

    // Test 1: Find all functions
    let response: serde_json::Value = server
        .client
        .request(
            "semantic_search",
            rpc_params![json!({
                "path": temp_dir.path(),
                "query": "create",
                "search_type": "functions",
                "max_results": 10
            })],
        )
        .await?;

    let results = response["results"].as_array().unwrap();
    assert!(results.len() >= 2); // Should find create_user and create_post
    assert!(results
        .iter()
        .any(|r| r["symbol_name"].as_str().unwrap().contains("create_user")));
    assert!(results
        .iter()
        .any(|r| r["symbol_name"].as_str().unwrap().contains("create_post")));

    // Test 2: Find all types/structs
    let response: serde_json::Value = server
        .client
        .request(
            "semantic_search",
            rpc_params![json!({
                "path": temp_dir.path(),
                "query": "",
                "search_type": "types",
                "max_results": 20
            })],
        )
        .await?;

    let results = response["results"].as_array().unwrap();
    assert!(results.iter().any(|r| r["symbol_name"] == "User"));
    assert!(results.iter().any(|r| r["symbol_name"] == "Post"));
    assert!(results.iter().any(|r| r["symbol_name"] == "UserHandler"));

    // Test 3: Find imports
    let response: serde_json::Value = server
        .client
        .request(
            "semantic_search",
            rpc_params![json!({
                "path": temp_dir.path(),
                "query": "models",
                "search_type": "imports"
            })],
        )
        .await?;

    assert!(response["total_matches"].as_u64().unwrap() > 0);

    // Test 4: Find references to a symbol
    let response: serde_json::Value = server
        .client
        .request(
            "semantic_search",
            rpc_params![json!({
                "path": temp_dir.path(),
                "query": "User",
                "search_type": "references"
            })],
        )
        .await?;

    // Should find references in multiple files
    let file_paths: Vec<_> = response["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["file_path"].as_str().unwrap())
        .collect();

    assert!(file_paths.iter().any(|p| p.ends_with("models.rs")));
    assert!(file_paths.iter().any(|p| p.ends_with("handlers.rs")));

    println!(
        "Semantic search results: {}",
        serde_json::to_string_pretty(&response)?
    );

    server.stop()?;
    Ok(())
}

#[tokio::test]
async fn test_performance_and_caching() -> Result<()> {
    let server = TestServer::start().await?;
    let temp_dir = TempDir::new()?;

    // Create a moderately complex project
    for i in 0..10 {
        std::fs::write(
            temp_dir.path().join(format!("module{}.rs", i)),
            format!(
                r#"
pub mod module{} {{
    pub fn process_{}_data(input: &[u8]) -> Vec<u8> {{
        input.iter().map(|&b| b.wrapping_add({} as u8)).collect()
    }}
    
    pub struct Processor{} {{
        id: u64,
        name: String,
    }}
    
    impl Processor{} {{
        pub fn new(name: String) -> Self {{
            Self {{ id: {}, name }}
        }}
    }}
}}
"#,
                i, i, i, i, i, i
            ),
        )?;
    }

    // First request (cache miss)
    let start1 = std::time::Instant::now();
    let response1: serde_json::Value = server
        .client
        .request(
            "process_local_codebase",
            rpc_params![json!({
                "prompt": "List all the processor types and their IDs",
                "path": temp_dir.path(),
                "include_patterns": ["**/*.rs"],
                "ignore_patterns": [],
                "include_imports": false
            })],
        )
        .await?;
    let duration1 = start1.elapsed();

    // Second identical request (cache hit)
    let start2 = std::time::Instant::now();
    let response2: serde_json::Value = server
        .client
        .request(
            "process_local_codebase",
            rpc_params![json!({
                "prompt": "List all the processor types and their IDs",
                "path": temp_dir.path(),
                "include_patterns": ["**/*.rs"],
                "ignore_patterns": [],
                "include_imports": false
            })],
        )
        .await?;
    let duration2 = start2.elapsed();

    // Cache should make second request much faster
    assert!(
        duration2 < duration1 / 2,
        "Cache didn't improve performance: {:?} vs {:?}",
        duration1,
        duration2
    );

    // Responses should be identical
    assert_eq!(response1["answer"], response2["answer"]);
    assert_eq!(response1["file_count"], response2["file_count"]);

    println!(
        "Performance test - First request: {:?}, Cached request: {:?}",
        duration1, duration2
    );

    server.stop()?;
    Ok(())
}

#[tokio::test]
async fn test_concurrent_requests() -> Result<()> {
    let server = TestServer::start().await?;
    let temp_dir = TempDir::new()?;

    // Create test files
    for i in 0..5 {
        std::fs::write(
            temp_dir.path().join(format!("file{}.rs", i)),
            format!("fn function_{}() {{ println!(\"{}\"); }}", i, i),
        )?;
    }

    // Send multiple concurrent requests
    let mut handles = vec![];

    for i in 0..5 {
        let client = server.client.clone();
        let path = temp_dir.path().to_path_buf();

        let handle = tokio::spawn(async move {
            let response: serde_json::Value = client
                .request(
                    "process_local_codebase",
                    rpc_params![json!({
                        "prompt": format!("What does function_{} do?", i),
                        "path": path,
                        "include_patterns": [format!("**/file{}.rs", i)],
                        "ignore_patterns": [],
                        "include_imports": false
                    })],
                )
                .await?;
            Ok::<_, anyhow::Error>(response)
        });

        handles.push(handle);
    }

    // Wait for all requests to complete
    let mut results = vec![];
    for handle in handles {
        results.push(handle.await);
    }

    // All requests should succeed
    for (i, result) in results.iter().enumerate() {
        let response = result.as_ref().unwrap().as_ref().unwrap();
        assert!(response["answer"]
            .as_str()
            .unwrap()
            .contains(&format!("{}", i)));
    }

    server.stop()?;
    Ok(())
}

#[tokio::test]
async fn test_security_validation() -> Result<()> {
    let server = TestServer::start().await?;

    // Test 1: Path traversal attempt
    let result = server
        .client
        .request::<_, serde_json::Value>(
            "process_local_codebase",
            rpc_params![json!({
                "prompt": "What's in this directory?",
                "path": "../../../etc",
                "include_patterns": ["**/*"],
                "ignore_patterns": [],
                "include_imports": false
            })],
        )
        .await;

    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("Invalid path") || error.to_string().contains("traversal"));

    // Test 2: Invalid repository URL
    let result = server
        .client
        .request::<_, serde_json::Value>(
            "process_remote_repo",
            rpc_params![json!({
                "prompt": "Analyze this repo",
                "repo_url": "file:///etc/passwd",
                "include_patterns": [],
                "ignore_patterns": [],
                "include_imports": false
            })],
        )
        .await;

    assert!(result.is_err());

    // Test 3: Non-existent path
    let result = server
        .client
        .request::<_, serde_json::Value>(
            "get_file_metadata",
            rpc_params![json!({
                "file_path": "/definitely/does/not/exist/file.rs"
            })],
        )
        .await;

    assert!(result.is_err());

    server.stop()?;
    Ok(())
}

#[tokio::test]
async fn test_edge_cases() -> Result<()> {
    let server = TestServer::start().await?;
    let temp_dir = TempDir::new()?;

    // Test 1: Empty directory
    let empty_dir = temp_dir.path().join("empty");
    std::fs::create_dir(&empty_dir)?;

    let response: serde_json::Value = server
        .client
        .request(
            "process_local_codebase",
            rpc_params![json!({
                "prompt": "What files are in this directory?",
                "path": empty_dir,
                "include_patterns": ["**/*"],
                "ignore_patterns": [],
                "include_imports": false
            })],
        )
        .await?;

    assert_eq!(response["file_count"], 0);

    // Test 2: Binary files
    let binary_file = temp_dir.path().join("binary.dat");
    std::fs::write(&binary_file, &[0u8, 1, 2, 3, 255, 254, 253])?;

    let response: serde_json::Value = server
        .client
        .request(
            "diff_files",
            rpc_params![json!({
                "file1_path": &binary_file,
                "file2_path": &binary_file,
                "context_lines": 3
            })],
        )
        .await?;

    assert_eq!(response["is_binary"], true);

    // Test 3: Very long prompt with token limits
    let long_prompt = "Explain this: ".repeat(1000);
    let response: serde_json::Value = server
        .client
        .request(
            "process_local_codebase",
            rpc_params![json!({
                "prompt": long_prompt,
                "path": temp_dir.path(),
                "include_patterns": ["**/*"],
                "ignore_patterns": [],
                "include_imports": false,
                "max_tokens": 1000
            })],
        )
        .await?;

    // Should still work but with limited context
    assert!(response["answer"].is_string());

    // Test 4: Special characters in file names
    let special_file = temp_dir.path().join("file with spaces & special.rs");
    std::fs::write(&special_file, "fn main() {}")?;

    let response: serde_json::Value = server
        .client
        .request(
            "get_file_metadata",
            rpc_params![json!({
                "file_path": special_file
            })],
        )
        .await?;

    assert_eq!(response["language"], "rust");

    server.stop()?;
    Ok(())
}

#[tokio::test]
async fn test_llm_tool_selection() -> Result<()> {
    let server = TestServer::start().await?;
    let temp_dir = TempDir::new()?;

    std::fs::write(
        temp_dir.path().join("test.rs"),
        "fn main() { println!(\"Hello\"); }",
    )?;

    // Test with different LLM tools
    for tool in &["gemini", "codex"] {
        let response: serde_json::Value = server
            .client
            .request(
                "process_local_codebase",
                rpc_params![json!({
                    "prompt": "What does this code do?",
                    "path": temp_dir.path(),
                    "include_patterns": ["**/*.rs"],
                    "ignore_patterns": [],
                    "include_imports": false,
                    "llm_tool": tool
                })],
            )
            .await?;

        assert_eq!(response["llm_tool"], *tool);
    }

    // Test with invalid tool (should default to gemini)
    let response: serde_json::Value = server
        .client
        .request(
            "process_local_codebase",
            rpc_params![json!({
                "prompt": "What does this code do?",
                "path": temp_dir.path(),
                "include_patterns": ["**/*.rs"],
                "ignore_patterns": [],
                "include_imports": false,
                "llm_tool": "invalid_tool"
            })],
        )
        .await?;

    assert_eq!(response["llm_tool"], "gemini");

    server.stop()?;
    Ok(())
}
