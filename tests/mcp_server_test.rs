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

// Helper function to start test server
async fn start_test_server(addr: &str) -> Result<context_creator::mcp_server::ServerHandle> {
    context_creator::mcp_server::start_server(addr).await
}
