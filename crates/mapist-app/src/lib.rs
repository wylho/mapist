//! Mapist desktop application.
//!
//! The Tauri shell owns a single open document and exposes it to the webview
//! through commands. All mind-map semantics live in `mapist-core`; this layer
//! only handles window concerns, file dialogs, and JSON shaping.

use mapist_core::{model::MindMap, model::NodeSide, undo::Action, Editor};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::{Emitter, Manager, State};

/// Application state shared across command invocations.
#[derive(Default)]
pub struct AppState {
    inner: Mutex<Option<Editor>>,
    /// Set once the person chose to quit without saving, so the exit that
    /// follows is not stopped again.
    quitting: AtomicBool,
}

type Doc = Option<Editor>;

/// A node's measured box, reported by the renderer.
#[derive(serde::Deserialize)]
pub struct NodeSize {
    id: u64,
    width: f64,
    height: f64,
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
    /// The file changed on disk since it was read or saved here (an agent saved it).
    stale: bool,
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
    let Some(ed) = doc.as_ref() else {
        return Snapshot {
            nodes: Vec::new(),
            connections: Vec::new(),
            dirty: false,
            path: None,
            title: "Sem título".to_string(),
            can_undo: false,
            can_redo: false,
            stale: false,
            bounds: Bounds::default(),
        };
    };
    let map: &MindMap = ed.map();
    let mut nodes = Vec::with_capacity(map.node_count());

    // Pre-order walk carrying the parent id and depth. A folded node's
    // descendants are hidden: they have no position until it unfolds.
    fn walk(
        node: &mapist_core::model::Node,
        parent: Option<u64>,
        depth: u32,
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
        if node.folded {
            return;
        }
        for child in &node.children {
            walk(child, Some(node.id), depth + 1, out);
        }
    }
    walk(&map.root, None, 0, &mut nodes);

    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    let mut max_x = f64::MIN;
    let mut max_y = f64::MIN;
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
        dirty: ed.is_dirty(),
        path: ed.path().map(|p| p.display().to_string()),
        title: map.root.text.clone(),
        can_undo: ed.can_undo(),
        can_redo: ed.can_redo(),
        stale: ed.changed_on_disk(),
        bounds: Bounds {
            min_x: if min_x == f64::MAX { 0.0 } else { min_x },
            min_y: if min_y == f64::MAX { 0.0 } else { min_y },
            max_x: if max_x == f64::MIN { 0.0 } else { max_x },
            max_y: if max_y == f64::MIN { 0.0 } else { max_y },
        },
    }
}

/// Lock the document. A poisoned lock means an earlier command panicked
/// mid-edit; the map may be inconsistent, so refuse rather than guess.
fn lock(state: &AppState) -> Result<std::sync::MutexGuard<'_, Doc>, String> {
    state
        .inner
        .lock()
        .map_err(|_| "estado interno corrompido; reabra o mapa".to_string())
}

fn editor(doc: &mut Doc) -> Result<&mut Editor, String> {
    doc.as_mut().ok_or_else(|| "nenhum mapa aberto".to_string())
}

/// Return the current map, or an empty snapshot when nothing is open.
#[tauri::command]
fn get_snapshot(state: State<'_, AppState>) -> Result<Snapshot, String> {
    Ok(snapshot(&*lock(&state)?))
}

/// Create a new map, replacing any open document.
#[tauri::command]
fn new_map(state: State<'_, AppState>, title: String) -> Result<Snapshot, String> {
    let mut doc = lock(&state)?;
    *doc = Some(Editor::new(&title));
    Ok(snapshot(&doc))
}

/// Open a map from disk.
#[tauri::command]
fn open_map(state: State<'_, AppState>, path: String) -> Result<Snapshot, String> {
    let ed = Editor::open(&path).map_err(|e| format!("não foi possível abrir {path}: {e}"))?;
    let mut doc = lock(&state)?;
    *doc = Some(ed);
    Ok(snapshot(&doc))
}

/// Read the open file again, for when an agent saved it through MCP.
#[tauri::command]
fn reload_map(state: State<'_, AppState>) -> Result<Snapshot, String> {
    let mut doc = lock(&state)?;
    let path = editor(&mut doc)?
        .path()
        .ok_or("o mapa ainda não foi salvo")?
        .to_path_buf();
    let ed =
        Editor::open(&path).map_err(|e| format!("não foi possível ler {}: {e}", path.display()))?;
    *doc = Some(ed);
    Ok(snapshot(&doc))
}

/// Save the open map, optionally to a new path.
#[tauri::command]
fn save_map(state: State<'_, AppState>, path: Option<String>) -> Result<Snapshot, String> {
    let mut doc = lock(&state)?;
    editor(&mut doc)?
        .save(path.as_deref().map(std::path::Path::new))
        .map_err(|e| match e {
            mapist_core::Error::NoPath => "escolha onde salvar o mapa".to_string(),
            e => format!("não foi possível salvar: {e}"),
        })?;
    Ok(snapshot(&doc))
}

/// Apply an edit through the editor (undo history and positioning).
fn mutate(state: State<'_, AppState>, action: Action) -> Result<Snapshot, String> {
    let mut doc = lock(&state)?;
    editor(&mut doc)?.apply(action).map_err(|e| e.to_string())?;
    Ok(snapshot(&doc))
}

/// Add a child under an existing node.
#[tauri::command]
fn add_node(state: State<'_, AppState>, parent: u64, text: String) -> Result<Snapshot, String> {
    mutate(state, Action::AddChild { parent, text })
}

/// Remove a node and its subtree.
#[tauri::command]
fn remove_node(state: State<'_, AppState>, id: u64) -> Result<Snapshot, String> {
    mutate(state, Action::RemoveNode { id })
}

/// Replace a node's text.
#[tauri::command]
fn set_text(state: State<'_, AppState>, id: u64, text: String) -> Result<Snapshot, String> {
    mutate(state, Action::SetText { id, text })
}

/// Replace a node's note.
#[tauri::command]
fn set_note(state: State<'_, AppState>, id: u64, note: String) -> Result<Snapshot, String> {
    mutate(state, Action::SetNote { id, note })
}

/// Set or clear a node's branch colour.
#[tauri::command]
fn set_color(
    state: State<'_, AppState>,
    id: u64,
    color: Option<String>,
) -> Result<Snapshot, String> {
    mutate(state, Action::SetColor { id, color })
}

/// Place a branch left or right of the root.
#[tauri::command]
fn set_side(state: State<'_, AppState>, id: u64, side: String) -> Result<Snapshot, String> {
    let side = match side.as_str() {
        "left" => NodeSide::Left,
        "right" => NodeSide::Right,
        other => return Err(format!("lado inválido: {other}")),
    };
    mutate(state, Action::SetSide { id, side })
}

/// Fold or unfold a node.
#[tauri::command]
fn toggle_fold(state: State<'_, AppState>, id: u64) -> Result<Snapshot, String> {
    mutate(state, Action::ToggleFold { id })
}

/// Move a node to an absolute position. It stays there until the next edit
/// that changes the tree's shape.
#[tauri::command]
fn move_node(state: State<'_, AppState>, id: u64, x: f64, y: f64) -> Result<Snapshot, String> {
    mutate(state, Action::MoveNode { id, x, y })
}

/// Undo the last change.
#[tauri::command]
fn undo(state: State<'_, AppState>) -> Result<Snapshot, String> {
    let mut doc = lock(&state)?;
    editor(&mut doc)?.undo();
    Ok(snapshot(&doc))
}

/// Redo the last undone change.
#[tauri::command]
fn redo(state: State<'_, AppState>) -> Result<Snapshot, String> {
    let mut doc = lock(&state)?;
    editor(&mut doc)?.redo();
    Ok(snapshot(&doc))
}

/// Find nodes whose text or note contains a substring.
#[tauri::command]
fn search_nodes(
    state: State<'_, AppState>,
    query: String,
    limit: usize,
) -> Result<Vec<u64>, String> {
    let doc = lock(&state)?;
    let Some(ed) = doc.as_ref() else {
        return Ok(Vec::new());
    };
    let q = query.to_lowercase();
    Ok(ed
        .map()
        .nodes()
        .filter(|n| n.text.to_lowercase().contains(&q) || n.note.to_lowercase().contains(&q))
        .take(if limit == 0 { 50 } else { limit })
        .map(|n| n.id)
        .collect())
}

/// Apply node dimensions measured by the frontend, then reposition.
///
/// The webview measures text with real font metrics, which the core cannot do
/// without duplicating the font stack. So the renderer measures and reports
/// back, and positioning stays in the core.
#[tauri::command]
fn set_sizes(state: State<'_, AppState>, sizes: Vec<NodeSize>) -> Result<Snapshot, String> {
    let mut doc = lock(&state)?;
    editor(&mut doc)?.set_sizes(sizes.iter().map(|s| (s.id, s.width, s.height)));
    Ok(snapshot(&doc))
}

/// Close without saving, after the person confirmed it in the window.
#[tauri::command]
fn quit_app(app: tauri::AppHandle, state: State<'_, AppState>) {
    state.quitting.store(true, Ordering::SeqCst);
    app.exit(0);
}

/// Describe the MCP registration state, for the settings page.
#[tauri::command]
fn mcp_info() -> Result<mapist_mcp::setup::McpInfo, String> {
    mapist_mcp::setup::info()
}

/// Register Mapist with Claude Desktop. Takes effect after Claude restarts.
#[tauri::command]
fn mcp_install() -> Result<mapist_mcp::setup::McpInfo, String> {
    let config = mapist_mcp::setup::desktop_config()
        .ok_or("could not locate the user's config directory")?;
    let cmd = mapist_mcp::setup::command()?.display().to_string();
    mapist_mcp::setup::install(&config, &cmd)?;
    mapist_mcp::setup::info()
}

/// Remove Mapist from Claude Desktop's config.
#[tauri::command]
fn mcp_uninstall() -> Result<mapist_mcp::setup::McpInfo, String> {
    let config = mapist_mcp::setup::desktop_config()
        .ok_or("could not locate the user's config directory")?;
    mapist_mcp::setup::uninstall(&config)?;
    mapist_mcp::setup::info()
}

/// Whether closing now would lose unsaved work.
fn has_unsaved(app: &tauri::AppHandle) -> bool {
    let state = app.state::<AppState>();
    if state.quitting.load(Ordering::SeqCst) {
        return false;
    }
    let dirty = state
        .inner
        .lock()
        .map(|d| d.as_ref().is_some_and(Editor::is_dirty))
        .unwrap_or(false);
    dirty
}

/// Run the application.
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        // Opens links in the user's browser rather than inside the webview.
        .plugin(tauri_plugin_opener::init())
        // Reopens the window at the size and place it was left. Only geometry:
        // appearance belongs to the app's own theme setting.
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_state_flags(
                    tauri_plugin_window_state::StateFlags::SIZE
                        | tauri_plugin_window_state::StateFlags::POSITION
                        | tauri_plugin_window_state::StateFlags::MAXIMIZED,
                )
                .build(),
        )
        .manage(AppState::default())
        // Unsaved work: the window asks, with the app's own confirmation dialog
        // (a native dialog here would block the main thread that runs it).
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if has_unsaved(window.app_handle()) {
                    api.prevent_close();
                    let _ = window.emit("close-requested", ());
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_snapshot,
            new_map,
            open_map,
            reload_map,
            save_map,
            add_node,
            remove_node,
            set_text,
            set_note,
            set_color,
            set_side,
            toggle_fold,
            move_node,
            undo,
            redo,
            search_nodes,
            set_sizes,
            quit_app,
            mcp_info,
            mcp_install,
            mcp_uninstall
        ])
        .build(tauri::generate_context!())
        .expect("error while building Mapist")
        .run(|app, event| {
            // Cmd+Q on macOS quits the app without closing the window first.
            if let tauri::RunEvent::ExitRequested {
                api, code: None, ..
            } = event
            {
                if has_unsaved(app) {
                    api.prevent_exit();
                    let _ = app.emit("close-requested", ());
                }
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc_with_map() -> Doc {
        let mut ed = Editor::new("Root");
        let a = ed
            .apply(Action::AddChild {
                parent: 0,
                text: "A".into(),
            })
            .unwrap()
            .unwrap();
        ed.apply(Action::AddChild {
            parent: a,
            text: "A1".into(),
        })
        .unwrap();
        Some(ed)
    }

    #[test]
    fn snapshot_flattens_in_preorder() {
        let snap = snapshot(&doc_with_map());
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
        let snap = snapshot(&doc_with_map());
        assert_eq!(snap.nodes[0].parent, None);
        assert_eq!(snap.nodes[1].parent, Some(0));
        assert_eq!(snap.nodes[2].parent, Some(1));
    }

    #[test]
    fn snapshot_computes_bounds() {
        let snap = snapshot(&doc_with_map());
        assert!(snap.bounds.max_x > snap.bounds.min_x);
        assert!(snap.bounds.max_y > snap.bounds.min_y);
    }

    #[test]
    fn empty_snapshot_is_safe() {
        let snap = snapshot(&None);
        assert!(snap.nodes.is_empty());
        assert_eq!(snap.title, "Sem título");
        assert_eq!(snap.bounds.min_x, 0.0);
    }

    #[test]
    fn child_ids_are_listed() {
        let snap = snapshot(&doc_with_map());
        assert_eq!(snap.nodes[0].child_ids, vec![1]);
        assert_eq!(snap.nodes[1].child_ids, vec![2]);
        assert!(snap.nodes[2].child_ids.is_empty());
    }

    #[test]
    fn folded_children_are_not_drawn() {
        let mut doc = doc_with_map();
        doc.as_mut()
            .unwrap()
            .apply(Action::ToggleFold { id: 1 })
            .unwrap();
        let snap = snapshot(&doc);
        assert_eq!(
            snap.nodes.len(),
            2,
            "o filho de um nó recolhido não aparece"
        );
        assert!(snap.nodes[1].folded && snap.nodes[1].has_children);
    }
}
