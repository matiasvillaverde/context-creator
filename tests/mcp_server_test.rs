//! Integration tests for MCP server functionality

use anyhow::Result;
use jsonrpsee::{core::client::ClientT, http_client::HttpClientBuilder, rpc_params};
use std::time::Duration;
use tokio::time::timeout;

/// Test that the MCP server starts and responds to health check
#[tokio::test]
async fn test_health_check_endpoint() -> Result<()> {
    // Given: A running MCP server
    let server_handle = start_test_server("127.0.0.1:0").await?;
    let addr = server_handle.local_addr()?;

    // When: We call the health check endpoint
    let client = HttpClientBuilder::default().build(format!("http://{addr}"))?;

    let response: context_creator::mcp_server::HealthResponse =
        client.request("health_check", rpc_params![]).await?;

    // Then: We get a healthy response
    assert_eq!(response.status, "healthy");
    assert!(response.timestamp > 0);
    assert!(!response.version.is_empty());

    Ok(())
}

/// Test that health check responds quickly (< 100ms)
#[tokio::test]
async fn test_health_check_performance() -> Result<()> {
    // Given: A running MCP server
    let server_handle = start_test_server("127.0.0.1:0").await?;
    let addr = server_handle.local_addr()?;

    let client = HttpClientBuilder::default().build(format!("http://{addr}"))?;

    // When: We call health check with timeout
    let response_future = client
        .request::<context_creator::mcp_server::HealthResponse, _>("health_check", rpc_params![]);

    // Then: It responds within 100ms
    let response = timeout(Duration::from_millis(100), response_future).await??;
    assert_eq!(response.status, "healthy");

    Ok(())
}

/// Test graceful shutdown
#[tokio::test]
async fn test_graceful_shutdown() -> Result<()> {
    // Given: A running MCP server
    let server_handle = start_test_server("127.0.0.1:0").await?;
    let addr = server_handle.local_addr()?;

    // When: We stop the server
    server_handle.stop()?;

    // Then: The server should stop accepting new connections
    tokio::time::sleep(Duration::from_millis(100)).await;

    let client = HttpClientBuilder::default().build(format!("http://{addr}"))?;

    let result = client
        .request::<context_creator::mcp_server::HealthResponse, _>("health_check", rpc_params![])
        .await;

    assert!(result.is_err());

    Ok(())
}

/// Test server handles port conflicts
#[tokio::test]
async fn test_port_already_in_use() -> Result<()> {
    // Given: A server already listening on a port
    let server1 = start_test_server("127.0.0.1:0").await?;
    let addr = server1.local_addr()?;

    // When: We try to start another server on the same port
    let result = start_test_server(&addr.to_string()).await;

    // Then: It should fail
    assert!(result.is_err());

    Ok(())
}

/// Test process_local_codebase RPC method
#[tokio::test]
async fn test_process_local_codebase_handler() -> Result<()> {
    use std::path::PathBuf;
    use tempfile::TempDir;

    // Given: A running MCP server and a test directory
    let server_handle = start_test_server("127.0.0.1:0").await?;
    let addr = server_handle.local_addr()?;
    let temp_dir = TempDir::new()?;

    // Create test files
    std::fs::write(
        temp_dir.path().join("main.rs"),
        r#"fn main() {
    println!("Hello, world!");
}"#,
    )?;
    std::fs::write(
        temp_dir.path().join("lib.rs"),
        r#"pub fn add(a: i32, b: i32) -> i32 {
    a + b
}"#,
    )?;

    // When: We call process_local_codebase
    let client = HttpClientBuilder::default().build(format!("http://{addr}"))?;

    #[derive(serde::Serialize)]
    struct ProcessLocalRequest {
        path: PathBuf,
        include_patterns: Vec<String>,
        ignore_patterns: Vec<String>,
        include_imports: bool,
        max_tokens: Option<u32>,
    }

    let request = ProcessLocalRequest {
        path: temp_dir.path().to_path_buf(),
        include_patterns: vec!["*.rs".to_string()],
        ignore_patterns: vec![],
        include_imports: false,
        max_tokens: Some(10000),
    };

    let response: serde_json::Value = client
        .request("process_local_codebase", rpc_params![request])
        .await?;

    // Then: We get a valid response with markdown content
    assert!(response.get("markdown").is_some());
    assert!(response.get("file_count").is_some());
    assert!(response.get("token_count").is_some());
    assert!(response.get("processing_time_ms").is_some());

    let markdown = response["markdown"].as_str().unwrap();
    assert!(markdown.contains("main.rs"));
    assert!(markdown.contains("lib.rs"));
    assert!(markdown.contains("Hello, world!"));
    assert!(markdown.contains("pub fn add"));

    Ok(())
}

/// Test process_local_codebase with path traversal attempt
#[tokio::test]
async fn test_process_local_codebase_path_traversal_rejected() -> Result<()> {
    use std::path::PathBuf;

    // Given: A running MCP server
    let server_handle = start_test_server("127.0.0.1:0").await?;
    let addr = server_handle.local_addr()?;

    // When: We attempt path traversal
    let client = HttpClientBuilder::default().build(format!("http://{addr}"))?;

    #[derive(serde::Serialize)]
    struct ProcessLocalRequest {
        path: PathBuf,
        include_patterns: Vec<String>,
        ignore_patterns: Vec<String>,
        include_imports: bool,
        max_tokens: Option<u32>,
    }

    let request = ProcessLocalRequest {
        path: PathBuf::from("../../../etc/passwd"),
        include_patterns: vec![],
        ignore_patterns: vec![],
        include_imports: false,
        max_tokens: None,
    };

    let result: Result<serde_json::Value, _> = client
        .request("process_local_codebase", rpc_params![request])
        .await;

    // Then: The request should be rejected
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("Invalid path") || err.to_string().contains("security"));

    Ok(())
}

/// Test that repeated requests are cached
#[tokio::test]
async fn test_process_local_codebase_caching() -> Result<()> {
    use std::path::PathBuf;
    use std::time::Instant;
    use tempfile::TempDir;

    // Given: A running MCP server and a test directory
    let server_handle = start_test_server("127.0.0.1:0").await?;
    let addr = server_handle.local_addr()?;
    let temp_dir = TempDir::new()?;

    // Create a larger test file to make timing differences measurable
    let content = "fn main() {\n    println!(\"Hello, world!\");\n}\n".repeat(1000);
    std::fs::write(temp_dir.path().join("main.rs"), &content)?;

    let client = HttpClientBuilder::default().build(format!("http://{addr}"))?;

    #[derive(serde::Serialize)]
    struct ProcessLocalRequest {
        path: PathBuf,
        include_patterns: Vec<String>,
        ignore_patterns: Vec<String>,
        include_imports: bool,
        max_tokens: Option<u32>,
    }

    let request = ProcessLocalRequest {
        path: temp_dir.path().to_path_buf(),
        include_patterns: vec!["*.rs".to_string()],
        ignore_patterns: vec![],
        include_imports: false,
        max_tokens: None,
    };

    // When: We make the first request
    let start1 = Instant::now();
    let response1: serde_json::Value = client
        .request("process_local_codebase", rpc_params![&request])
        .await?;
    let time1 = start1.elapsed();

    // And then make the same request again
    let start2 = Instant::now();
    let response2: serde_json::Value = client
        .request("process_local_codebase", rpc_params![&request])
        .await?;
    let time2 = start2.elapsed();

    // Then: The second request should be significantly faster
    assert!(
        time2 < time1 / 2,
        "Second request should be at least 2x faster due to caching: {time1:?} vs {time2:?}"
    );

    // And the responses should be identical
    assert_eq!(response1["markdown"], response2["markdown"]);
    assert_eq!(response1["file_count"], response2["file_count"]);
    assert_eq!(response1["token_count"], response2["token_count"]);

    Ok(())
}

/// Test process_remote_repo RPC method
#[tokio::test]
async fn test_process_remote_repo_handler() -> Result<()> {
    // Given: A running MCP server
    let server_handle = start_test_server("127.0.0.1:0").await?;
    let addr = server_handle.local_addr()?;

    let client = HttpClientBuilder::default().build(format!("http://{addr}"))?;

    #[derive(serde::Serialize)]
    struct ProcessRemoteRequest {
        repo_url: String,
        include_patterns: Vec<String>,
        ignore_patterns: Vec<String>,
        include_imports: bool,
        max_tokens: Option<u32>,
    }

    // We'll use a small, known public repo for testing
    let request = ProcessRemoteRequest {
        repo_url: "https://github.com/octocat/Hello-World".to_string(),
        include_patterns: vec!["*".to_string()],
        ignore_patterns: vec![],
        include_imports: false,
        max_tokens: Some(10000),
    };

    // When: We call process_remote_repo
    let response: serde_json::Value = client
        .request("process_remote_repo", rpc_params![request])
        .await?;

    // Then: We get a valid response with markdown content
    assert!(response.get("markdown").is_some());
    assert!(response.get("file_count").is_some());
    assert!(response.get("token_count").is_some());
    assert!(response.get("processing_time_ms").is_some());
    assert!(response.get("repo_name").is_some());

    let markdown = response["markdown"].as_str().unwrap();
    assert!(!markdown.is_empty());
    assert!(response["file_count"].as_u64().unwrap() > 0);

    Ok(())
}

/// Test process_remote_repo with invalid URL
#[tokio::test]
async fn test_process_remote_repo_invalid_url() -> Result<()> {
    // Given: A running MCP server
    let server_handle = start_test_server("127.0.0.1:0").await?;
    let addr = server_handle.local_addr()?;

    let client = HttpClientBuilder::default().build(format!("http://{addr}"))?;

    #[derive(serde::Serialize)]
    struct ProcessRemoteRequest {
        repo_url: String,
        include_patterns: Vec<String>,
        ignore_patterns: Vec<String>,
        include_imports: bool,
        max_tokens: Option<u32>,
    }

    let request = ProcessRemoteRequest {
        repo_url: "not-a-valid-url".to_string(),
        include_patterns: vec![],
        ignore_patterns: vec![],
        include_imports: false,
        max_tokens: None,
    };

    // When: We attempt to process an invalid URL
    let result: Result<serde_json::Value, _> = client
        .request("process_remote_repo", rpc_params![request])
        .await;

    // Then: The request should be rejected
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("Invalid URL") || err.to_string().contains("validation"));

    Ok(())
}

/// Test get_file_metadata RPC method
#[tokio::test]
async fn test_get_file_metadata_handler() -> Result<()> {
    use std::path::PathBuf;
    use tempfile::TempDir;

    // Given: A running MCP server and a test file
    let server_handle = start_test_server("127.0.0.1:0").await?;
    let addr = server_handle.local_addr()?;
    let temp_dir = TempDir::new()?;

    // Create a test file
    let test_file = temp_dir.path().join("test.rs");
    std::fs::write(&test_file, "fn main() { println!(\"Hello!\"); }")?;

    // When: We call get_file_metadata
    let client = HttpClientBuilder::default().build(format!("http://{addr}"))?;

    #[derive(serde::Serialize)]
    struct GetFileMetadataRequest {
        file_path: PathBuf,
    }

    let request = GetFileMetadataRequest {
        file_path: test_file.clone(),
    };

    let response: serde_json::Value = client
        .request("get_file_metadata", rpc_params![request])
        .await?;

    // Then: We get valid metadata
    assert_eq!(response["path"], test_file.to_string_lossy().as_ref());
    assert!(response["size"].as_u64().unwrap() > 0);
    assert!(response["modified"].as_u64().unwrap() > 0);
    assert_eq!(response["is_symlink"], false);
    assert_eq!(response["language"].as_str(), Some("rust"));

    Ok(())
}

/// Test get_file_metadata with non-existent file
#[tokio::test]
async fn test_get_file_metadata_non_existent() -> Result<()> {
    use std::path::PathBuf;

    // Given: A running MCP server
    let server_handle = start_test_server("127.0.0.1:0").await?;
    let addr = server_handle.local_addr()?;

    let client = HttpClientBuilder::default().build(format!("http://{addr}"))?;

    #[derive(serde::Serialize)]
    struct GetFileMetadataRequest {
        file_path: PathBuf,
    }

    let request = GetFileMetadataRequest {
        file_path: PathBuf::from("/non/existent/file.rs"),
    };

    // When: We request metadata for a non-existent file
    let result: Result<serde_json::Value, _> = client
        .request("get_file_metadata", rpc_params![request])
        .await;

    // Then: The request should fail
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("not found") || err.to_string().contains("does not exist"));

    Ok(())
}

/// Test get_file_metadata with symlink
#[tokio::test]
async fn test_get_file_metadata_symlink() -> Result<()> {
    use std::path::PathBuf;
    use tempfile::TempDir;

    // Given: A running MCP server, a test file, and a symlink
    let server_handle = start_test_server("127.0.0.1:0").await?;
    let addr = server_handle.local_addr()?;
    let temp_dir = TempDir::new()?;

    // Create a test file and symlink
    let test_file = temp_dir.path().join("target.py");
    std::fs::write(&test_file, "print('Hello from Python')")?;

    let symlink = temp_dir.path().join("link.py");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&test_file, &symlink)?;
    #[cfg(windows)]
    std::os::windows::fs::symlink_file(&test_file, &symlink)?;

    // When: We call get_file_metadata on the symlink
    let client = HttpClientBuilder::default().build(format!("http://{addr}"))?;

    #[derive(serde::Serialize)]
    struct GetFileMetadataRequest {
        file_path: PathBuf,
    }

    let request = GetFileMetadataRequest {
        file_path: symlink.clone(),
    };

    let response: serde_json::Value = client
        .request("get_file_metadata", rpc_params![request])
        .await?;

    // Then: We should see it's a symlink
    assert_eq!(response["path"], symlink.to_string_lossy().as_ref());
    assert_eq!(response["is_symlink"], true);
    assert_eq!(response["language"].as_str(), Some("python"));

    Ok(())
}

/// Test search_codebase RPC method
#[tokio::test]
async fn test_search_codebase_handler() -> Result<()> {
    use std::path::PathBuf;
    use tempfile::TempDir;

    // Given: A running MCP server and a test directory with searchable content
    let server_handle = start_test_server("127.0.0.1:0").await?;
    let addr = server_handle.local_addr()?;
    let temp_dir = TempDir::new()?;

    // Create test files with searchable content
    std::fs::write(
        temp_dir.path().join("main.rs"),
        r#"fn main() {
    println!("Hello, world!");
    let important_value = 42;
    process_data(important_value);
}

fn process_data(value: i32) {
    println!("Processing value: {}", value);
}"#,
    )?;

    std::fs::write(
        temp_dir.path().join("lib.rs"),
        r#"pub fn important_function() {
    // This is an important function
    let result = calculate_important_stuff();
    println!("Important result: {}", result);
}

fn calculate_important_stuff() -> i32 {
    42
}"#,
    )?;

    // When: We search for "important"
    let client = HttpClientBuilder::default().build(format!("http://{addr}"))?;

    #[derive(serde::Serialize)]
    struct SearchCodebaseRequest {
        path: PathBuf,
        query: String,
        max_results: Option<u32>,
        file_pattern: Option<String>,
    }

    let request = SearchCodebaseRequest {
        path: temp_dir.path().to_path_buf(),
        query: "important".to_string(),
        max_results: Some(10),
        file_pattern: Some("*.rs".to_string()),
    };

    let response: serde_json::Value = client
        .request("search_codebase", rpc_params![request])
        .await?;

    // Then: We get search results
    assert!(response["results"].is_array());
    let results = response["results"].as_array().unwrap();
    assert!(!results.is_empty());
    assert!(response["total_matches"].as_u64().unwrap() >= 3); // At least 3 occurrences of "important"
    assert_eq!(response["files_searched"].as_u64().unwrap(), 2);
    assert!(response["search_time_ms"].is_number());

    // Verify result structure
    let first_result = &results[0];
    assert!(first_result["file_path"].is_string());
    assert!(first_result["line_number"].is_number());
    assert!(first_result["line_content"].is_string());
    assert!(first_result["match_context"].is_string());

    Ok(())
}

/// Test search_codebase with no results
#[tokio::test]
async fn test_search_codebase_no_results() -> Result<()> {
    use std::path::PathBuf;
    use tempfile::TempDir;

    // Given: A running MCP server and a test directory
    let server_handle = start_test_server("127.0.0.1:0").await?;
    let addr = server_handle.local_addr()?;
    let temp_dir = TempDir::new()?;

    std::fs::write(
        temp_dir.path().join("test.txt"),
        "This is just a test file with no special content",
    )?;

    let client = HttpClientBuilder::default().build(format!("http://{addr}"))?;

    #[derive(serde::Serialize)]
    struct SearchCodebaseRequest {
        path: PathBuf,
        query: String,
        max_results: Option<u32>,
        file_pattern: Option<String>,
    }

    let request = SearchCodebaseRequest {
        path: temp_dir.path().to_path_buf(),
        query: "nonexistentstring123".to_string(),
        max_results: None,
        file_pattern: None,
    };

    // When: We search for a non-existent string
    let response: serde_json::Value = client
        .request("search_codebase", rpc_params![request])
        .await?;

    // Then: We get empty results
    assert!(response["results"].as_array().unwrap().is_empty());
    assert_eq!(response["total_matches"].as_u64().unwrap(), 0);
    assert!(response["files_searched"].as_u64().unwrap() > 0);

    Ok(())
}

/// Test search_codebase with file pattern filtering
#[tokio::test]
async fn test_search_codebase_with_pattern() -> Result<()> {
    use std::path::PathBuf;
    use tempfile::TempDir;

    // Given: A running MCP server and mixed file types
    let server_handle = start_test_server("127.0.0.1:0").await?;
    let addr = server_handle.local_addr()?;
    let temp_dir = TempDir::new()?;

    std::fs::write(temp_dir.path().join("code.rs"), "fn test() { /* test */ }")?;
    std::fs::write(temp_dir.path().join("doc.md"), "# Test documentation")?;
    std::fs::write(temp_dir.path().join("config.toml"), "test = true")?;

    let client = HttpClientBuilder::default().build(format!("http://{addr}"))?;

    #[derive(serde::Serialize)]
    struct SearchCodebaseRequest {
        path: PathBuf,
        query: String,
        max_results: Option<u32>,
        file_pattern: Option<String>,
    }

    let request = SearchCodebaseRequest {
        path: temp_dir.path().to_path_buf(),
        query: "test".to_string(),
        max_results: None,
        file_pattern: Some("*.rs".to_string()),
    };

    // When: We search with a file pattern
    let response: serde_json::Value = client
        .request("search_codebase", rpc_params![request])
        .await?;

    // Then: Only Rust files are searched
    let results = response["results"].as_array().unwrap();
    for result in results {
        assert!(result["file_path"].as_str().unwrap().ends_with(".rs"));
    }

    Ok(())
}

/// Test diff_files RPC method
#[tokio::test]
async fn test_diff_files_handler() -> Result<()> {
    use std::path::PathBuf;
    use tempfile::TempDir;

    // Given: A running MCP server and two files to diff
    let server_handle = start_test_server("127.0.0.1:0").await?;
    let addr = server_handle.local_addr()?;
    let temp_dir = TempDir::new()?;

    let file1_path = temp_dir.path().join("file1.rs");
    let file2_path = temp_dir.path().join("file2.rs");

    std::fs::write(
        &file1_path,
        r#"fn main() {
    println!("Hello, world!");
}

fn helper() {
    println!("Helper function");
}"#,
    )?;

    std::fs::write(
        &file2_path,
        r#"fn main() {
    println!("Hello, Rust!");
    println!("Welcome!");
}

fn helper() {
    println!("Helper function");
}

fn new_function() {
    println!("New function");
}"#,
    )?;

    // When: We call diff_files
    let client = HttpClientBuilder::default().build(format!("http://{addr}"))?;

    #[derive(serde::Serialize)]
    struct DiffFilesRequest {
        file1_path: PathBuf,
        file2_path: PathBuf,
        context_lines: Option<u32>,
    }

    let request = DiffFilesRequest {
        file1_path: file1_path.clone(),
        file2_path: file2_path.clone(),
        context_lines: Some(3),
    };

    let response: serde_json::Value = client.request("diff_files", rpc_params![request]).await?;

    // Then: We get a diff response
    assert_eq!(
        response["file1_path"],
        file1_path.to_string_lossy().as_ref()
    );
    assert_eq!(
        response["file2_path"],
        file2_path.to_string_lossy().as_ref()
    );
    assert!(response["hunks"].is_array());
    assert!(!response["hunks"].as_array().unwrap().is_empty());
    assert!(response["added_lines"].as_u64().unwrap() > 0);
    assert!(response["removed_lines"].as_u64().unwrap() > 0);
    assert_eq!(response["is_binary"], false);

    Ok(())
}

/// Test diff_files with identical files
#[tokio::test]
async fn test_diff_files_identical() -> Result<()> {
    use std::path::PathBuf;
    use tempfile::TempDir;

    // Given: A running MCP server and two identical files
    let server_handle = start_test_server("127.0.0.1:0").await?;
    let addr = server_handle.local_addr()?;
    let temp_dir = TempDir::new()?;

    let content = "fn main() { println!(\"Same content\"); }";
    let file1_path = temp_dir.path().join("same1.rs");
    let file2_path = temp_dir.path().join("same2.rs");

    std::fs::write(&file1_path, content)?;
    std::fs::write(&file2_path, content)?;

    let client = HttpClientBuilder::default().build(format!("http://{addr}"))?;

    #[derive(serde::Serialize)]
    struct DiffFilesRequest {
        file1_path: PathBuf,
        file2_path: PathBuf,
        context_lines: Option<u32>,
    }

    let request = DiffFilesRequest {
        file1_path,
        file2_path,
        context_lines: None,
    };

    // When: We diff identical files
    let response: serde_json::Value = client.request("diff_files", rpc_params![request]).await?;

    // Then: There should be no hunks
    assert!(response["hunks"].as_array().unwrap().is_empty());
    assert_eq!(response["added_lines"].as_u64().unwrap(), 0);
    assert_eq!(response["removed_lines"].as_u64().unwrap(), 0);

    Ok(())
}

/// Test diff_files with binary files
#[tokio::test]
async fn test_diff_files_binary() -> Result<()> {
    use std::path::PathBuf;
    use tempfile::TempDir;

    // Given: A running MCP server and binary files
    let server_handle = start_test_server("127.0.0.1:0").await?;
    let addr = server_handle.local_addr()?;
    let temp_dir = TempDir::new()?;

    let file1_path = temp_dir.path().join("binary1.bin");
    let file2_path = temp_dir.path().join("binary2.bin");

    // Write binary content
    std::fs::write(&file1_path, [0u8, 1, 2, 3, 255, 254, 253])?;
    std::fs::write(&file2_path, [0u8, 1, 2, 4, 255, 254, 252])?;

    let client = HttpClientBuilder::default().build(format!("http://{addr}"))?;

    #[derive(serde::Serialize)]
    struct DiffFilesRequest {
        file1_path: PathBuf,
        file2_path: PathBuf,
        context_lines: Option<u32>,
    }

    let request = DiffFilesRequest {
        file1_path,
        file2_path,
        context_lines: None,
    };

    // When: We diff binary files
    let response: serde_json::Value = client.request("diff_files", rpc_params![request]).await?;

    // Then: It should be marked as binary
    assert_eq!(response["is_binary"], true);
    assert!(response["hunks"].as_array().unwrap().is_empty());

    Ok(())
}

/// Test semantic_search RPC method for functions
#[tokio::test]
async fn test_semantic_search_functions() -> Result<()> {
    use tempfile::TempDir;

    // Given: A running MCP server and code with functions
    let server_handle = start_test_server("127.0.0.1:0").await?;
    let addr = server_handle.local_addr()?;
    let temp_dir = TempDir::new()?;

    std::fs::write(
        temp_dir.path().join("main.rs"),
        r#"fn main() {
    calculate_total(10, 20);
}

fn calculate_total(a: i32, b: i32) -> i32 {
    add_numbers(a, b)
}

fn add_numbers(x: i32, y: i32) -> i32 {
    x + y
}"#,
    )?;

    std::fs::write(
        temp_dir.path().join("lib.rs"),
        r#"pub fn calculate_average(values: &[f64]) -> f64 {
    let sum: f64 = values.iter().sum();
    sum / values.len() as f64
}

pub fn calculate_median(values: &mut [f64]) -> f64 {
    values.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mid = values.len() / 2;
    values[mid]
}"#,
    )?;

    // When: We search for functions with "calculate" in the name
    let client = HttpClientBuilder::default().build(format!("http://{addr}"))?;

    let response: serde_json::Value = client
        .request(
            "semantic_search",
            rpc_params![serde_json::json!({
                "path": temp_dir.path(),
                "query": "calculate",
                "search_type": "functions",
                "max_results": 10
            })],
        )
        .await?;

    // Then: We find the matching functions
    assert!(response["results"].is_array());
    let results = response["results"].as_array().unwrap();
    assert!(results.len() >= 3); // calculate_total, calculate_average, calculate_median

    // Verify result structure
    for result in results {
        assert!(result["symbol_name"]
            .as_str()
            .unwrap()
            .contains("calculate"));
        assert_eq!(result["symbol_type"].as_str().unwrap(), "function");
        assert!(result["file_path"].is_string());
        assert!(result["line_number"].is_number());
        assert!(result["context"].is_string());
    }

    Ok(())
}

/// Test semantic_search RPC method for types
#[tokio::test]
async fn test_semantic_search_types() -> Result<()> {
    use tempfile::TempDir;

    // Given: A running MCP server and code with types
    let server_handle = start_test_server("127.0.0.1:0").await?;
    let addr = server_handle.local_addr()?;
    let temp_dir = TempDir::new()?;

    std::fs::write(
        temp_dir.path().join("types.rs"),
        r#"struct UserData {
    name: String,
    age: u32,
}

pub struct UserProfile {
    data: UserData,
    preferences: UserPreferences,
}

struct UserPreferences {
    theme: String,
    notifications: bool,
}

enum UserRole {
    Admin,
    User,
    Guest,
}"#,
    )?;

    // When: We search for types with "User" in the name
    let client = HttpClientBuilder::default().build(format!("http://{addr}"))?;

    let response: serde_json::Value = client
        .request(
            "semantic_search",
            rpc_params![serde_json::json!({
                "path": temp_dir.path(),
                "query": "User",
                "search_type": "types",
                "max_results": null
            })],
        )
        .await?;

    // Then: We find the matching types
    let results = response["results"].as_array().unwrap();
    // Note: We're searching type references, not definitions, so we might not find all 4
    assert!(!results.is_empty());

    for result in results {
        assert!(result["symbol_name"].as_str().unwrap().contains("User"));
        assert_eq!(result["symbol_type"].as_str().unwrap(), "type");
    }

    Ok(())
}

/// Test semantic_search RPC method for imports
#[tokio::test]
async fn test_semantic_search_imports() -> Result<()> {
    use tempfile::TempDir;

    // Given: A running MCP server and code with imports
    let server_handle = start_test_server("127.0.0.1:0").await?;
    let addr = server_handle.local_addr()?;
    let temp_dir = TempDir::new()?;

    std::fs::write(
        temp_dir.path().join("main.rs"),
        r#"use std::collections::HashMap;
use std::path::{Path, PathBuf};
use serde::{Serialize, Deserialize};
use tokio::fs;

mod utils;
use utils::helpers::process_data;

fn main() {
    let map = HashMap::new();
}"#,
    )?;

    // When: We search for imports containing "std"
    let client = HttpClientBuilder::default().build(format!("http://{addr}"))?;

    let response: serde_json::Value = client
        .request(
            "semantic_search",
            rpc_params![serde_json::json!({
                "path": temp_dir.path(),
                "query": "std",
                "search_type": "imports",
                "max_results": 10
            })],
        )
        .await?;

    // Then: We find imports from std
    let results = response["results"].as_array().unwrap();
    assert!(results.len() >= 2); // HashMap and Path imports

    for result in results {
        assert!(result["symbol_name"].as_str().unwrap().contains("std"));
        assert_eq!(result["symbol_type"].as_str().unwrap(), "import");
    }

    Ok(())
}

// Helper function to start test server
async fn start_test_server(addr: &str) -> Result<context_creator::mcp_server::ServerHandle> {
    context_creator::mcp_server::start_server(addr).await
}
