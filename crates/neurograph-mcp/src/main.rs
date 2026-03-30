// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! NeuroGraph MCP Server — Model Context Protocol for AI agent integration.
//!
//! Exposes NeuroGraph's knowledge graph capabilities as MCP tools that can be
//! called by Claude Desktop, Cursor, VS Code Copilot, and any MCP-compatible client.
//!
//! ## Architecture
//!
//! ```text
//! ┌──────────────────────────────┐
//! │  Claude / Cursor / Copilot   │
//! └────────────┬─────────────────┘
//!              │ JSON-RPC over stdio
//! ┌────────────▼─────────────────┐
//! │    neurograph-mcp (this)     │
//! │  tools.rs → NeuroGraph Core  │
//! └────────────┬─────────────────┘
//!              │
//! ┌────────────▼─────────────────┐
//! │     neurograph-core          │
//! │  memory · graphs · temporal  │
//! └──────────────────────────────┘
//! ```
//!
//! ## Usage
//!
//! ```bash
//! # Run directly
//! cargo run -p neurograph-mcp
//!
//! # Via Docker
//! docker run -i --rm neurograph/mcp-server
//! ```

mod tools;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{self, BufRead, Write};

/// JSON-RPC 2.0 request.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    params: Option<Value>,
}

/// JSON-RPC 2.0 response.
#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

/// MCP Server implementation.
struct McpServer {
    handler: tools::ToolHandler,
}

impl McpServer {
    fn new() -> Self {
        Self {
            handler: tools::ToolHandler::new(),
        }
    }

    /// Handle a single JSON-RPC request.
    async fn handle_request(&self, req: JsonRpcRequest) -> JsonRpcResponse {
        let result = match req.method.as_str() {
            "initialize" => self.handle_initialize(),
            "tools/list" => self.handle_list_tools(),
            "tools/call" => self.handle_call_tool(req.params).await,
            "notifications/initialized" => {
                return JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: req.id,
                    result: Some(Value::Null),
                    error: None,
                };
            }
            _ => Err((-32601, format!("Method not found: {}", req.method))),
        };

        match result {
            Ok(value) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: req.id,
                result: Some(value),
                error: None,
            },
            Err((code, msg)) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: req.id,
                result: None,
                error: Some(JsonRpcError {
                    code,
                    message: msg,
                    data: None,
                }),
            },
        }
    }

    fn handle_initialize(&self) -> Result<Value, (i32, String)> {
        Ok(serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {}
            },
            "serverInfo": {
                "name": "neurograph-mcp",
                "version": env!("CARGO_PKG_VERSION")
            }
        }))
    }

    fn handle_list_tools(&self) -> Result<Value, (i32, String)> {
        let tools = self.handler.list_tools();
        Ok(serde_json::json!({ "tools": tools }))
    }

    async fn handle_call_tool(&self, params: Option<Value>) -> Result<Value, (i32, String)> {
        let params = params.ok_or((-32602, "Missing params".to_string()))?;
        let tool_name = params
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or((-32602, "Missing tool name".to_string()))?;
        let arguments = params
            .get("arguments")
            .cloned()
            .unwrap_or(Value::Object(serde_json::Map::new()));

        self.handler
            .call(tool_name, arguments)
            .await
            .map_err(|e| (-32000, e))
    }
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_writer(io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("neurograph=info".parse().unwrap()),
        )
        .init();

    tracing::info!("NeuroGraph MCP Server starting...");

    let server = McpServer::new();
    let stdin = io::stdin();
    let stdout = io::stdout();

    // JSON-RPC over stdio: read line-delimited JSON
    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                tracing::error!("Failed to read stdin: {}", e);
                break;
            }
        };

        if line.trim().is_empty() {
            continue;
        }

        let request: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(req) => req,
            Err(e) => {
                let err = JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: None,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32700,
                        message: format!("Parse error: {}", e),
                        data: None,
                    }),
                };
                let mut out = stdout.lock();
                let _ = serde_json::to_writer(&mut out, &err);
                let _ = writeln!(out);
                let _ = out.flush();
                continue;
            }
        };

        let response = server.handle_request(request).await;

        let mut out = stdout.lock();
        let _ = serde_json::to_writer(&mut out, &response);
        let _ = writeln!(out);
        let _ = out.flush();
    }

    tracing::info!("NeuroGraph MCP Server shutting down.");
}
