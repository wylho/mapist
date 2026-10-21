//! `nodo-core` — mind-map model, layout, and document I/O.
//!
//! This crate is deliberately UI-free so that both the desktop application and
//! the MCP server embed exactly the same semantics. Anything that needs a
//! window, a font, or a canvas belongs in the application crate, not here.
//!
//! ```no_run
//! use nodo_core::{MindMap, layout, document};
//!
//! let mut map = MindMap::new("Root");
//! map.add_child(0, "Child");
//! layout::layout(&mut map);
//! let xml = document::to_xml(&map).unwrap();
//! let back = document::from_xml(&xml).unwrap();
//! assert_eq!(map.node_count(), back.node_count());
//! ```

pub mod document;
pub mod editor;
pub mod export;
pub mod import;
pub mod layout;
pub mod model;
pub mod nodofile;
pub mod undo;

/// The library shared with the Ideario: card rules (`atrio::text`) and note body ↔ Markdown (`atrio::markdown`).
pub use atrio;
pub use editor::Editor;
pub use model::{
    ArrowStyle, BorderStyle, Connection, DashStyle, DrawArea, LayoutKind, MindMap, Node, NodeSide,
    Point, Style, ThemeRef,
};

/// Format version written by [`document::to_xml`].
///
/// Matches the Minder 2.0.9 release whose XML shape this crate reads.
pub const FORMAT_VERSION: &str = "1.15.0";

/// Map themes the app knows how to draw. The look itself lives in the
/// renderer; the core only checks names, so a file never carries a theme
/// that no door can show.
pub const MAP_STYLES: [&str; 6] = [
    "classico",
    "contorno",
    "sublinhado",
    "vibrante",
    "pilulas",
    "mono",
];

/// Ways to arrange a map: both sides of the root (Minder's default), all to the right, an org chart growing down,
/// and an indented tree. Kept in `nodo-structure`; each node also gets the closest Minder layout, so Minder shows a
/// similar shape.
pub const STRUCTURES: [&str; 4] = ["mapa", "logico", "organograma", "arvore"];

/// Room between nodes, as a factor of the default gaps.
pub const SPACINGS: [&str; 3] = ["compacto", "normal", "largo"];

/// Errors produced by this crate.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("XML error: {0}")]
    Xml(String),

    #[error("document has no <minder> root element")]
    MissingRoot,

    #[error("node {0} not found")]
    NodeNotFound(u64),

    #[error("note {0} not found")]
    NoteNotFound(u64),

    #[error("cannot remove the root node")]
    CannotRemoveRoot,

    #[error("there is no sibling in that direction to swap with")]
    NoSiblingThere,

    #[error("unknown map theme {0:?}; use one of: classico, contorno, sublinhado, vibrante, pilulas, mono")]
    UnknownStyle(String),

    #[error("unknown map structure {0:?}; use one of: mapa, logico, organograma, arvore")]
    UnknownStructure(String),

    #[error("unknown export format {0:?}; use markdown, opml or html")]
    UnknownFormat(String),

    #[error("unknown spacing {0:?}; use one of: compacto, normal, largo")]
    UnknownSpacing(String),

    #[error("the map has never been saved; choose where to save it")]
    NoPath,

    #[error("not a valid map file: {0}")]
    Format(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result alias for this crate.
pub type Result<T> = std::result::Result<T, Error>;

/// Extension of Nodo's own map files (D27).
pub const EXTENSION: &str = "nodo";

/// Read a `.nodo` map from a file. (A `.minder` from the early versions comes in through [`Editor::open_or_import`].)
pub fn load(path: impl AsRef<std::path::Path>) -> Result<MindMap> {
    let text = std::fs::read_to_string(path)?;
    nodofile::from_nodo(&text)
}

/// Write a map as `.nodo`, atomically: the text goes to a temporary file beside the target and is renamed over it,
/// so a crash mid-save never leaves half a map.
pub fn save(map: &MindMap, path: impl AsRef<std::path::Path>) -> Result<()> {
    write_atomic(path.as_ref(), nodofile::to_nodo(map)?.as_bytes())
}

/// Write a file through a temporary sibling and a rename, so a crash never leaves it half written. Used for the map
/// and for exports.
pub fn write_atomic(path: &std::path::Path, bytes: &[u8]) -> Result<()> {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let tmp = path.with_file_name(format!(".{name}.nodo-tmp"));
    std::fs::write(&tmp, bytes)?;
    if let Err(e) = std::fs::rename(&tmp, path) {
        let _ = std::fs::remove_file(&tmp);
        return Err(e.into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> MindMap {
        let mut map = MindMap::new("Root");
        let a = map.add_child(0, "Alpha").unwrap();
        map.add_child(0, "Beta").unwrap();
        map.add_child(a, "Alpha-1").unwrap();
        map
    }

    #[test]
    fn counts_nodes() {
        assert_eq!(sample().node_count(), 4);
    }

    #[test]
    fn assigns_distinct_ids() {
        let map = sample();
        let mut ids: Vec<u64> = map.nodes().map(|n| n.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), 4);
    }

    #[test]
    fn round_trips_through_xml() {
        let map = sample();
        let xml = document::to_xml(&map).unwrap();
        let back = document::from_xml(&xml).unwrap();
        assert_eq!(map.node_count(), back.node_count());
        assert_eq!(map.root.text, back.root.text);
        assert_eq!(
            map.nodes().map(|n| n.text.clone()).collect::<Vec<_>>(),
            back.nodes().map(|n| n.text.clone()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn round_trips_attributes() {
        let mut map = sample();
        map.root.color = Some("#c6262e".into());
        map.root.side = NodeSide::Left;
        map.root.layout = LayoutKind::Vertical;
        let back = document::from_xml(&document::to_xml(&map).unwrap()).unwrap();
        assert_eq!(back.root.color.as_deref(), Some("#c6262e"));
        assert_eq!(back.root.side, NodeSide::Left);
        assert_eq!(back.root.layout, LayoutKind::Vertical);
    }

    #[test]
    fn round_trips_free_notes_and_minder_skips_them() {
        let mut map = sample();
        map.notes.push(model::FreeNote {
            id: 1,
            pos: model::Point { x: -40.5, y: 120.0 },
            width: 300.0,
            height: 200.0,
            text: "# Ideias\n\n- a & b < c\n- **forte**".into(),
        });
        map.notes.push(model::FreeNote {
            id: 2,
            pos: model::Point { x: 10.0, y: 20.0 },
            width: 260.0,
            height: 180.0,
            text: String::new(),
        });
        let xml = document::to_xml(&map).unwrap();
        assert_eq!(document::from_xml(&xml).unwrap().notes, map.notes);
        // Sem notas soltas, o arquivo não ganha elemento novo.
        assert!(!document::to_xml(&sample()).unwrap().contains("nodo-notes"));
    }

    #[test]
    fn escapes_special_characters() {
        let mut map = MindMap::new("a & b < c > d \" e");
        map.add_child(0, "x & y").unwrap();
        let back = document::from_xml(&document::to_xml(&map).unwrap()).unwrap();
        assert_eq!(back.root.text, "a & b < c > d \" e");
        assert_eq!(back.root.children[0].text, "x & y");
    }

    #[test]
    fn finds_by_id_and_depth() {
        let map = sample();
        assert_eq!(map.depth_of(0), Some(0));
        assert_eq!(map.depth_of(1), Some(1));
        assert_eq!(map.depth_of(3), Some(2));
        assert!(map.find(3).is_some());
        assert!(map.find(999).is_none());
    }

    #[test]
    fn refuses_to_remove_root() {
        let mut map = sample();
        assert!(map.remove_node(0).is_none());
        assert_eq!(map.node_count(), 4);
    }

    #[test]
    fn removes_subtree() {
        let mut map = sample();
        let removed = map.remove_node(1).unwrap();
        assert_eq!(removed.subtree_len(), 2);
        assert_eq!(map.node_count(), 2);
    }

    #[test]
    fn layout_positions_children_apart() {
        let mut map = sample();
        layout::layout(&mut map);
        let kids = &map.root.children;
        assert_eq!(kids.len(), 2);
        assert!(kids[0].pos.y != kids[1].pos.y, "siblings must not overlap");
        assert!(kids[0].pos.x > map.root.pos.x, "right side grows rightward");
    }

    #[test]
    fn layout_respects_left_side() {
        let mut map = MindMap::new("Root");
        map.root.side = NodeSide::Left;
        map.add_child(0, "L").unwrap();
        layout::layout(&mut map);
        assert!(map.root.children[0].pos.x < map.root.pos.x);
    }

    #[test]
    fn layout_centres_parent_on_children() {
        let mut map = MindMap::new("Root");
        map.add_child(0, "A").unwrap();
        map.add_child(0, "B").unwrap();
        map.add_child(0, "C").unwrap();
        layout::layout(&mut map);
        let kids = &map.root.children;
        let mid = (kids[0].pos.y + kids[2].pos.y + kids[2].height) / 2.0;
        let parent_mid = map.root.pos.y + map.root.height / 2.0;
        assert!((mid - parent_mid).abs() < 1.0, "parent should be centred");
    }

    #[test]
    fn undo_redo_restores_state() {
        let mut hist = undo::History::new(sample());
        let before = hist.current().node_count();
        hist.apply(undo::Action::AddChild {
            parent: 0,
            text: "Gamma".into(),
        })
        .unwrap();
        assert_eq!(hist.current().node_count(), before + 1);
        assert!(hist.can_undo());
        hist.undo();
        assert_eq!(hist.current().node_count(), before);
        hist.redo();
        assert_eq!(hist.current().node_count(), before + 1);
    }

    #[test]
    fn undo_redo_removal() {
        let mut hist = undo::History::new(sample());
        hist.apply(undo::Action::RemoveNode { id: 1 }).unwrap();
        assert_eq!(hist.current().node_count(), 2);
        hist.undo();
        assert_eq!(hist.current().node_count(), 4);
    }

    #[test]
    fn move_node_carries_its_subtree() {
        // sample(): root(0) -> Alpha(1) -> Alpha-1(3); root -> Beta(2).
        let mut map = sample();
        layout::layout(&mut map);

        let child_pos_before = map.find(1).unwrap().pos;
        let grandchild_pos_before = map.find(3).unwrap().pos;

        undo::Action::MoveNode {
            id: 1,
            x: 500.0,
            y: 500.0,
        }
        .apply(&mut map)
        .unwrap();

        // O nó foi para onde pedimos…
        assert_eq!(map.find(1).unwrap().pos.x, 500.0);
        assert_eq!(map.find(1).unwrap().pos.y, 500.0);

        // …e o neto se moveu junto, pelo mesmo deslocamento.
        let dx = 500.0 - child_pos_before.x;
        let dy = 500.0 - child_pos_before.y;
        let g = map.find(3).unwrap().pos;
        assert!((g.x - (grandchild_pos_before.x + dx)).abs() < 1e-6);
        assert!((g.y - (grandchild_pos_before.y + dy)).abs() < 1e-6);
    }

    #[test]
    fn move_node_does_not_drift_other_branches() {
        let mut map = sample();
        layout::layout(&mut map);
        let beta_before = map.find(2).unwrap().pos;

        undo::Action::MoveNode {
            id: 1,
            x: 300.0,
            y: 300.0,
        }
        .apply(&mut map)
        .unwrap();

        // O ramo irmão não se move.
        assert_eq!(map.find(2).unwrap().pos, beta_before);
    }

    #[test]
    fn round_trips_notes() {
        let mut map = MindMap::new("Root");
        let id = map.add_child(0, "Child").unwrap();
        map.find_mut(id).unwrap().note = "a note with & and <b>markup</b>".into();
        let back = document::from_xml(&document::to_xml(&map).unwrap()).unwrap();
        assert_eq!(
            back.root.children[0].note,
            "a note with & and <b>markup</b>"
        );
    }

    #[test]
    fn parses_real_minder_file_shape() {
        let xml = r##"<?xml version="1.0"?>
<minder version="1.15.0">
  <theme name="default" label="Light" index="1"/>
  <styles><style level="0" branchmargin="100" nodewidth="500"/></styles>
  <drawarea x="-299.53125" y="-454.21875" scale="1"/>
  <images/>
  <nodes>
    <node id="0" posx="269.53125" posy="444.21875" width="63" height="46" side="right" fold="false" treesize="828" layout="Horizontal" group="false">
      <style nodemargin="8" nodepadding="6"/>
      <nodename posx="283.53125" posy="458.21875" maxwidth="500"><text data="build"/></nodename>
      <nodenote></nodenote>
      <nodes>
        <node id="1" posx="432.53125" posy="53.21875" width="98" height="46" side="right" fold="false" treesize="46" color="#c6262e" colorroot="false" layout="Horizontal" group="false">
          <style nodemargin="8" nodepadding="6"/>
          <nodename posx="446.53125" posy="67.21875" maxwidth="500"><text data=".gitignore"/></nodename>
          <nodenote></nodenote>
        </node>
      </nodes>
    </node>
  </nodes>
</minder>"##;
        let map = document::from_xml(xml).unwrap();
        assert_eq!(map.version, "1.15.0");
        assert_eq!(map.node_count(), 2);
        assert_eq!(map.root.text, "build");
        assert_eq!(map.root.children[0].text, ".gitignore");
        assert_eq!(map.root.children[0].color.as_deref(), Some("#c6262e"));
        assert_eq!(map.draw_area.x, -299.53125);
        assert_eq!(map.root.children[0].pos.x, 432.53125);
    }

    #[test]
    fn map_style_round_trips_as_an_extra_theme_attribute() {
        let mut map = sample();
        map.theme.style = Some("contorno".into());
        let xml = document::to_xml(&map).unwrap();
        assert!(
            xml.contains(r#"nodo-style="contorno""#),
            "atributo novo, que o Minder ignora"
        );
        let back = document::from_xml(&xml).unwrap();
        assert_eq!(back.theme.style.as_deref(), Some("contorno"));
        assert_eq!(
            back.theme.name, map.theme.name,
            "o tema do Minder continua lá"
        );

        map.theme.style = None;
        let xml = document::to_xml(&map).unwrap();
        assert!(
            !xml.contains("nodo-style"),
            "sem tema escolhido, o arquivo fica igual ao do Minder"
        );
    }

    #[test]
    fn map_style_is_an_undoable_edit() {
        let mut ed = Editor::new("Raiz");
        ed.apply(undo::Action::SetMapStyle {
            style: Some("vibrante".into()),
        })
        .unwrap();
        assert_eq!(ed.map().theme.style.as_deref(), Some("vibrante"));
        assert!(ed.is_dirty());
        ed.undo();
        assert_eq!(ed.map().theme.style, None);
    }

    #[test]
    fn unknown_map_style_is_refused() {
        let mut ed = Editor::new("Raiz");
        let err = ed.apply(undo::Action::SetMapStyle {
            style: Some("inexistente".into()),
        });
        assert!(matches!(err, Err(Error::UnknownStyle(_))));
        assert!(!ed.can_undo(), "recusado não entra no histórico");
    }

    #[test]
    fn reads_the_theme_attribute_written_before_the_rename() {
        // A 0.1.0 saiu com o nome Mapist e gravava `mapist-style`; quem abrir esses mapas mantém o tema.
        let xml = r#"<?xml version="1.0"?>
<minder version="1.15.0">
  <theme name="default" label="Light" index="1" mapist-style="vibrante"/>
  <nodes><node id="0" posx="0" posy="0" width="63" height="46" side="right" fold="false" layout="Horizontal">
    <nodename><text data="Raiz"/></nodename></node></nodes>
</minder>"#;
        let map = document::from_xml(xml).unwrap();
        assert_eq!(map.theme.style.as_deref(), Some("vibrante"));
        let again = document::to_xml(&map).unwrap();
        assert!(again.contains(r#"nodo-style="vibrante""#) && !again.contains("mapist-style"));
    }
}
