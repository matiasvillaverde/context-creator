#!/usr/bin/env bash
set -euo pipefail

# Verify current and legacy MCP protocol behavior without external LLM access.
cargo test --locked --test rmcp_basic_test --test rmcp_integration_test

printf '%s\n' \
  'Stdio: ./target/debug/context-creator --rmcp' \
  'HTTP: ./target/debug/context-creator --rmcp --rmcp-transport http --mcp-port 9090' \
  'HTTP endpoint: http://127.0.0.1:9090/mcp' \
  'Inspector: npx @modelcontextprotocol/inspector ./target/debug/context-creator --rmcp'
