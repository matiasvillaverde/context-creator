//! Example MCP client demonstrating how to use the context-creator MCP server
//!
//! Run this example with:
//! ```
//! cargo run --example mcp_client
//! ```
//!
//! Make sure the MCP server is running first:
//! ```
//! cargo run -- --mcp
//! ```

use anyhow::Result;
use jsonrpsee::{core::client::ClientT, http_client::HttpClientBuilder, rpc_params};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<()> {
    // Connect to the MCP server
    let client = HttpClientBuilder::default().build("http://127.0.0.1:8080")?;

    println!("Connected to MCP server at http://127.0.0.1:8080");

    // 1. Health check
    println!("\n=== Health Check ===");
    let health: serde_json::Value = client.request("health_check", rpc_params![]).await?;
    println!("Server status: {}", health["status"]);
    println!("Server version: {}", health["version"]);

    // 2. Process local codebase
    println!("\n=== Process Local Codebase ===");
    let local_response: serde_json::Value = client
        .request(
            "process_local_codebase",
            rpc_params![json!({
                "path": ".",
                "include_patterns": ["*.rs"],
                "ignore_patterns": ["target/*"],
                "include_imports": false,
                "max_tokens": 5000
            })],
        )
        .await?;

    println!("Files processed: {}", local_response["file_count"]);
    println!("Tokens used: {}", local_response["token_count"]);
    println!(
        "Processing time: {}ms",
        local_response["processing_time_ms"]
    );

    // 3. Get file metadata
    println!("\n=== Get File Metadata ===");
    let metadata_response: serde_json::Value = client
        .request(
            "get_file_metadata",
            rpc_params![json!({
                "file_path": "Cargo.toml"
            })],
        )
        .await?;

    println!("File: {}", metadata_response["path"]);
    println!("Size: {} bytes", metadata_response["size"]);
    println!(
        "Language: {}",
        metadata_response["language"].as_str().unwrap_or("unknown")
    );

    // 4. Search codebase
    println!("\n=== Search Codebase ===");
    let search_response: serde_json::Value = client
        .request(
            "search_codebase",
            rpc_params![json!({
                "path": ".",
                "query": "TODO",
                "max_results": 5,
                "file_pattern": "*.rs"
            })],
        )
        .await?;

    println!("Total matches: {}", search_response["total_matches"]);
    println!("Files searched: {}", search_response["files_searched"]);

    if let Some(results) = search_response["results"].as_array() {
        for (i, result) in results.iter().take(3).enumerate() {
            println!("\nMatch {}:", i + 1);
            println!("  File: {}", result["file_path"]);
            println!(
                "  Line {}: {}",
                result["line_number"], result["line_content"]
            );
        }
    }

    // 5. Semantic search for functions
    println!("\n=== Semantic Search (Functions) ===");
    let semantic_response: serde_json::Value = client
        .request(
            "semantic_search",
            rpc_params![json!({
                "path": "src",
                "query": "analyze",
                "search_type": "functions",
                "max_results": 5
            })],
        )
        .await?;

    println!(
        "Functions analyzed: {}",
        semantic_response["files_analyzed"]
    );

    if let Some(results) = semantic_response["results"].as_array() {
        for result in results.iter().take(3) {
            println!("\nFunction: {}", result["symbol_name"]);
            println!("  File: {}", result["file_path"]);
            println!("  Line: {}", result["line_number"]);
        }
    }

    // 6. Diff two files
    println!("\n=== Diff Files ===");
    // Create two temporary files for demonstration
    let temp_dir = tempfile::TempDir::new()?;
    let file1 = temp_dir.path().join("file1.txt");
    let file2 = temp_dir.path().join("file2.txt");

    std::fs::write(&file1, "Line 1\nLine 2\nLine 3\n")?;
    std::fs::write(&file2, "Line 1\nLine 2 modified\nLine 3\nLine 4\n")?;

    let diff_response: serde_json::Value = client
        .request(
            "diff_files",
            rpc_params![json!({
                "file1_path": file1,
                "file2_path": file2,
                "context_lines": 2
            })],
        )
        .await?;

    println!("Added lines: {}", diff_response["added_lines"]);
    println!("Removed lines: {}", diff_response["removed_lines"]);

    // 7. Process remote repository
    println!("\n=== Process Remote Repository ===");
    println!("Processing https://github.com/octocat/Hello-World...");

    let remote_response: serde_json::Value = client
        .request(
            "process_remote_repo",
            rpc_params![json!({
                "repo_url": "https://github.com/octocat/Hello-World",
                "include_patterns": ["*"],
                "ignore_patterns": [],
                "include_imports": false,
                "max_tokens": 5000
            })],
        )
        .await?;

    println!("Repository: {}", remote_response["repo_name"]);
    println!("Files processed: {}", remote_response["file_count"]);
    println!("Tokens used: {}", remote_response["token_count"]);

    println!("\n=== All examples completed successfully! ===");

    Ok(())
}
