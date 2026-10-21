//! The open document: map, undo history, file and saved state in one place.
//!
//! Both doors use this type — the window through Tauri commands, the agent
//! through MCP tools — so an edit made by either one goes through exactly the
//! same steps: record for undo, apply, position. Before this existed each door
//! kept its own copy of the map beside the history, and edits that touched only
//! one of the two (folding, choosing a side, measured sizes) were silently lost
//! on the next change.

use crate::layout;
use crate::model::MindMap;
use crate::undo::{Action, History};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// An open mind map and everything needed to edit and save it.
#[derive(Debug, Clone)]
pub struct Editor {
    history: History,
    path: Option<PathBuf>,
    /// Identity of the current history state; changes on every edit, undo and redo.
    rev: u64,
    next_rev: u64,
    /// Revisions of the undo and redo stacks, kept in step with `history`.
    past_revs: Vec<u64>,
    future_revs: Vec<u64>,
    /// Revision that matches the file on disk, if any.
    saved_rev: Option<u64>,
    /// Modification time of the file when it was last read or written.
    disk_mtime: Option<SystemTime>,
}

impl Editor {
    /// A new, unsaved map with a single root.
    pub fn new(title: &str) -> Self {
        let title = title.trim();
        let mut map = MindMap::new(if title.is_empty() {
            "Tópico central"
        } else {
            title
        });
        layout::layout(&mut map);
        let mut ed = Self::from_map(map, None);
        // Nothing to lose yet: an untouched new map can be replaced without asking.
        ed.saved_rev = Some(ed.rev);
        ed
    }

    /// Open a `.minder` file.
    pub fn open(path: impl AsRef<Path>) -> crate::Result<Self> {
        let path = path.as_ref();
        let mut map = crate::load(path)?;
        layout::layout(&mut map);
        let mut ed = Self::from_map(map, Some(path.to_path_buf()));
        ed.saved_rev = Some(ed.rev);
        ed.disk_mtime = mtime(path);
        Ok(ed)
    }

    fn from_map(map: MindMap, path: Option<PathBuf>) -> Self {
        Self {
            history: History::new(map),
            path,
            rev: 0,
            next_rev: 1,
            past_revs: Vec::new(),
            future_revs: Vec::new(),
            saved_rev: None,
            disk_mtime: None,
        }
    }

    /// The map as it is now.
    pub fn map(&self) -> &MindMap {
        self.history.current()
    }

    /// The file this map was opened from or last saved to.
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Whether there are changes the file does not have.
    pub fn is_dirty(&self) -> bool {
        self.saved_rev != Some(self.rev)
    }

    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }

    pub fn can_redo(&self) -> bool {
        self.history.can_redo()
    }

    pub fn undo_depth(&self) -> usize {
        self.history.undo_depth()
    }

    pub fn redo_depth(&self) -> usize {
        self.history.redo_depth()
    }

    /// Apply an edit through the undo history, then position the map.
    ///
    /// Returns the affected node id. A node moved by hand keeps its place until
    /// the next edit that changes the tree's shape, as in Minder.
    pub fn apply(&mut self, action: Action) -> crate::Result<Option<u64>> {
        let id = self.history.apply(action.clone())?;
        self.past_revs.push(self.rev);
        self.past_revs
            .drain(..self.past_revs.len() - self.history.undo_depth());
        self.future_revs.clear();
        self.rev = self.bump();

        let map = self.history.current_mut();
        match action {
            Action::MoveNode { .. } | Action::Relayout => {}
            Action::AddChild { .. } | Action::SetText { .. } => {
                // Only the touched node gets the core's estimate; every other box
                // keeps the size the renderer measured.
                if let Some(node) = id.and_then(|id| map.find_mut(id)) {
                    layout::measure(node);
                }
                layout::reposition(map);
            }
            _ => layout::reposition(map),
        }
        Ok(id)
    }

    /// Box sizes measured by the renderer. Not an edit: no undo step, not dirty.
    pub fn set_sizes(&mut self, sizes: impl IntoIterator<Item = (u64, f64, f64)>) {
        let map = self.history.current_mut();
        for (id, width, height) in sizes {
            if let Some(node) = map.find_mut(id) {
                node.width = width;
                node.height = height;
            }
        }
        layout::reposition(map);
    }

    /// Step back one edit. Returns `false` when there is nothing to undo.
    pub fn undo(&mut self) -> bool {
        if !self.history.undo() {
            return false;
        }
        self.future_revs.push(self.rev);
        self.rev = self.past_revs.pop().unwrap_or_else(|| self.bump());
        true
    }

    /// Step forward one edit. Returns `false` when there is nothing to redo.
    pub fn redo(&mut self) -> bool {
        if !self.history.redo() {
            return false;
        }
        self.past_revs.push(self.rev);
        self.rev = self.future_revs.pop().unwrap_or_else(|| self.bump());
        true
    }

    /// Save to `path`, or to the current file when `None`. The write is atomic:
    /// a crash mid-save leaves the old file intact, never half a map.
    pub fn save(&mut self, path: Option<&Path>) -> crate::Result<PathBuf> {
        let target = path
            .map(Path::to_path_buf)
            .or_else(|| self.path.clone())
            .ok_or(crate::Error::NoPath)?;
        crate::save(self.map(), &target)?;
        self.disk_mtime = mtime(&target);
        self.path = Some(target.clone());
        self.saved_rev = Some(self.rev);
        Ok(target)
    }

    /// Whether the file changed on disk since it was read or written here,
    /// for example because an agent saved it through MCP.
    pub fn changed_on_disk(&self) -> bool {
        match (&self.path, self.disk_mtime) {
            (Some(p), Some(seen)) => mtime(p).is_some_and(|now| now != seen),
            _ => false,
        }
    }

    fn bump(&mut self) -> u64 {
        let r = self.next_rev;
        self.next_rev += 1;
        r
    }
}

fn mtime(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::NodeSide;

    fn add(ed: &mut Editor, parent: u64, text: &str) -> u64 {
        ed.apply(Action::AddChild {
            parent,
            text: text.into(),
        })
        .unwrap()
        .unwrap()
    }

    fn order(ed: &Editor, parent: u64) -> Vec<String> {
        ed.map()
            .find(parent)
            .unwrap()
            .children
            .iter()
            .map(|c| c.text.clone())
            .collect()
    }

    #[test]
    fn reorder_moves_a_topic_among_its_siblings_and_undoes() {
        let mut ed = Editor::new("Raiz");
        let p = add(&mut ed, 0, "P");
        add(&mut ed, p, "A");
        add(&mut ed, p, "B");
        let c = add(&mut ed, p, "C");

        ed.apply(Action::Reorder { id: c, delta: -1 }).unwrap();
        assert_eq!(order(&ed, p), ["A", "C", "B"]);
        // O layout acompanha: C fica acima de B.
        let y = |ed: &Editor, t: &str| {
            ed.map()
                .find(p)
                .unwrap()
                .children
                .iter()
                .find(|n| n.text == t)
                .unwrap()
                .pos
                .y
        };
        assert!(y(&ed, "C") < y(&ed, "B"));

        ed.apply(Action::Reorder { id: c, delta: -1 }).unwrap();
        assert_eq!(order(&ed, p), ["C", "A", "B"]);
        // Já é o primeiro: nada muda e nada entra no histórico.
        let depth = ed.undo_depth();
        assert!(matches!(
            ed.apply(Action::Reorder { id: c, delta: -1 }),
            Err(crate::Error::NoSiblingThere)
        ));
        assert_eq!(ed.undo_depth(), depth);

        ed.undo();
        ed.undo();
        assert_eq!(order(&ed, p), ["A", "B", "C"]);
    }

    #[test]
    fn reorder_on_the_root_only_swaps_with_branches_on_the_same_side() {
        let mut ed = Editor::new("Raiz");
        let a = add(&mut ed, 0, "A");
        let b = add(&mut ed, 0, "B");
        let c = add(&mut ed, 0, "C");
        ed.apply(Action::SetSide {
            id: a,
            side: NodeSide::Right,
        })
        .unwrap();
        ed.apply(Action::SetSide {
            id: b,
            side: NodeSide::Left,
        })
        .unwrap();
        ed.apply(Action::SetSide {
            id: c,
            side: NodeSide::Right,
        })
        .unwrap();

        // C sobe para antes de A (o vizinho do mesmo lado), passando por cima de B, que é do outro lado.
        ed.apply(Action::Reorder { id: c, delta: -1 }).unwrap();
        assert_eq!(order(&ed, 0), ["C", "B", "A"]);
        // B é o único à esquerda: não tem para onde ir.
        assert!(ed.apply(Action::Reorder { id: b, delta: 1 }).is_err());
        assert!(ed.apply(Action::Reorder { id: 0, delta: 1 }).is_err());
    }

    #[test]
    fn fold_survives_the_next_edit_and_undoes() {
        let mut ed = Editor::new("Raiz");
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
        ed.apply(Action::ToggleFold { id: a }).unwrap();
        ed.apply(Action::SetText {
            id: 0,
            text: "Raiz nova".into(),
        })
        .unwrap();
        assert!(
            ed.map().find(a).unwrap().folded,
            "dobrar não pode se perder na edição seguinte"
        );
        ed.undo();
        ed.undo();
        assert!(!ed.map().find(a).unwrap().folded);
    }

    #[test]
    fn side_survives_the_next_edit() {
        let mut ed = Editor::new("Raiz");
        let a = ed
            .apply(Action::AddChild {
                parent: 0,
                text: "A".into(),
            })
            .unwrap()
            .unwrap();
        let a1 = ed
            .apply(Action::AddChild {
                parent: a,
                text: "A1".into(),
            })
            .unwrap()
            .unwrap();
        ed.apply(Action::SetSide {
            id: a,
            side: NodeSide::Left,
        })
        .unwrap();
        ed.apply(Action::AddChild {
            parent: 0,
            text: "B".into(),
        })
        .unwrap();
        let map = ed.map();
        assert_eq!(
            map.find(a1).unwrap().side,
            NodeSide::Left,
            "o ramo inteiro muda de lado"
        );
        assert!(map.find(a).unwrap().pos.x + map.find(a).unwrap().width <= map.root.pos.x);
        assert!(
            map.find(a1).unwrap().pos.x + map.find(a1).unwrap().width <= map.find(a).unwrap().pos.x
        );
    }

    #[test]
    fn measured_sizes_are_kept_and_do_not_dirty() {
        let mut ed = Editor::new("Raiz");
        let a = ed
            .apply(Action::AddChild {
                parent: 0,
                text: "A".into(),
            })
            .unwrap()
            .unwrap();
        let b = ed
            .apply(Action::AddChild {
                parent: 0,
                text: "B".into(),
            })
            .unwrap()
            .unwrap();
        ed.set_sizes([(a, 300.0, 80.0)]);
        ed.apply(Action::SetColor {
            id: b,
            color: Some("#c6262e".into()),
        })
        .unwrap();
        assert_eq!(ed.map().find(a).unwrap().width, 300.0);
        assert_eq!(ed.undo_depth(), 3, "medir não é uma edição");
    }

    #[test]
    fn a_moved_node_stays_where_it_was_dropped() {
        let mut ed = Editor::new("Raiz");
        let a = ed
            .apply(Action::AddChild {
                parent: 0,
                text: "A".into(),
            })
            .unwrap()
            .unwrap();
        ed.apply(Action::MoveNode {
            id: a,
            x: 400.0,
            y: -300.0,
        })
        .unwrap();
        assert_eq!(ed.map().find(a).unwrap().pos.x, 400.0);
        assert_eq!(ed.map().find(a).unwrap().pos.y, -300.0);
    }

    #[test]
    fn dirty_follows_undo_back_to_the_saved_state() {
        let dir = std::env::temp_dir().join(format!("nodo-editor-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("mapa.minder");
        let mut ed = Editor::new("Raiz");
        assert!(
            !ed.is_dirty(),
            "mapa novo, sem edição, não tem o que perder"
        );
        ed.apply(Action::SetText {
            id: 0,
            text: "Raiz".into(),
        })
        .unwrap();
        assert!(ed.is_dirty());
        ed.save(Some(&file)).unwrap();
        assert!(!ed.is_dirty());
        ed.apply(Action::AddChild {
            parent: 0,
            text: "A".into(),
        })
        .unwrap();
        assert!(ed.is_dirty());
        ed.undo();
        assert!(
            !ed.is_dirty(),
            "desfazer até o que está no disco não deixa nada por salvar"
        );
        ed.redo();
        assert!(ed.is_dirty());

        let reopened = Editor::open(&file).unwrap();
        assert!(!reopened.is_dirty());
        assert!(!reopened.changed_on_disk());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn save_without_a_path_is_an_error() {
        let mut ed = Editor::new("Raiz");
        assert!(matches!(ed.save(None), Err(crate::Error::NoPath)));
    }
}
