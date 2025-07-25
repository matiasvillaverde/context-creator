//! MCP (Model Context Protocol) server implementation for context-creator
//!
//! This module provides a JSON-RPC server that allows AI agents to
//! analyze codebases programmatically.

use anyhow::Result;
use jsonrpsee::{
    core::RpcResult,
    proc_macros::rpc,
    server::{Server, ServerHandle as JsonRpcServerHandle},
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

pub mod handlers;

/// Health check response structure
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: u64,
    pub version: String,
}

/// RPC trait for health check
#[rpc(server)]
pub trait HealthRpc {
    /// Returns the current health status of the server
    #[method(name = "health_check")]
    async fn health_check(&self) -> RpcResult<HealthResponse>;
}

/// Server handle wrapper for managing the MCP server lifecycle
pub struct ServerHandle {
    inner: JsonRpcServerHandle,
    local_addr: SocketAddr,
}

impl ServerHandle {
    /// Get the local address the server is listening on
    pub fn local_addr(&self) -> Result<SocketAddr> {
        Ok(self.local_addr)
    }

    /// Stop the server gracefully
    pub fn stop(self) -> Result<()> {
        self.inner.stop()?;
        Ok(())
    }
}

/// Start the MCP server on the specified address
pub async fn start_server(addr: &str) -> Result<ServerHandle> {
    let addr: SocketAddr = addr.parse()?;

    // Build the server
    let server = Server::builder().build(addr).await?;

    // Get the actual address (in case port 0 was used)
    let local_addr = server.local_addr()?;

    // Create and register the health check handler
    let health_impl = handlers::HealthRpcImpl;
    let rpc_module = health_impl.into_rpc();

    // Start the server in the background
    let handle = server.start(rpc_module);

    Ok(ServerHandle {
        inner: handle,
        local_addr,
    })
}
