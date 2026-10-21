//! Nodo MCP server.
//!
//! Speaks Model Context Protocol over stdio using line-delimited JSON-RPC 2.0.
//! Exposes mind-map operations so that an external agent can read and edit
//! maps without a window.
//!
//! The server keeps one open document at a time. Tools that need a document
//! fail with a clear message when none is open, rather than silently creating
//! one.

use nodo_core::{model::MindMap, Editor};
use serde_json::{json, Value};
use std::io::{BufRead, Write};

pub mod setup;
pub mod tools;

/// Protocol revision this server implements.
pub const PROTOCOL_VERSION: &str = "2024-11-05";

/// Server name reported during initialisation.
pub const SERVER_NAME: &str = "nodo-mcp";

/// Server version reported during initialisation.
pub const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Session state shared by all tool calls: at most one open document, edited
/// through the same [`Editor`] the window uses.
#[derive(Default)]
pub struct Session {
    pub editor: Option<Editor>,
}

impl Session {
    pub fn new() -> Self {
        Self::default()
    }

    /// Borrow the open map or produce a protocol error.
    pub fn require(&self) -> Result<&MindMap, String> {
        self.editor
            .as_ref()
            .map(Editor::map)
            .ok_or_else(no_document)
    }

    /// Borrow the open document for editing or produce a protocol error.
    pub fn editor_mut(&mut self) -> Result<&mut Editor, String> {
        self.editor.as_mut().ok_or_else(no_document)
    }
}

fn no_document() -> String {
    "no document is open; call open_map or new_map first".to_string()
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
    let ed = Editor::open_or_import(path).map_err(|e| format!("failed to open {path}: {e}"))?;
    let count = ed.map().node_count();
    session.editor = Some(ed);
    Ok(count)
}

/// Create a fresh map in the session.
pub fn new_into(session: &mut Session, title: &str) -> usize {
    let ed = Editor::new(title);
    let count = ed.map().node_count();
    session.editor = Some(ed);
    count
}

/// Save the open map, optionally to a new path.
pub fn save_from(session: &mut Session, path: Option<&str>) -> Result<String, String> {
    let ed = session.editor_mut()?;
    let saved = ed
        .save(path.map(std::path::Path::new))
        .map_err(|e| match e {
            nodo_core::Error::NoPath => {
                "no path given and the document has never been saved".to_string()
            }
            e => format!("failed to write: {e}"),
        })?;
    Ok(saved.display().to_string())
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
Nodo is a mind-mapping app. A map is a tree of nodes: one root, each node \
with children, and branches drawn either side of the root. Every node has text \
and an optional Markdown note. Loose notes can also sit on the canvas itself, \
apart from any node (add_free_note, set_free_note, remove_free_note).

Read the map with get_map or map_stats first. Prefer the targeted tools \
(add_node, set_text, set_note, set_color, set_side, toggle_fold) over rewriting a \
whole map. Positions are recomputed after every edit; relayout only discards \
positions a person placed by hand.

Save with save_map when done: the person sees the change when the app reloads \
the file.

Node ids are stable integers assigned in creation order; the root is always id 0 \
and cannot be removed. Removing a node removes its whole subtree.

Every mutating tool goes through the same undo history a person uses, so undo \
reverts the last change regardless of who made it. Maps are saved as .nodo \
(readable JSON); save_map with another extension writes .nodo beside it and says where.";

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
            "title": "Nodo",
            "version": SERVER_VERSION
        },
        "instructions": INSTRUCTIONS,
    })
}
