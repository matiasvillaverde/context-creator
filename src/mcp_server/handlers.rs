//! RPC method handlers for the MCP server

use super::{HealthResponse, HealthRpcServer};
use jsonrpsee::core::RpcResult;
use std::time::SystemTime;

/// Implementation of health check RPC methods
pub struct HealthRpcImpl;

#[jsonrpsee::core::async_trait]
impl HealthRpcServer for HealthRpcImpl {
    async fn health_check(&self) -> RpcResult<HealthResponse> {
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|e| {
                jsonrpsee::types::ErrorObject::owned(-32000, "Timestamp error", Some(e.to_string()))
            })?
            .as_secs();

        Ok(HealthResponse {
            status: "healthy".to_string(),
            timestamp,
            version: env!("CARGO_PKG_VERSION").to_string(),
        })
    }
}
