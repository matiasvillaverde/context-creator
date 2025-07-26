//! Basic tests for RMCP server functionality

use context_creator::mcp_server::rmcp_server::ContextCreatorServer;
use rmcp::ServerHandler;

#[test]
fn test_server_info() {
    let server = ContextCreatorServer::new();
    let info = server.get_info();
    
    // Verify server info
    assert!(info.instructions.is_some());
    assert!(info.instructions.unwrap().contains("Context Creator MCP Server"));
    assert!(info.capabilities.tools.is_some());
    assert!(info.capabilities.tools.unwrap().list_changed.is_none());
}

#[test]
fn test_server_creation() {
    let server1 = ContextCreatorServer::new();
    let server2 = server1.clone();
    
    // Both should provide same info
    let info1 = server1.get_info();
    let info2 = server2.get_info();
    
    assert_eq!(info1.instructions, info2.instructions);
}