//! Mapist MCP server entry point.
//!
//! Reads line-delimited JSON-RPC 2.0 from stdin and writes responses to
//! stdout. All behaviour lives in the library so it can be tested without
//! spawning a process.

use mapist_mcp::{error_response, success, write_msg, Session};
use serde_json::Value;
use std::io::BufRead;

fn main() {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    let mut session = Session::new();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        if line.trim().is_empty() {
            continue;
        }

        let request: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                let _ = write_msg(
                    &mut stdout,
                    &error_response(Value::Null, -32700, &format!("parse error: {e}")),
                );
                continue;
            }
        };

        // Notifications carry no id and expect no reply.
        let id = match request.get("id").cloned() {
            Some(id) => id,
            None => continue,
        };
        let method = request.get("method").and_then(|m| m.as_str()).unwrap_or("");
        let params = request
            .get("params")
            .cloned()
            .unwrap_or(serde_json::json!({}));

        let response = match method {
            "initialize" => success(
                id,
                serde_json::json!({
                    "protocolVersion": mapist_mcp::PROTOCOL_VERSION,
                    "capabilities": { "tools": {} },
                    "serverInfo": {
                        "name": mapist_mcp::SERVER_NAME,
                        "version": mapist_mcp::SERVER_VERSION
                    }
                }),
            ),
            "tools/list" => success(
                id,
                serde_json::json!({ "tools": mapist_mcp::tools::catalog() }),
            ),
            "tools/call" => match mapist_mcp::tools::call(&mut session, &params) {
                Ok(result) => success(id, result),
                Err(msg) => success(
                    id,
                    serde_json::json!({
                        "content": [{ "type": "text", "text": msg }],
                        "isError": true
                    }),
                ),
            },
            "ping" => success(id, serde_json::json!({})),
            other => error_response(id, -32601, &format!("method not found: {other}")),
        };

        if write_msg(&mut stdout, &response).is_err() {
            break;
        }
    }
}
