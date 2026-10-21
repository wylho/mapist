//! End-to-end tests for the MCP tool surface.
//!
//! These drive the same functions `main` dispatches to, so they cover the
//! protocol-facing behaviour without spawning a process.

use serde_json::json;

// The binary crate exposes its modules for integration testing via the
// `nodo_mcp` lib target; this file builds against that target.
use nodo_mcp::{new_into, open_into, save_from, tools, Session};

fn fresh() -> Session {
    let mut s = Session::default();
    new_into(&mut s, "Root");
    s
}

fn call(session: &mut Session, name: &str, args: serde_json::Value) -> serde_json::Value {
    let params = json!({ "name": name, "arguments": args });
    let result = tools::call(session, &params).expect("tool call failed");
    let text = result["content"][0]["text"]
        .as_str()
        .expect("tool result has text content");
    serde_json::from_str(text).expect("tool result is JSON")
}

#[test]
fn catalog_is_non_empty_and_well_formed() {
    let cat = tools::catalog();
    assert!(cat.len() >= 10, "expected a useful tool surface");
    for t in &cat {
        assert!(t["name"].is_string(), "every tool needs a name");
        assert!(
            t["description"].is_string(),
            "every tool needs a description"
        );
        assert_eq!(t["inputSchema"]["type"], "object");
    }
}

#[test]
fn catalog_names_are_unique() {
    let cat = tools::catalog();
    let mut names: Vec<&str> = cat.iter().filter_map(|t| t["name"].as_str()).collect();
    let total = names.len();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), total, "duplicate tool names");
}

#[test]
fn new_map_reports_one_node() {
    let mut s = Session::default();
    let out = call(&mut s, "new_map", json!({ "title": "Hello" }));
    assert_eq!(out["nodes"], 1);
    assert_eq!(out["ok"], true);
}

#[test]
fn add_node_assigns_increasing_ids() {
    let mut s = fresh();
    let a = call(&mut s, "add_node", json!({ "parent": 0, "text": "A" }));
    let b = call(&mut s, "add_node", json!({ "parent": 0, "text": "B" }));
    assert_eq!(a["id"], 1);
    assert_eq!(b["id"], 2);
}

#[test]
fn add_node_rejects_missing_parent() {
    let mut s = fresh();
    let err = tools::call(
        &mut s,
        &json!({ "name": "add_node", "arguments": { "parent": 999, "text": "x" } }),
    )
    .unwrap_err();
    assert!(err.contains("not found"), "got: {err}");
}

#[test]
fn set_text_and_search_round_trip() {
    let mut s = fresh();
    let a = call(&mut s, "add_node", json!({ "parent": 0, "text": "old" }));
    let id = a["id"].as_u64().unwrap();
    call(&mut s, "set_text", json!({ "id": id, "text": "needle" }));
    let found = call(&mut s, "search_nodes", json!({ "query": "NEEDLE" }));
    assert_eq!(found["count"], 1, "search should be case-insensitive");
    assert_eq!(found["results"][0]["id"], id);
}

#[test]
fn search_can_match_notes() {
    let mut s = fresh();
    let a = call(&mut s, "add_node", json!({ "parent": 0, "text": "plain" }));
    let id = a["id"].as_u64().unwrap();
    call(
        &mut s,
        "set_note",
        json!({ "id": id, "note": "buried treasure" }),
    );
    let found = call(&mut s, "search_nodes", json!({ "query": "treasure" }));
    assert_eq!(found["count"], 1);
    assert_eq!(found["results"][0]["match"], "note");
}

#[test]
fn set_color_accepts_and_clears() {
    let mut s = fresh();
    let a = call(&mut s, "add_node", json!({ "parent": 0, "text": "A" }));
    let id = a["id"].as_u64().unwrap();
    call(&mut s, "set_color", json!({ "id": id, "color": "#c6262e" }));
    let n = call(&mut s, "get_node", json!({ "id": id }));
    assert_eq!(n["node"]["color"], "#c6262e");
    call(&mut s, "set_color", json!({ "id": id, "color": null }));
    let n = call(&mut s, "get_node", json!({ "id": id }));
    assert!(n["node"]["color"].is_null());
}

#[test]
fn set_side_validates_input() {
    let mut s = fresh();
    let a = call(&mut s, "add_node", json!({ "parent": 0, "text": "A" }));
    let id = a["id"].as_u64().unwrap();
    call(&mut s, "set_side", json!({ "id": id, "side": "left" }));
    let n = call(&mut s, "get_node", json!({ "id": id }));
    assert_eq!(n["node"]["side"], "left");

    let err = tools::call(
        &mut s,
        &json!({ "name": "set_side", "arguments": { "id": id, "side": "up" } }),
    )
    .unwrap_err();
    assert!(err.contains("left or right"), "got: {err}");
}

#[test]
fn remove_node_drops_subtree() {
    let mut s = fresh();
    let a = call(&mut s, "add_node", json!({ "parent": 0, "text": "A" }));
    let id = a["id"].as_u64().unwrap();
    call(&mut s, "add_node", json!({ "parent": id, "text": "A1" }));
    call(&mut s, "remove_node", json!({ "id": id }));
    let stats = call(&mut s, "map_stats", json!({}));
    assert_eq!(stats["nodes"], 1, "subtree should be gone");
}

#[test]
fn root_cannot_be_removed() {
    let mut s = fresh();
    let err = tools::call(
        &mut s,
        &json!({ "name": "remove_node", "arguments": { "id": 0 } }),
    )
    .unwrap_err();
    assert!(err.contains("root"), "got: {err}");
}

#[test]
fn undo_and_redo_track_history() {
    let mut s = fresh();
    call(&mut s, "add_node", json!({ "parent": 0, "text": "A" }));
    let before = call(&mut s, "map_stats", json!({}));
    assert_eq!(before["nodes"], 2);

    call(&mut s, "undo", json!({}));
    let undone = call(&mut s, "map_stats", json!({}));
    assert_eq!(undone["nodes"], 1);

    call(&mut s, "redo", json!({}));
    let redone = call(&mut s, "map_stats", json!({}));
    assert_eq!(redone["nodes"], 2);
}

#[test]
fn undo_on_empty_history_reports_false() {
    let mut s = fresh();
    let out = call(&mut s, "undo", json!({}));
    assert_eq!(out["ok"], false);
}

#[test]
fn tools_require_an_open_document() {
    let mut s = Session::default();
    let err = tools::call(&mut s, &json!({ "name": "get_map", "arguments": {} })).unwrap_err();
    assert!(err.contains("no document is open"), "got: {err}");
}

#[test]
fn unknown_tool_is_rejected() {
    let mut s = fresh();
    let err = tools::call(&mut s, &json!({ "name": "nope", "arguments": {} })).unwrap_err();
    assert!(err.contains("unknown tool"), "got: {err}");
}

#[test]
fn save_then_open_round_trips() {
    let dir = std::env::temp_dir().join("nodo-mcp-test");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("roundtrip.minder");
    let path_str = path.to_str().unwrap();

    let mut s = fresh();
    let a = call(
        &mut s,
        "add_node",
        json!({ "parent": 0, "text": "Persisted" }),
    );
    let id = a["id"].as_u64().unwrap();
    call(&mut s, "set_note", json!({ "id": id, "note": "a note" }));
    call(&mut s, "save_map", json!({ "path": path_str }));

    let mut s2 = Session::default();
    let opened = open_into(&mut s2, path_str).unwrap();
    assert_eq!(opened, 2);
    let n = call(&mut s2, "get_node", json!({ "id": id }));
    assert_eq!(n["node"]["text"], "Persisted");
    assert_eq!(n["node"]["note"], "a note");

    let _ = std::fs::remove_file(&path);
}

#[test]
fn save_without_path_or_file_errors() {
    let mut s = fresh();
    let err = save_from(&mut s, None).unwrap_err();
    assert!(err.contains("never been saved"), "got: {err}");
}

#[test]
fn open_missing_file_reports_path() {
    let mut s = Session::default();
    let err = open_into(&mut s, "/nonexistent/nope.minder").unwrap_err();
    assert!(err.contains("/nonexistent/nope.minder"), "got: {err}");
}

#[test]
fn dirty_flag_clears_on_save() {
    let dir = std::env::temp_dir().join("nodo-mcp-test");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("dirty.minder");
    let path_str = path.to_str().unwrap();

    let mut s = fresh();
    call(&mut s, "add_node", json!({ "parent": 0, "text": "A" }));
    assert_eq!(call(&mut s, "map_stats", json!({}))["dirty"], true);
    call(&mut s, "save_map", json!({ "path": path_str }));
    assert_eq!(call(&mut s, "map_stats", json!({}))["dirty"], false);

    let _ = std::fs::remove_file(&path);
}

#[test]
fn set_side_and_fold_survive_later_edits() {
    // Antes do Editor, set_side mexia numa cópia do mapa e a edição seguinte o apagava.
    let mut s = fresh();
    let a = call(&mut s, "add_node", json!({ "parent": 0, "text": "A" }))["id"]
        .as_u64()
        .unwrap();
    call(&mut s, "add_node", json!({ "parent": a, "text": "A1" }));
    call(&mut s, "set_side", json!({ "id": a, "side": "left" }));
    call(&mut s, "toggle_fold", json!({ "id": a }));
    call(&mut s, "add_node", json!({ "parent": 0, "text": "B" }));

    let node = call(&mut s, "get_node", json!({ "id": a }));
    assert_eq!(node["node"]["side"], "left");
    assert_eq!(node["node"]["folded"], true);

    // E tudo isso desfaz pelo mesmo histórico.
    call(&mut s, "undo", json!({}));
    call(&mut s, "undo", json!({}));
    call(&mut s, "undo", json!({}));
    assert_eq!(
        call(&mut s, "get_node", json!({ "id": a }))["node"]["side"],
        "right"
    );
}

#[test]
fn map_theme_is_set_read_back_and_validated() {
    let mut s = fresh();
    assert_eq!(call(&mut s, "get_map", json!({}))["theme"], "classico");
    call(&mut s, "set_map_theme", json!({ "theme": "sublinhado" }));
    assert_eq!(call(&mut s, "get_map", json!({}))["theme"], "sublinhado");
    let params = json!({ "name": "set_map_theme", "arguments": { "theme": "neon" } });
    assert!(
        tools::call(&mut s, &params).is_err(),
        "tema desconhecido é recusado"
    );
    call(&mut s, "undo", json!({}));
    assert_eq!(call(&mut s, "get_map", json!({}))["theme"], "classico");
}

#[test]
fn move_sibling_reorders_and_undoes_like_the_window() {
    let mut s = fresh();
    let p = call(&mut s, "add_node", json!({ "parent": 0, "text": "P" }))["id"]
        .as_u64()
        .unwrap();
    let a = call(&mut s, "add_node", json!({ "parent": p, "text": "A" }))["id"]
        .as_u64()
        .unwrap();
    let b = call(&mut s, "add_node", json!({ "parent": p, "text": "B" }))["id"]
        .as_u64()
        .unwrap();
    call(
        &mut s,
        "move_sibling",
        json!({ "id": b, "direction": "up" }),
    );
    let ids = |s: &mut Session| -> Vec<u64> {
        call(s, "get_node", json!({ "id": p }))["node"]["children"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|c| c.as_u64())
            .collect()
    };
    assert_eq!(ids(&mut s), [b, a]);
    call(&mut s, "undo", json!({}));
    assert_eq!(ids(&mut s), [a, b]);
}

#[test]
fn set_layout_arranges_the_map_and_undoes() {
    let mut s = fresh();
    let a = call(&mut s, "add_node", json!({ "parent": 0, "text": "A" }))["id"]
        .as_u64()
        .unwrap();
    let r = call(
        &mut s,
        "set_layout",
        json!({ "structure": "organograma", "spacing": "largo" }),
    );
    assert_eq!(r["structure"], "organograma");
    assert_eq!(r["spacing"], "largo");
    let map = call(&mut s, "get_map", json!({}));
    assert_eq!(map["structure"], "organograma");
    let root_y = call(&mut s, "get_node", json!({ "id": 0 }))["node"]["y"].as_f64();
    let a_y = call(&mut s, "get_node", json!({ "id": a }))["node"]["y"].as_f64();
    if let (Some(ry), Some(ay)) = (root_y, a_y) {
        assert!(ay > ry, "no organograma o filho fica abaixo");
    }
    call(&mut s, "undo", json!({}));
    call(&mut s, "undo", json!({}));
    assert_eq!(call(&mut s, "get_map", json!({}))["structure"], "mapa");
    let bad = tools::call(
        &mut s,
        &json!({ "name": "set_layout", "arguments": { "structure": "espiral" } }),
    );
    assert!(bad.is_err() || bad.unwrap()["isError"] == true);
}

#[test]
fn export_map_returns_markdown_and_writes_a_file() {
    let mut s = fresh();
    let a = call(
        &mut s,
        "add_node",
        json!({ "parent": 0, "text": "Roteiro" }),
    )["id"]
        .as_u64()
        .unwrap();
    call(&mut s, "add_node", json!({ "parent": a, "text": "Dia 1" }));
    let md = call(&mut s, "export_map", json!({ "format": "markdown" }));
    assert!(md["text"]
        .as_str()
        .unwrap()
        .contains("- Roteiro\n  - Dia 1\n"));

    let dir = std::env::temp_dir().join(format!("nodo-export-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("mapa.opml");
    call(
        &mut s,
        "export_map",
        json!({ "format": "opml", "path": path.to_string_lossy() }),
    );
    let written = std::fs::read_to_string(&path).unwrap();
    assert!(written.contains("<outline text=\"Dia 1\"/>"));
    let _ = std::fs::remove_dir_all(&dir);
}
