//! MCP tool catalogue and dispatch.
//!
//! Every tool is a thin wrapper over `nodo-core`. Mutating tools go through
//! the session's undo history so that an agent's edits are reversible by the
//! same mechanism a human's are.

use crate::{new_into, open_into, save_from, Session};
use nodo_core::{model::NodeSide, undo::Action};
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
            "Open a mind map from a .minder XML file on disk. A .md (Markdown) or .opml file is imported as a new, unsaved map: headings and list items become topics, other text becomes notes; save_map then needs a .minder path.",
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
            "add_free_note",
            "Put a loose Markdown note on the canvas, not attached to any node (a node's own note is set_note). Without x/y it goes to the right of the map. Returns the note id.",
            json!({
                "type": "object",
                "properties": {
                    "text": { "type": "string", "description": "Markdown text of the note." },
                    "x": { "type": "number", "description": "Left edge, in map units." },
                    "y": { "type": "number", "description": "Top edge, in map units." }
                },
                "required": ["text"]
            }),
        ),
        tool(
            "set_free_note",
            "Change a loose note: its text, and optionally where it sits (x, y) and its size (width, height).",
            json!({
                "type": "object",
                "properties": {
                    "id": { "type": "integer" },
                    "text": { "type": "string" },
                    "x": { "type": "number" },
                    "y": { "type": "number" },
                    "width": { "type": "number" },
                    "height": { "type": "number" }
                },
                "required": ["id"]
            }),
        ),
        tool(
            "remove_free_note",
            "Take a loose note off the canvas.",
            json!({
                "type": "object",
                "properties": { "id": { "type": "integer" } },
                "required": ["id"]
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
            "Place a node's branch on the left or right of the root. Its whole subtree follows.",
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
            "set_map_theme",
            "Choose the map's look, as in XMind: classico (soft filled branches), contorno (outlined), sublinhado (text on a coloured line), vibrante (bold colours), pilulas (rounded pills) or mono (greys). Omit theme for the default.",
            json!({
                "type": "object",
                "properties": {
                    "theme": { "type": "string", "enum": nodo_core::MAP_STYLES }
                }
            }),
        ),
        tool(
            "toggle_fold",
            "Fold a node (hide its children) or unfold it.",
            json!({
                "type": "object",
                "properties": { "id": { "type": "integer" } },
                "required": ["id"]
            }),
        ),
        tool(
            "export_map",
            "Export the open map as text: markdown (nested list with notes), opml (outline for other apps) or html (a self-contained page with the map as an outline). Returns the text; with path, also writes it there. Images (PNG, SVG, PDF) are made in the app window.",
            json!({
                "type": "object",
                "properties": {
                    "format": { "type": "string", "enum": nodo_core::export::TEXT_FORMATS },
                    "path": { "type": "string", "description": "Optional file to write" }
                },
                "required": ["format"]
            }),
        ),
        tool(
            "set_layout",
            "Arrange the map automatically. structure: mapa (branches on both sides of the root), logico (everything to the right), organograma (org chart, top to bottom) or arvore (indented tree). spacing: compacto, normal or largo. Both are saved in the file; omit one to keep it.",
            json!({
                "type": "object",
                "properties": {
                    "structure": { "type": "string", "enum": nodo_core::STRUCTURES },
                    "spacing": { "type": "string", "enum": nodo_core::SPACINGS }
                }
            }),
        ),
        tool(
            "move_sibling",
            "Move a node up or down among its siblings (Alt+Up/Alt+Down in the app). Under the root, it swaps with the nearest branch on the same side.",
            json!({
                "type": "object",
                "properties": {
                    "id": { "type": "integer" },
                    "direction": { "type": "string", "enum": ["up", "down"] }
                },
                "required": ["id", "direction"]
            }),
        ),
        tool(
            "relayout",
            "Discard positions placed by hand and lay the whole map out again. Positions are already recomputed after every edit.",
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
            json!({
                "version": map.version,
                "theme": map.theme.style.as_deref().unwrap_or("classico"),
                "structure": map.theme.structure.as_deref().unwrap_or("mapa"),
                "spacing": map.theme.spacing.as_deref().unwrap_or("normal"),
                "nodes": map.node_count(),
                "tree": tree_json(&map.root),
                "notes": map.notes.iter().map(|n| json!({
                    "id": n.id,
                    "text": n.text,
                    "x": n.pos.x,
                    "y": n.pos.y,
                    "width": n.width,
                    "height": n.height
                })).collect::<Vec<_>>()
            })
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
        "add_free_note" => {
            let text = str_arg(&args, "text")?;
            let map = session.require()?;
            // Sem posição: à direita de tudo, na altura da raiz.
            let right = map
                .nodes()
                .map(|n| n.pos.x + n.width)
                .chain(map.notes.iter().map(|n| n.pos.x + n.width))
                .fold(f64::MIN, f64::max);
            let x = num_arg(&args, "x").unwrap_or(right + 80.0);
            let y = num_arg(&args, "y").unwrap_or(map.root.pos.y);
            let id = mutate(session, Action::AddFreeNote { x, y, text })?;
            json!({ "ok": true, "id": id })
        }
        "set_free_note" => {
            let id = int_arg(&args, "id")?;
            let note = session
                .require()?
                .free_note(id)
                .cloned()
                .ok_or_else(|| format!("note {id} not found"))?;
            if let Some(text) = args.get("text").and_then(|v| v.as_str()) {
                mutate(
                    session,
                    Action::SetFreeNote {
                        id,
                        text: text.to_string(),
                    },
                )?;
            }
            let place = ["x", "y", "width", "height"]
                .iter()
                .any(|k| args.get(*k).is_some());
            if place {
                mutate(
                    session,
                    Action::PlaceFreeNote {
                        id,
                        x: num_arg(&args, "x").unwrap_or(note.pos.x),
                        y: num_arg(&args, "y").unwrap_or(note.pos.y),
                        width: num_arg(&args, "width").unwrap_or(note.width),
                        height: num_arg(&args, "height").unwrap_or(note.height),
                    },
                )?;
            }
            json!({ "ok": true, "id": id })
        }
        "remove_free_note" => {
            let id = int_arg(&args, "id")?;
            mutate(session, Action::RemoveFreeNote { id })?;
            json!({ "ok": true, "removed": id })
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
            mutate(session, Action::SetSide { id, side })?;
            json!({ "ok": true, "id": id, "side": side.as_str() })
        }
        "set_map_theme" => {
            let style = args
                .get("theme")
                .and_then(|t| t.as_str())
                .map(str::to_string);
            mutate(
                session,
                Action::SetMapStyle {
                    style: style.clone(),
                },
            )?;
            json!({ "ok": true, "theme": style.unwrap_or_else(|| "classico".into()) })
        }
        "toggle_fold" => {
            let id = int_arg(&args, "id")?;
            mutate(session, Action::ToggleFold { id })?;
            let folded = session
                .require()?
                .find(id)
                .map(|n| n.folded)
                .unwrap_or(false);
            json!({ "ok": true, "id": id, "folded": folded })
        }
        "export_map" => {
            let format = str_arg(&args, "format")?;
            let text = nodo_core::export::to_text(session.require()?, &format)
                .map_err(|e| e.to_string())?;
            match args.get("path").and_then(|p| p.as_str()) {
                Some(path) => {
                    nodo_core::write_atomic(std::path::Path::new(path), text.as_bytes())
                        .map_err(|e| format!("could not write {path}: {e}"))?;
                    json!({ "ok": true, "path": path, "bytes": text.len() })
                }
                None => json!({ "ok": true, "format": format, "text": text }),
            }
        }
        "set_layout" => {
            // "mapa" e "normal" são o padrão: gravam como ausentes, igual ao que o app faz.
            let pick = |key: &str, default: &str| {
                args.get(key)
                    .and_then(|v| v.as_str())
                    .map(|v| (v != default).then(|| v.to_string()))
            };
            let structure = pick("structure", "mapa");
            let spacing = pick("spacing", "normal");
            if structure.is_none() && spacing.is_none() {
                return Err("pass structure and/or spacing".into());
            }
            if let Some(structure) = structure {
                mutate(session, Action::SetStructure { structure })?;
            }
            if let Some(spacing) = spacing {
                mutate(session, Action::SetSpacing { spacing })?;
            }
            let theme = &session.require()?.theme;
            json!({
                "ok": true,
                "structure": theme.structure.clone().unwrap_or_else(|| "mapa".into()),
                "spacing": theme.spacing.clone().unwrap_or_else(|| "normal".into())
            })
        }
        "move_sibling" => {
            let id = int_arg(&args, "id")?;
            let delta = match str_arg(&args, "direction")?.as_str() {
                "up" => -1,
                "down" => 1,
                other => {
                    return Err(format!(
                        "direction must be \"up\" or \"down\", got {other:?}"
                    ))
                }
            };
            mutate(session, Action::Reorder { id, delta })?;
            json!({ "ok": true, "id": id })
        }
        "relayout" => {
            mutate(session, Action::Relayout)?;
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
            let ed = session.editor_mut()?;
            if ed.undo() {
                json!({ "ok": true, "undo_depth": ed.undo_depth() })
            } else {
                json!({ "ok": false, "message": "nothing to undo" })
            }
        }
        "redo" => {
            let ed = session.editor_mut()?;
            if ed.redo() {
                json!({ "ok": true, "redo_depth": ed.redo_depth() })
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
                "dirty": session.editor.as_ref().is_some_and(|e| e.is_dirty()),
                "path": session.editor.as_ref().and_then(|e| e.path()).map(|p| p.display().to_string()),
                "undo_depth": session.editor.as_ref().map(|e| e.undo_depth()).unwrap_or(0)
            })
        }
        other => return Err(format!("unknown tool: {other}")),
    };

    Ok(json!({
        "content": [{ "type": "text", "text": serde_json::to_string_pretty(&text).unwrap_or_default() }]
    }))
}

/// Apply a mutating action through the editor's undo history.
fn mutate(session: &mut Session, action: Action) -> Result<u64, String> {
    let id = session
        .editor_mut()?
        .apply(action)
        .map_err(|e| e.to_string())?;
    Ok(id.unwrap_or(0))
}

fn node_json(n: &nodo_core::model::Node) -> Value {
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

fn tree_json(n: &nodo_core::model::Node) -> Value {
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

fn num_arg(args: &Value, key: &str) -> Option<f64> {
    args.get(key).and_then(|v| v.as_f64())
}

fn int_arg(args: &Value, key: &str) -> Result<u64, String> {
    args.get(key)
        .and_then(|v| v.as_u64())
        .ok_or_else(|| format!("missing required integer argument: {key}"))
}
