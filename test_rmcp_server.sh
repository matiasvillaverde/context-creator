#!/bin/bash

# Test script for RMCP MCP server

echo "Building context-creator with RMCP support..."
cargo build --release

echo -e "\n=== Testing RMCP Server with stdio transport ==="
echo "Starting server with: ./target/release/context-creator --rmcp"
echo "This will use stdio transport by default"
echo ""
echo "To test, use the MCP inspector:"
echo "npx @modelcontextprotocol/inspector ./target/release/context-creator --rmcp"
echo ""
echo "Alternative test with direct stdio communication:"
echo "You can also pipe JSON-RPC messages directly"

# Create a test request for server info
cat > /tmp/test_request.json << 'EOF'
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "initialize",
  "params": {
    "protocolVersion": "0.1.0",
    "capabilities": {}
  }
}
EOF

echo -e "\nTest request saved to /tmp/test_request.json"
echo "To test manually: cat /tmp/test_request.json | ./target/release/context-creator --rmcp"