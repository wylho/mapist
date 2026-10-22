//! Nodo desktop application.
//!
//! The Tauri shell owns the open maps, one per tab (`tabs.rs`), and exposes the one
//! in front to the webview through commands. All mind-map semantics live in `nodo-core`; this layer
//! only handles window concerns, file dialogs, and JSON shaping.

use nodo_core::{model::MindMap, model::NodeSide, undo::Action, Editor};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::{Emitter, Manager, State, WebviewWindowBuilder};

mod attachments;
mod tabs;
mod updates;

use tabs::Tabs;

/// Application state shared across command invocations.
#[derive(Default)]
pub struct AppState {
    inner: Mutex<Tabs>,
    /// Set once the person chose to quit without saving, so the exit that
    /// follows is not stopped again.
    quitting: AtomicBool,
}

type Doc = Tabs;

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
    /// Tags, emoji, reminder and status (fields left out when unset).
    card: nodo_core::model::CardFields,
    /// The card preview (Atrio's projection, the same as the Ideario's): blocks, cover, counts, `#tags`.
    #[serde(skip_serializing_if = "Option::is_none")]
    view: Option<nodo_core::preview::CardView>,
    child_ids: Vec<u64>,
    parent: Option<u64>,
    has_children: bool,
}

/// Everything the renderer needs to draw the current map.
#[derive(Serialize)]
pub struct Snapshot {
    nodes: Vec<FlatNode>,
    connections: Vec<Edge>,
    /// Loose notes on the canvas.
    notes: Vec<NoteCard>,
    dirty: bool,
    path: Option<String>,
    title: String,
    can_undo: bool,
    can_redo: bool,
    /// The file changed on disk since it was read or saved here (an agent saved it).
    stale: bool,
    /// The map's theme (`nodo_core::MAP_STYLES`), or `None` for the default.
    style: Option<String>,
    /// How the map is arranged (`nodo_core::STRUCTURES`), or `None` for the classic map.
    structure: Option<String>,
    /// Room between nodes (`nodo_core::SPACINGS`), or `None` for normal.
    spacing: Option<String>,
    bounds: Bounds,
    /// The open maps, in tab order.
    tabs: Vec<TabInfo>,
    /// The tab in front.
    active: Option<u64>,
}

/// What a tab shows: the map's name and whether it has unsaved changes.
#[derive(Serialize)]
pub struct TabInfo {
    id: u64,
    title: String,
    path: Option<String>,
    dirty: bool,
}

fn tab_infos(doc: &Doc) -> Vec<TabInfo> {
    doc.iter()
        .map(|t| TabInfo {
            id: t.id,
            title: t.editor.map().root.text.clone(),
            path: t.editor.path().map(|p| p.display().to_string()),
            dirty: t.editor.is_dirty(),
        })
        .collect()
}

/// A loose note on the canvas, in world units.
#[derive(Serialize)]
pub struct NoteCard {
    id: u64,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    text: String,
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
    let Some(ed) = doc.current() else {
        return Snapshot {
            nodes: Vec::new(),
            connections: Vec::new(),
            notes: Vec::new(),
            dirty: false,
            path: None,
            title: "Sem título".to_string(),
            can_undo: false,
            can_redo: false,
            stale: false,
            style: None,
            structure: None,
            spacing: None,
            bounds: Bounds::default(),
            tabs: tab_infos(doc),
            active: None,
        };
    };
    let map: &MindMap = ed.map();
    let mut nodes = Vec::with_capacity(map.node_count());

    // Pre-order walk carrying the parent id and depth. A folded node's
    // descendants are hidden: they have no position until it unfolds.
    fn walk(
        node: &nodo_core::model::Node,
        parent: Option<u64>,
        depth: u32,
        atts: &nodo_core::attachments::Attachments,
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
            card: node.card.clone(),
            view: nodo_core::preview::card_view(&node.note, atts),
            child_ids: node.children.iter().map(|c| c.id).collect(),
            parent,
            has_children: !node.children.is_empty(),
        });
        if node.folded {
            return;
        }
        for child in &node.children {
            walk(child, Some(node.id), depth + 1, atts, out);
        }
    }
    walk(&map.root, None, 0, ed.attachments(), &mut nodes);

    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    let mut max_x = f64::MIN;
    let mut max_y = f64::MIN;
    let notes: Vec<NoteCard> = map
        .notes
        .iter()
        .map(|n| NoteCard {
            id: n.id,
            x: n.pos.x,
            y: n.pos.y,
            width: n.width,
            height: n.height,
            text: n.text.clone(),
        })
        .collect();
    // As notas soltas entram na extensão: "ver o mapa inteiro" e a exportação mostram elas também.
    let boxes = nodes
        .iter()
        .map(|n| (n.x, n.y, n.width, n.height))
        .chain(notes.iter().map(|n| (n.x, n.y, n.width, n.height)));
    for (x, y, w, h) in boxes {
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x + w);
        max_y = max_y.max(y + h);
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
        notes,
        dirty: ed.is_dirty(),
        path: ed.path().map(|p| p.display().to_string()),
        title: map.root.text.clone(),
        can_undo: ed.can_undo(),
        can_redo: ed.can_redo(),
        stale: ed.changed_on_disk(),
        style: map.theme.style.clone(),
        structure: map.theme.structure.clone(),
        spacing: map.theme.spacing.clone(),
        bounds: Bounds {
            min_x: if min_x == f64::MAX { 0.0 } else { min_x },
            min_y: if min_y == f64::MAX { 0.0 } else { min_y },
            max_x: if max_x == f64::MIN { 0.0 } else { max_x },
            max_y: if max_y == f64::MIN { 0.0 } else { max_y },
        },
        tabs: tab_infos(doc),
        active: doc.active_id(),
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
    doc.current_mut()
        .ok_or_else(|| "nenhum mapa aberto".to_string())
}

/// Return the current map, or an empty snapshot when nothing is open.
#[tauri::command]
fn get_snapshot(state: State<'_, AppState>) -> Result<Snapshot, String> {
    Ok(snapshot(&*lock(&state)?))
}

/// Create a new map in a tab of its own, in front.
#[tauri::command]
fn new_map(state: State<'_, AppState>, title: String) -> Result<Snapshot, String> {
    let mut doc = lock(&state)?;
    doc.add(Editor::new(&title));
    Ok(snapshot(&doc))
}

/// Bring a tab to the front.
#[tauri::command]
fn switch_tab(state: State<'_, AppState>, id: u64) -> Result<Snapshot, String> {
    let mut doc = lock(&state)?;
    if !doc.switch(id) {
        return Err("essa aba não está mais aberta".to_string());
    }
    Ok(snapshot(&doc))
}

/// Close a tab; the interface asks about unsaved changes before.
#[tauri::command]
fn close_tab(state: State<'_, AppState>, id: u64) -> Result<Snapshot, String> {
    let mut doc = lock(&state)?;
    doc.close(id);
    Ok(snapshot(&doc))
}

/// Open a map from disk, or import a Markdown or OPML file as a new map.
#[tauri::command]
fn open_map(state: State<'_, AppState>, path: String) -> Result<Snapshot, String> {
    // Already open: its tab comes to the front instead of a second copy.
    if let Some(id) = lock(&state)?.find_path(std::path::Path::new(&path)) {
        let mut doc = lock(&state)?;
        doc.switch(id);
        return Ok(snapshot(&doc));
    }
    let ed =
        Editor::open_or_import(&path).map_err(|e| format!("não foi possível abrir {path}: {e}"))?;
    let mut doc = lock(&state)?;
    doc.open(ed);
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
    let ed = Editor::open_or_import(&path)
        .map_err(|e| format!("não foi possível ler {}: {e}", path.display()))?;
    doc.replace_current(ed);
    Ok(snapshot(&doc))
}

/// Save the open map, optionally to a new path.
#[tauri::command]
fn save_map(state: State<'_, AppState>, path: Option<String>) -> Result<Snapshot, String> {
    let mut doc = lock(&state)?;
    editor(&mut doc)?
        .save(path.as_deref().map(std::path::Path::new))
        .map_err(|e| match e {
            nodo_core::Error::NoPath => "escolha onde salvar o mapa".to_string(),
            e => format!("não foi possível salvar: {e}"),
        })?;
    Ok(snapshot(&doc))
}

/// The open map as text: markdown, opml or html (`nodo_core::export`), the same the MCP server exports.
#[tauri::command]
fn export_text(state: State<'_, AppState>, format: String) -> Result<String, String> {
    let mut doc = lock(&state)?;
    nodo_core::export::to_text(editor(&mut doc)?.map(), &format).map_err(|e| e.to_string())
}

/// Write an export the interface made (PNG, SVG, PDF, HTML, text) to the path the person chose.
#[tauri::command]
fn write_export(path: String, bytes: Vec<u8>) -> Result<(), String> {
    nodo_core::write_atomic(std::path::Path::new(&path), &bytes)
        .map_err(|e| format!("não foi possível exportar: {e}"))
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

/// Add a child with a note, branch color and card fields already in it (a pasted card), as one undo step.
#[tauri::command]
fn add_topic(
    state: State<'_, AppState>,
    parent: u64,
    text: String,
    note: String,
    color: Option<String>,
    card: Option<nodo_core::model::CardFields>,
) -> Result<Snapshot, String> {
    let card = card.unwrap_or_default();
    mutate(
        state,
        Action::AddTopic {
            parent,
            text,
            note,
            color,
            card,
        },
    )
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

/// Replace a node's card fields (tags, emoji, reminder, status); the core cleans them with the card rules.
#[tauri::command]
fn set_card(
    state: State<'_, AppState>,
    id: u64,
    card: nodo_core::model::CardFields,
) -> Result<Snapshot, String> {
    mutate(state, Action::SetCard { id, card })
}

/// A node's note as the card editor's document (TipTap JSON), read with the Ideario's Markdown rules.
#[tauri::command]
fn note_doc(state: State<'_, AppState>, id: u64) -> Result<serde_json::Value, String> {
    let mut doc = lock(&state)?;
    let node = editor(&mut doc)?
        .map()
        .find(id)
        .ok_or_else(|| format!("nó {id} não encontrado"))?;
    Ok(nodo_core::atrio::markdown::from_markdown(&node.note))
}

/// Markdown pasted in the card, as the editor's document.
#[tauri::command]
fn markdown_doc(markdown: String) -> serde_json::Value {
    nodo_core::atrio::markdown::from_markdown(&markdown)
}

/// Save the card editor's document as the node's Markdown note.
#[tauri::command]
fn set_note_doc(
    state: State<'_, AppState>,
    id: u64,
    doc: serde_json::Value,
) -> Result<Snapshot, String> {
    let mut tabs = lock(&state)?;
    let ed = editor(&mut tabs)?;
    // O nome de cada anexo vai no texto (`![praia.jpg](ideario://att/…)`), para a nota ser legível fora do app.
    let atts = ed.attachments();
    let note =
        nodo_core::atrio::markdown::to_markdown(&doc, &|h| atts.get(h).map(|a| a.name.clone()));
    ed.apply(Action::SetNote { id, note })
        .map_err(|e| e.to_string())?;
    Ok(snapshot(&tabs))
}

/// Put a loose note on the canvas, its top-left corner at `(x, y)`.
#[tauri::command]
fn add_free_note(
    state: State<'_, AppState>,
    x: f64,
    y: f64,
    text: String,
) -> Result<Snapshot, String> {
    mutate(state, Action::AddFreeNote { x, y, text })
}

/// Replace a loose note's text.
#[tauri::command]
fn set_free_note(state: State<'_, AppState>, id: u64, text: String) -> Result<Snapshot, String> {
    mutate(state, Action::SetFreeNote { id, text })
}

/// Move or resize a loose note.
#[tauri::command]
fn place_free_note(
    state: State<'_, AppState>,
    id: u64,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<Snapshot, String> {
    mutate(
        state,
        Action::PlaceFreeNote {
            id,
            x,
            y,
            width,
            height,
        },
    )
}

/// Take a loose note off the canvas.
#[tauri::command]
fn remove_free_note(state: State<'_, AppState>, id: u64) -> Result<Snapshot, String> {
    mutate(state, Action::RemoveFreeNote { id })
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

/// Choose the map's theme; it is saved in the file and undoable.
#[tauri::command]
fn set_map_style(state: State<'_, AppState>, style: Option<String>) -> Result<Snapshot, String> {
    mutate(state, Action::SetMapStyle { style })
}

/// Arrange the map (mapa, logico, organograma, arvore); saved in the file and undoable.
#[tauri::command]
fn set_structure(
    state: State<'_, AppState>,
    structure: Option<String>,
) -> Result<Snapshot, String> {
    mutate(state, Action::SetStructure { structure })
}

/// Room between nodes (compacto, normal, largo); saved in the file and undoable.
#[tauri::command]
fn set_spacing(state: State<'_, AppState>, spacing: Option<String>) -> Result<Snapshot, String> {
    mutate(state, Action::SetSpacing { spacing })
}

/// Put every node back where the structure wants it, undoing hand-placed positions.
#[tauri::command]
fn relayout(state: State<'_, AppState>) -> Result<Snapshot, String> {
    mutate(state, Action::Relayout)
}

/// Fold or unfold a node.
#[tauri::command]
fn toggle_fold(state: State<'_, AppState>, id: u64) -> Result<Snapshot, String> {
    mutate(state, Action::ToggleFold { id })
}

/// Move a node up (`delta < 0`) or down among its siblings.
#[tauri::command]
fn reorder_node(state: State<'_, AppState>, id: u64, delta: i64) -> Result<Snapshot, String> {
    mutate(state, Action::Reorder { id, delta })
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
    let Some(ed) = doc.current() else {
        return Ok(Vec::new());
    };
    // Sem acento e sem maiúsculas, como o Ideario ("acao" acha "Ação").
    let fold = nodo_core::atrio::text::fold;
    let q = fold(&query);
    Ok(ed
        .map()
        .nodes()
        .filter(|n| fold(&n.text).contains(&q) || fold(&n.note).contains(&q))
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
fn mcp_info() -> Result<nodo_mcp::setup::McpInfo, String> {
    nodo_mcp::setup::info()
}

/// Register Nodo with Claude Desktop. Takes effect after Claude restarts.
#[tauri::command]
fn mcp_install() -> Result<nodo_mcp::setup::McpInfo, String> {
    let config =
        nodo_mcp::setup::desktop_config().ok_or("could not locate the user's config directory")?;
    let cmd = nodo_mcp::setup::command()?.display().to_string();
    nodo_mcp::setup::install(&config, &cmd)?;
    nodo_mcp::setup::info()
}

/// Remove Nodo from Claude Desktop's config.
#[tauri::command]
fn mcp_uninstall() -> Result<nodo_mcp::setup::McpInfo, String> {
    let config =
        nodo_mcp::setup::desktop_config().ok_or("could not locate the user's config directory")?;
    nodo_mcp::setup::uninstall(&config)?;
    nodo_mcp::setup::info()
}

/// Tells the page the title bar is transparent (macOS): the app bar makes room
/// for the window buttons in its corner and drags the window, as in Ideario.
#[cfg(target_os = "macos")]
const TITLEBAR_SCRIPT: &str = "document.documentElement.dataset.titlebar = 'overlay';";
#[cfg(not(target_os = "macos"))]
const TITLEBAR_SCRIPT: &str = "";

/// macOS: width of the corner holding the three window buttons, with the same
/// margin on both sides. Measured, because their size and spacing change between
/// macOS versions. Not `async`: it runs on the main thread, as AppKit requires.
#[tauri::command]
fn mac_lights_width(window: tauri::WebviewWindow) -> Option<f64> {
    #[cfg(target_os = "macos")]
    {
        use objc2_app_kit::{NSWindow, NSWindowButton};
        let ptr = window.ns_window().ok()?.cast::<NSWindow>();
        // SAFETY: Tauri hands back this window's live NSWindow; we only read it, on the main thread.
        let ns = unsafe { ptr.as_ref() }?;
        let close = ns
            .standardWindowButton(NSWindowButton::CloseButton)?
            .frame();
        let zoom = ns.standardWindowButton(NSWindowButton::ZoomButton)?.frame();
        let width = zoom.origin.x + zoom.size.width + close.origin.x;
        (width > 0.0 && width < 200.0).then_some(width)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = window;
        None
    }
}

/// Linux: a WebKitGTK vem com a captura de mídia desligada e nega o pedido de microfone e câmera se ninguém responder
/// (o mesmo do Ideario). Liga a captura e aceita só áudio e vídeo, para o gravador e a câmera do card.
#[cfg(target_os = "linux")]
fn linux_media(app: &tauri::App) -> tauri::Result<()> {
    use webkit2gtk::{
        glib::prelude::*, PermissionRequestExt, SettingsExt, UserMediaPermissionRequest,
        UserMediaPermissionRequestExt, WebViewExt,
    };
    let Some(window) = app.get_webview_window("main") else {
        return Ok(());
    };
    window.with_webview(|wv| {
        let view = wv.inner();
        if let Some(settings) = view.settings() {
            settings.set_enable_media_stream(true);
            settings.set_enable_mediasource(true);
        }
        view.connect_permission_request(|_, request| {
            let Some(media) = request.downcast_ref::<UserMediaPermissionRequest>() else {
                return false;
            };
            if media.is_for_audio_device() || media.is_for_video_device() {
                request.allow();
            } else {
                request.deny();
            }
            true
        });
    })
}

/// Whether closing now would lose unsaved work.
fn has_unsaved(app: &tauri::AppHandle) -> bool {
    let state = app.state::<AppState>();
    if state.quitting.load(Ordering::SeqCst) {
        return false;
    }
    let dirty = state.inner.lock().map(|d| d.any_dirty()).unwrap_or(false);
    dirty
}

/// A map path among the program arguments (double-click on Windows and Linux, or `nodo mapa.nodo` in a terminal):
/// a `.nodo`, or a `.minder` from the early versions, which opens as an import.
fn file_from_args() -> Option<std::path::PathBuf> {
    std::env::args_os()
        .skip(1)
        .map(std::path::PathBuf::from)
        .find(|p| {
            p.extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("nodo") || e.eq_ignore_ascii_case("minder"))
        })
}

/// Run the application.
pub fn run() {
    // Opened with a file: it is the document from the first frame.
    let state = AppState::default();
    if let Some(path) = file_from_args() {
        if let (Ok(ed), Ok(mut doc)) = (Editor::open_or_import(&path), state.inner.lock()) {
            doc.add(ed);
        }
    }

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
        .manage(state)
        .manage(attachments::Served::default())
        // Fotos, áudios, vídeos e arquivos das notas: `att://localhost/<hash>` (convertFileSrc(hash, 'att')).
        .register_asynchronous_uri_scheme_protocol("att", |ctx, req, responder| {
            let app = ctx.app_handle().clone();
            std::thread::spawn(move || responder.respond(attachments::serve(&app, &req)));
        })
        // The window is built here rather than from the config, so that macOS
        // gets the transparent title bar with the buttons inside the app bar.
        .setup(|app| {
            let config = app
                .config()
                .app
                .windows
                .iter()
                .find(|w| w.label == "main")
                .cloned()
                .ok_or("janela main ausente no tauri.conf.json")?;
            // macOS: barra de título transparente com os três botões dentro da barra do app, como no Ideario.
            // Estilo, título escondido e posição dos botões vêm do tauri.conf.json (titleBarStyle, hiddenTitle,
            // trafficLightPosition), não do construtor: pela configuração a posição fica guardada na janela (tao),
            // que a reaplica quando o título muda. Pelo construtor ela ia só para a página (wry), e cada troca de
            // título (o nome do arquivo, o "•" de não salvo) devolvia os botões ao canto. Nos outros sistemas
            // essas chaves não têm efeito.
            let window = WebviewWindowBuilder::from_config(app.handle(), &config)?
                .initialization_script(TITLEBAR_SCRIPT);
            window.build()?;
            #[cfg(target_os = "linux")]
            linux_media(app)?;
            updates::init(app.handle())?;
            Ok(())
        })
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
            switch_tab,
            close_tab,
            open_map,
            attachments::import_file,
            attachments::import_path,
            attachments::get_attachments,
            attachments::save_attachment,
            attachments::open_attachment,
            reload_map,
            save_map,
            add_node,
            add_topic,
            remove_node,
            set_text,
            set_note,
            set_card,
            note_doc,
            markdown_doc,
            set_note_doc,
            add_free_note,
            set_free_note,
            place_free_note,
            remove_free_note,
            set_color,
            set_side,
            set_map_style,
            toggle_fold,
            reorder_node,
            set_structure,
            set_spacing,
            relayout,
            export_text,
            write_export,
            move_node,
            undo,
            redo,
            search_nodes,
            set_sizes,
            quit_app,
            mac_lights_width,
            mcp_info,
            mcp_install,
            mcp_uninstall,
            updates::update_supported,
            updates::update_check,
            updates::update_install
        ])
        .build(tauri::generate_context!())
        .expect("error while building Nodo")
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
        let mut doc = Tabs::default();
        doc.add(ed);
        doc
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
        let snap = snapshot(&Tabs::default());
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
        doc.current_mut()
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
