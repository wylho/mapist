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
use std::io::Write;

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
