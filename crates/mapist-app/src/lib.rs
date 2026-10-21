//! Mapist desktop application.
//!
//! The Tauri shell owns a single open document and exposes it to the webview
//! through commands. All mind-map semantics live in `mapist-core`; this layer
//! only handles window concerns, file dialogs, and JSON shaping.

use mapist_core::{layout, model::MindMap, undo};
use serde::Serialize;
use std::sync::Mutex;
use tauri::State;

/// Application state shared across command invocations.
pub struct AppState {
    inner: Mutex<Doc>,
}

struct Doc {
    map: Option<MindMap>,
    path: Option<String>,
    dirty: bool,
    history: Option<undo::History>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            inner: Mutex::new(Doc {
                map: None,
                path: None,
                dirty: false,
                history: None,
            }),
        }
    }
}

/// A node flattened for the renderer, with absolute geometry.
#[derive(Serialize)]
pub struct FlatNode {
    id: u64,
    text: String,
    note: String,
    side: String,
    depth: u32,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    color: Option<String>,
    folded: bool,
    child_ids: Vec<u64>,
    parent: Option<u64>,
    has_children: bool,
}

/// Everything the renderer needs to draw the current map.
#[derive(Serialize)]
pub struct Snapshot {
    nodes: Vec<FlatNode>,
    connections: Vec<Edge>,
    dirty: bool,
    path: Option<String>,
    title: String,
    can_undo: bool,
    can_redo: bool,
    bounds: Bounds,
}

/// A drawn connection between two nodes.
#[derive(Serialize)]
pub struct Edge {
    from: u64,
    to: u64,
    text: String,
    color: Option<String>,
}

/// Axis-aligned extent of the laid-out map.
#[derive(Serialize)]
pub struct Bounds {
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
}

impl Default for Bounds {
    fn default() -> Self {
        Self {
            min_x: 0.0,
            min_y: 0.0,
            max_x: 0.0,
            max_y: 0.0,
        }
    }
}

/// Flatten a laid-out map into renderer-friendly rows.
fn snapshot(doc: &Doc) -> Snapshot {
    let Some(map) = doc.map.as_ref() else {
        return Snapshot {
            nodes: Vec::new(),
            connections: Vec::new(),
            dirty: doc.dirty,
            path: doc.path.clone(),
            title: "Untitled".to_string(),
            can_undo: false,
            can_redo: false,
            bounds: Bounds::default(),
        };
    };

    let mut nodes = Vec::with_capacity(map.node_count());
    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    let mut max_x = f64::MIN;
    let mut max_y = f64::MIN;

    // Pre-order walk carrying the parent id and depth.
    fn walk(
        node: &mapist_core::model::Node,
        parent: Option<u64>,
        depth: u32,
        map: &MindMap,
        out: &mut Vec<FlatNode>,
    ) {
        out.push(FlatNode {
            id: node.id,
            text: node.text.clone(),
            note: node.note.clone(),
            side: node.side.as_str().to_string(),
            depth,
            x: node.pos.x,
            y: node.pos.y,
            width: node.width,
            height: node.height,
            color: node.color.clone(),
            folded: node.folded,
            child_ids: node.children.iter().map(|c| c.id).collect(),
            parent,
            has_children: !node.children.is_empty(),
        });
        let _ = map;
        for child in &node.children {
            walk(child, Some(node.id), depth + 1, map, out);
        }
    }
    walk(&map.root, None, 0, map, &mut nodes);

    for n in &nodes {
        min_x = min_x.min(n.x);
        min_y = min_y.min(n.y);
        max_x = max_x.max(n.x + n.width);
        max_y = max_y.max(n.y + n.height);
    }

    let connections = map
        .connections
        .iter()
        .map(|c| Edge {
            from: c.from,
            to: c.to,
            text: c.text.clone(),
            color: None,
        })
        .collect();

    Snapshot {
        nodes,
        connections,
        dirty: doc.dirty,
        path: doc.path.clone(),
        title: map.root.text.clone(),
        can_undo: doc.history.as_ref().map(|h| h.can_undo()).unwrap_or(false),
        can_redo: doc.history.as_ref().map(|h| h.can_redo()).unwrap_or(false),
        bounds: Bounds {
            min_x: if min_x == f64::MAX { 0.0 } else { min_x },
            min_y: if min_y == f64::MAX { 0.0 } else { min_y },
            max_x: if max_x == f64::MIN { 0.0 } else { max_x },
            max_y: if max_y == f64::MIN { 0.0 } else { max_y },
        },
    }
}

/// Return the current map, or an empty snapshot when nothing is open.
#[tauri::command]
fn get_snapshot(state: State<'_, AppState>) -> Result<Snapshot, String> {
    let doc = state.inner.lock().map_err(|_| "state poisoned")?;
    Ok(snapshot(&doc))
}

/// Create a new map, replacing any open document.
#[tauri::command]
fn new_map(state: State<'_, AppState>, title: String) -> Result<Snapshot, String> {
    let mut doc = state.inner.lock().map_err(|_| "state poisoned")?;
    let mut map = MindMap::new(if title.trim().is_empty() {
        "Central Topic"
    } else {
        title.trim()
    });
    layout::layout(&mut map);
    doc.history = Some(undo::History::new(map.clone()));
    doc.map = Some(map);
    doc.path = None;
    doc.dirty = false;
    Ok(snapshot(&doc))
}

/// Open a map from disk.
#[tauri::command]
fn open_map(state: State<'_, AppState>, path: String) -> Result<Snapshot, String> {
    let mut map = mapist_core::load(&path).map_err(|e| format!("could not open {path}: {e}"))?;
    layout::layout(&mut map);
    let mut doc = state.inner.lock().map_err(|_| "state poisoned")?;
    doc.history = Some(undo::History::new(map.clone()));
    doc.map = Some(map);
    doc.path = Some(path);
    doc.dirty = false;
    Ok(snapshot(&doc))
}

/// Save the open map, optionally to a new path.
#[tauri::command]
fn save_map(state: State<'_, AppState>, path: Option<String>) -> Result<Snapshot, String> {
    let mut doc = state.inner.lock().map_err(|_| "state poisoned")?;
    let target = path
        .or_else(|| doc.path.clone())
        .ok_or("no path to save to")?;
    let map = doc.map.as_ref().ok_or("no document is open")?;
    mapist_core::save(map, &target).map_err(|e| format!("could not save {target}: {e}"))?;
    doc.path = Some(target);
    doc.dirty = false;
    Ok(snapshot(&doc))
}

/// Add a child under an existing node and relayout.
#[tauri::command]
fn add_node(state: State<'_, AppState>, parent: u64, text: String) -> Result<Snapshot, String> {
    mutate(state, undo::Action::AddChild { parent, text })
}

/// Remove a node and its subtree.
#[tauri::command]
fn remove_node(state: State<'_, AppState>, id: u64) -> Result<Snapshot, String> {
    mutate(state, undo::Action::RemoveNode { id })
}

/// Replace a node's text.
#[tauri::command]
fn set_text(state: State<'_, AppState>, id: u64, text: String) -> Result<Snapshot, String> {
    mutate(state, undo::Action::SetText { id, text })
}

/// Replace a node's note.
#[tauri::command]
fn set_note(state: State<'_, AppState>, id: u64, note: String) -> Result<Snapshot, String> {
    mutate(state, undo::Action::SetNote { id, note })
}

/// Set or clear a node's branch colour.
#[tauri::command]
fn set_color(
    state: State<'_, AppState>,
    id: u64,
    color: Option<String>,
) -> Result<Snapshot, String> {
    mutate(state, undo::Action::SetColor { id, color })
}

/// Toggle a node's folded state and relayout.
#[tauri::command]
fn toggle_fold(state: State<'_, AppState>, id: u64) -> Result<Snapshot, String> {
    let mut doc = state.inner.lock().map_err(|_| "state poisoned")?;
    let before = doc.map.clone().ok_or("no document is open")?;
    {
        let map = doc.map.as_mut().ok_or("no document is open")?;
        let node = map
            .find_mut(id)
            .ok_or_else(|| format!("node {id} not found"))?;
        node.folded = !node.folded;
        layout::layout(map);
    }
    if let Some(h) = doc.history.as_mut() {
        h.commit(before);
    }
    doc.dirty = true;
    Ok(snapshot(&doc))
}

/// Undo the last change.
#[tauri::command]
fn undo(state: State<'_, AppState>) -> Result<Snapshot, String> {
    let mut doc = state.inner.lock().map_err(|_| "state poisoned")?;
    if let Some(h) = doc.history.as_mut() {
        if h.undo() {
            doc.map = Some(h.current().clone());
            doc.dirty = true;
        }
    }
    Ok(snapshot(&doc))
}

/// Redo the last undone change.
#[tauri::command]
fn redo(state: State<'_, AppState>) -> Result<Snapshot, String> {
    let mut doc = state.inner.lock().map_err(|_| "state poisoned")?;
    if let Some(h) = doc.history.as_mut() {
        if h.redo() {
            doc.map = Some(h.current().clone());
            doc.dirty = true;
        }
    }
    Ok(snapshot(&doc))
}

/// Find nodes whose text or note contains a substring.
#[tauri::command]
fn search_nodes(
    state: State<'_, AppState>,
    query: String,
    limit: usize,
) -> Result<Vec<u64>, String> {
    let doc = state.inner.lock().map_err(|_| "state poisoned")?;
    let Some(map) = doc.map.as_ref() else {
        return Ok(Vec::new());
    };
    let q = query.to_lowercase();
    Ok(map
        .nodes()
        .filter(|n| n.text.to_lowercase().contains(&q) || n.note.to_lowercase().contains(&q))
        .take(if limit == 0 { 50 } else { limit })
        .map(|n| n.id)
        .collect())
}

/// Apply a mutating action through history, then relayout.
fn mutate(state: State<'_, AppState>, action: undo::Action) -> Result<Snapshot, String> {
    let mut doc = state.inner.lock().map_err(|_| "state poisoned")?;
    {
        let hist = doc.history.as_mut().ok_or("no document is open")?;
        hist.apply(action).map_err(|e| e.to_string())?;
        let mut updated = hist.current().clone();
        layout::layout(&mut updated);
        doc.map = Some(updated);
    }
    doc.dirty = true;
    Ok(snapshot(&doc))
}

/// Run the application.
pub fn run() {
    tauri::Builder::default()
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            get_snapshot,
            new_map,
            open_map,
            save_map,
            add_node,
            remove_node,
            set_text,
            set_note,
            set_color,
            toggle_fold,
            undo,
            redo,
            search_nodes
        ])
        .run(tauri::generate_context!())
        .expect("error while running Mapist");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc_with_map() -> Doc {
        let mut map = MindMap::new("Root");
        let a = map.add_child(0, "A").unwrap();
        map.add_child(a, "A1").unwrap();
        layout::layout(&mut map);
        Doc {
            history: Some(undo::History::new(map.clone())),
            map: Some(map),
            path: None,
            dirty: false,
        }
    }

    #[test]
    fn snapshot_flattens_in_preorder() {
        let doc = doc_with_map();
        let snap = snapshot(&doc);
        assert_eq!(snap.nodes.len(), 3);
        assert_eq!(snap.nodes[0].text, "Root");
        assert_eq!(snap.nodes[0].depth, 0);
        assert_eq!(snap.nodes[1].text, "A");
        assert_eq!(snap.nodes[1].depth, 1);
        assert_eq!(snap.nodes[2].text, "A1");
        assert_eq!(snap.nodes[2].depth, 2);
    }

    #[test]
    fn snapshot_records_parent_links() {
        let doc = doc_with_map();
        let snap = snapshot(&doc);
        assert_eq!(snap.nodes[0].parent, None);
        assert_eq!(snap.nodes[1].parent, Some(0));
        assert_eq!(snap.nodes[2].parent, Some(1));
    }

    #[test]
    fn snapshot_computes_bounds() {
        let doc = doc_with_map();
        let snap = snapshot(&doc);
        assert!(snap.bounds.max_x > snap.bounds.min_x);
        assert!(snap.bounds.max_y > snap.bounds.min_y);
    }

    #[test]
    fn empty_snapshot_is_safe() {
        let doc = Doc {
            map: None,
            path: None,
            dirty: false,
            history: None,
        };
        let snap = snapshot(&doc);
        assert!(snap.nodes.is_empty());
        assert_eq!(snap.title, "Untitled");
        assert_eq!(snap.bounds.min_x, 0.0);
    }

    #[test]
    fn child_ids_are_listed() {
        let doc = doc_with_map();
        let snap = snapshot(&doc);
        assert_eq!(snap.nodes[0].child_ids, vec![1]);
        assert_eq!(snap.nodes[1].child_ids, vec![2]);
        assert!(snap.nodes[2].child_ids.is_empty());
    }
}
