//! MCP tool catalogue and dispatch.
//!
//! Every tool is a thin wrapper over `mapist-core`. Mutating tools go through
//! the session's undo history so that an agent's edits are reversible by the
//! same mechanism a human's are.

use crate::{new_into, open_into, save_from, Session};
use mapist_core::{layout, model::NodeSide, undo::Action};
use serde_json::{json, Value};

/// The tools this server advertises, in `tools/list` order.
pub fn catalog() -> Vec<Value> {
    vec![
        tool(
            "new_map",
            "Create a new empty mind map with a single root node. Replaces any open document.",
            json!({
                "type": "object",
                "properties": {
                    "title": { "type": "string", "description": "Text of the root node." }
                },
                "required": ["title"]
            }),
        ),
        tool(
            "open_map",
            "Open a mind map from a .minder XML file on disk.",
            json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Absolute path to the file." }
                },
                "required": ["path"]
            }),
        ),
        tool(
            "save_map",
            "Save the open map. Without a path, writes back to the file it was opened from.",
            json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Optional new path." }
                }
            }),
        ),
        tool(
            "get_map",
            "Return the whole map as a nested tree of nodes.",
            json!({ "type": "object", "properties": {} }),
        ),
        tool(
            "get_node",
            "Return one node and its immediate children by id.",
            json!({
                "type": "object",
                "properties": { "id": { "type": "integer" } },
                "required": ["id"]
            }),
        ),
        tool(
            "add_node",
            "Add a child node under an existing node. Returns the new node id.",
            json!({
                "type": "object",
                "properties": {
                    "parent": { "type": "integer", "description": "Id of the parent node." },
                    "text": { "type": "string", "description": "Text of the new node." }
                },
                "required": ["parent", "text"]
            }),
        ),
        tool(
            "remove_node",
            "Remove a node and its entire subtree. The root cannot be removed.",
            json!({
                "type": "object",
                "properties": { "id": { "type": "integer" } },
                "required": ["id"]
            }),
        ),
        tool(
            "set_text",
            "Replace a node's text.",
            json!({
                "type": "object",
                "properties": {
                    "id": { "type": "integer" },
                    "text": { "type": "string" }
                },
                "required": ["id", "text"]
            }),
        ),
        tool(
            "set_note",
            "Replace a node's note.",
            json!({
                "type": "object",
                "properties": {
                    "id": { "type": "integer" },
                    "note": { "type": "string" }
                },
                "required": ["id", "note"]
            }),
        ),
        tool(
            "set_color",
            "Set a node's branch colour as #rrggbb, or null to clear it.",
            json!({
                "type": "object",
                "properties": {
                    "id": { "type": "integer" },
                    "color": { "type": ["string", "null"] }
                },
                "required": ["id", "color"]
            }),
        ),
        tool(
            "set_side",
            "Place a node's branch on the left or right of the root.",
            json!({
                "type": "object",
                "properties": {
                    "id": { "type": "integer" },
                    "side": { "type": "string", "enum": ["left", "right"] }
                },
                "required": ["id", "side"]
            }),
        ),
        tool(
            "relayout",
            "Recompute node positions after structural changes.",
            json!({ "type": "object", "properties": {} }),
        ),
        tool(
            "search_nodes",
            "Find nodes whose text or note contains a substring (case-insensitive).",
            json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string" },
                    "limit": { "type": "integer", "description": "Max results, default 50." }
                },
                "required": ["query"]
            }),
        ),
        tool(
            "undo",
            "Undo the last mutating tool call.",
            json!({ "type": "object", "properties": {} }),
        ),
        tool(
            "redo",
            "Redo the last undone change.",
            json!({ "type": "object", "properties": {} }),
        ),
        tool(
            "map_stats",
            "Report node count, depth, and whether the document has unsaved changes.",
            json!({ "type": "object", "properties": {} }),
        ),
    ]
}

fn tool(name: &str, description: &str, input_schema: Value) -> Value {
    json!({ "name": name, "description": description, "inputSchema": input_schema })
}

/// Dispatch one `tools/call` request.
pub fn call(session: &mut Session, params: &Value) -> Result<Value, String> {
    let name = params
        .get("name")
        .and_then(|n| n.as_str())
        .ok_or_else(|| "missing tool name".to_string())?;
    let args = params.get("arguments").cloned().unwrap_or(json!({}));

    let text = match name {
        "new_map" => {
            let title = str_arg(&args, "title")?;
            let count = new_into(session, &title);
            json!({ "ok": true, "nodes": count, "message": format!("new map with root {title:?}") })
        }
        "open_map" => {
            let path = str_arg(&args, "path")?;
            let count = open_into(session, &path)?;
            json!({ "ok": true, "path": path, "nodes": count })
        }
        "save_map" => {
            let path = args.get("path").and_then(|p| p.as_str());
            let saved = save_from(session, path)?;
            json!({ "ok": true, "path": saved })
        }
        "get_map" => {
            let map = session.require()?;
            json!({ "version": map.version, "nodes": map.node_count(), "tree": tree_json(&map.root) })
        }
        "get_node" => {
            let id = int_arg(&args, "id")?;
            let map = session.require()?;
            let node = map.find(id).ok_or_else(|| format!("node {id} not found"))?;
            json!({ "node": node_json(node), "depth": map.depth_of(id) })
        }
        "add_node" => {
            let parent = int_arg(&args, "parent")?;
            let text = str_arg(&args, "text")?;
            let id = mutate(session, Action::AddChild { parent, text })?;
            json!({ "ok": true, "id": id })
        }
        "remove_node" => {
            let id = int_arg(&args, "id")?;
            mutate(session, Action::RemoveNode { id })?;
            json!({ "ok": true, "removed": id })
        }
        "set_text" => {
            let id = int_arg(&args, "id")?;
            let text = str_arg(&args, "text")?;
            mutate(session, Action::SetText { id, text })?;
            json!({ "ok": true, "id": id })
        }
        "set_note" => {
            let id = int_arg(&args, "id")?;
            let note = str_arg(&args, "note")?;
            mutate(session, Action::SetNote { id, note })?;
            json!({ "ok": true, "id": id })
        }
        "set_color" => {
            let id = int_arg(&args, "id")?;
            let color = args
                .get("color")
                .and_then(|c| c.as_str())
                .map(|s| s.to_string());
            mutate(session, Action::SetColor { id, color })?;
            json!({ "ok": true, "id": id })
        }
        "set_side" => {
            let id = int_arg(&args, "id")?;
            let side = match str_arg(&args, "side")?.as_str() {
                "left" => NodeSide::Left,
                "right" => NodeSide::Right,
                other => return Err(format!("side must be left or right, got {other:?}")),
            };
            let map = session.require_mut()?;
            let node = map
                .find_mut(id)
                .ok_or_else(|| format!("node {id} not found"))?;
            node.side = side;
            session.dirty = true;
            json!({ "ok": true, "id": id, "side": side.as_str() })
        }
        "relayout" => {
            let map = session.require_mut()?;
            layout::layout(map);
            session.dirty = true;
            json!({ "ok": true })
        }
        "search_nodes" => {
            let query = str_arg(&args, "query")?.to_lowercase();
            let limit = args.get("limit").and_then(|l| l.as_u64()).unwrap_or(50) as usize;
            let map = session.require()?;
            let hits: Vec<Value> = map
                .nodes()
                .filter(|n| {
                    n.text.to_lowercase().contains(&query)
                        || n.note.to_lowercase().contains(&query)
                })
                .take(limit)
                .map(|n| {
                    json!({
                        "id": n.id,
                        "text": n.text,
                        "depth": map.depth_of(n.id).unwrap_or(0),
                        "match": if n.text.to_lowercase().contains(&query) { "text" } else { "note" }
                    })
                })
                .collect();
            json!({ "query": query, "count": hits.len(), "results": hits })
        }
        "undo" => {
            let hist = session
                .history
                .as_mut()
                .ok_or_else(|| "no document is open".to_string())?;
            if hist.undo() {
                session.map = Some(hist.current().clone());
                session.dirty = true;
                json!({ "ok": true, "undo_depth": hist.undo_depth() })
            } else {
                json!({ "ok": false, "message": "nothing to undo" })
            }
        }
        "redo" => {
            let hist = session
                .history
                .as_mut()
                .ok_or_else(|| "no document is open".to_string())?;
            if hist.redo() {
                session.map = Some(hist.current().clone());
                session.dirty = true;
                json!({ "ok": true, "redo_depth": hist.redo_depth() })
            } else {
                json!({ "ok": false, "message": "nothing to redo" })
            }
        }
        "map_stats" => {
            let map = session.require()?;
            let max_depth = map
                .nodes()
                .map(|n| map.depth_of(n.id).unwrap_or(0))
                .max()
                .unwrap_or(0);
            json!({
                "nodes": map.node_count(),
                "max_depth": max_depth,
                "dirty": session.dirty,
                "path": session.path,
                "undo_depth": session.history.as_ref().map(|h| h.undo_depth()).unwrap_or(0)
            })
        }
        other => return Err(format!("unknown tool: {other}")),
    };

    Ok(json!({
        "content": [{ "type": "text", "text": serde_json::to_string_pretty(&text).unwrap_or_default() }]
    }))
}

/// Apply a mutating action through the undo history and keep the session map in
/// sync with it.
fn mutate(session: &mut Session, action: Action) -> Result<u64, String> {
    let hist = session
        .history
        .as_mut()
        .ok_or_else(|| "no document is open".to_string())?;
    let id = hist.apply(action).map_err(|e| e.to_string())?.unwrap_or(0);
    let mut updated = hist.current().clone();
    layout::layout(&mut updated);
    session.map = Some(updated);
    session.dirty = true;
    Ok(id)
}

fn node_json(n: &mapist_core::model::Node) -> Value {
    json!({
        "id": n.id,
        "text": n.text,
        "note": n.note,
        "side": n.side.as_str(),
        "layout": n.layout.as_str(),
        "color": n.color,
        "folded": n.folded,
        "x": n.pos.x,
        "y": n.pos.y,
        "width": n.width,
        "height": n.height,
        "children": n.children.iter().map(|c| c.id).collect::<Vec<_>>()
    })
}

fn tree_json(n: &mapist_core::model::Node) -> Value {
    json!({
        "id": n.id,
        "text": n.text,
        "side": n.side.as_str(),
        "color": n.color,
        "children": n.children.iter().map(tree_json).collect::<Vec<_>>()
    })
}

fn str_arg(args: &Value, key: &str) -> Result<String, String> {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| format!("missing required string argument: {key}"))
}

fn int_arg(args: &Value, key: &str) -> Result<u64, String> {
    args.get(key)
        .and_then(|v| v.as_u64())
        .ok_or_else(|| format!("missing required integer argument: {key}"))
}
