//! Mapist MCP server.
//!
//! Speaks Model Context Protocol over stdio using line-delimited JSON-RPC 2.0.
//! Exposes mind-map operations so that an external agent can read and edit
//! maps without a window.
//!
//! The server keeps one open document at a time. Tools that need a document
//! fail with a clear message when none is open, rather than silently creating
//! one.

use mapist_core::{document, layout, model::MindMap, undo};
use serde_json::{json, Value};
use std::io::{BufRead, Write};

pub mod setup;
pub mod tools;

/// Protocol revision this server implements.
pub const PROTOCOL_VERSION: &str = "2024-11-05";

/// Server name reported during initialisation.
pub const SERVER_NAME: &str = "mapist-mcp";

/// Server version reported during initialisation.
pub const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Session state shared by all tool calls.
pub struct Session {
    /// The open document, if any.
    pub map: Option<MindMap>,
    /// Path the open document was loaded from or last saved to.
    pub path: Option<String>,
    /// Whether the document has unsaved edits.
    pub dirty: bool,
    /// Undo history for the open document.
    pub history: Option<undo::History>,
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

impl Session {
    pub fn new() -> Self {
        Self {
            map: None,
            path: None,
            dirty: false,
            history: None,
        }
    }

    /// Borrow the open map or produce a protocol error.
    pub fn require(&self) -> Result<&MindMap, String> {
        self.map
            .as_ref()
            .ok_or_else(|| "no document is open; call open_map or new_map first".to_string())
    }

    /// Borrow the open map mutably or produce a protocol error.
    pub fn require_mut(&mut self) -> Result<&mut MindMap, String> {
        self.map
            .as_mut()
            .ok_or_else(|| "no document is open; call open_map or new_map first".to_string())
    }
}

pub fn success(id: Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

pub fn error_response(id: Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

pub fn write_msg(out: &mut impl Write, msg: &Value) -> std::io::Result<()> {
    let text = serde_json::to_string(msg).unwrap_or_else(|_| "{}".to_string());
    out.write_all(text.as_bytes())?;
    out.write_all(b"\n")?;
    out.flush()
}

/// Open a map from disk into the session.
pub fn open_into(session: &mut Session, path: &str) -> Result<usize, String> {
    let map = mapist_core::load(path).map_err(|e| format!("failed to open {path}: {e}"))?;
    let count = map.node_count();
    session.history = Some(undo::History::new(map.clone()));
    session.map = Some(map);
    session.path = Some(path.to_string());
    session.dirty = false;
    Ok(count)
}

/// Create a fresh map in the session.
pub fn new_into(session: &mut Session, title: &str) -> usize {
    let mut map = MindMap::new(title);
    layout::layout(&mut map);
    let count = map.node_count();
    session.history = Some(undo::History::new(map.clone()));
    session.map = Some(map);
    session.path = None;
    session.dirty = false;
    count
}

/// Save the open map, optionally to a new path.
pub fn save_from(session: &mut Session, path: Option<&str>) -> Result<String, String> {
    let target = match path {
        Some(p) => p.to_string(),
        None => session
            .path
            .clone()
            .ok_or_else(|| "no path given and the document has never been saved".to_string())?,
    };
    let map = session.require()?;
    document::to_xml(map)
        .map_err(|e| format!("serialize failed: {e}"))
        .and_then(|xml| {
            std::fs::write(&target, xml).map_err(|e| format!("failed to write {target}: {e}"))
        })?;
    session.path = Some(target.clone());
    session.dirty = false;
    Ok(target)
}

/// Run the MCP server over stdio until the client closes its end.
///
/// Returns the process exit code. This is the whole `--mcp` mode: the same
/// executable that opens the window can instead speak JSON-RPC, which keeps
/// the installer to one binary and guarantees both modes share this crate.
pub fn serve() -> i32 {
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
        let params = request.get("params").cloned().unwrap_or(json!({}));

        let response = match method {
            "initialize" => success(id, initialize_result(&params)),
            "tools/list" => success(id, json!({ "tools": tools::catalog() })),
            "tools/call" => match tools::call(&mut session, &params) {
                Ok(result) => success(id, result),
                Err(msg) => success(
                    id,
                    json!({
                        "content": [{ "type": "text", "text": msg }],
                        "isError": true
                    }),
                ),
            },
            "ping" => success(id, json!({})),
            other => error_response(id, -32601, &format!("method not found: {other}")),
        };

        if write_msg(&mut stdout, &response).is_err() {
            break;
        }
    }
    0
}

/// Protocol versions this server speaks, newest first.
const PROTOCOLS: [&str; 3] = ["2025-06-18", "2025-03-26", PROTOCOL_VERSION];

/// Guidance sent to the client once, describing what this app is and how to
/// use it well. Agents behave noticeably better with this than without it.
const INSTRUCTIONS: &str = "\
Mapist is a mind-mapping app. A map is a tree of nodes: one root, each node \
with children, and branches drawn either side of the root. Every node has text \
and an optional Markdown note.

Read the map with get_map or map_stats first. Prefer the targeted tools \
(add_node, set_text, set_note, set_color, set_side) over rewriting a whole map. \
After changing structure, call relayout so positions are recomputed.

Node ids are stable integers assigned in creation order; the root is always id 0 \
and cannot be removed. Removing a node removes its whole subtree.

Every mutating tool goes through the same undo history a person uses, so undo \
reverts the last change regardless of who made it. Files are the Minder XML \
format (.minder), so maps stay portable.";

/// The `initialize` result, including the instructions above.
fn initialize_result(params: &Value) -> Value {
    let asked = params
        .get("protocolVersion")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let version = PROTOCOLS
        .iter()
        .find(|v| **v == asked)
        .copied()
        .unwrap_or(PROTOCOLS[0]);
    json!({
        "protocolVersion": version,
        "capabilities": { "tools": { "listChanged": false } },
        "serverInfo": {
            "name": SERVER_NAME,
            "title": "Mapist",
            "version": SERVER_VERSION
        },
        "instructions": INSTRUCTIONS,
    })
}
