//! The maps open in the window, one per tab. Each tab is a full `Editor` (its own history, file and saved state);
//! the commands act on the active one. Opening a file reuses an untouched new map, and a file that is already open
//! brings its tab to the front instead of opening twice.

use nodo_core::Editor;
use std::path::Path;

/// One open map.
pub struct Tab {
    /// Stable for the tab's life (the interface keys its zoom and selection by it).
    pub id: u64,
    pub editor: Editor,
}

/// The open maps and which one is in front.
#[derive(Default)]
pub struct Tabs {
    list: Vec<Tab>,
    active: usize,
    next_id: u64,
}

/// A new map nobody touched yet: no file, nothing done to it. Opening a file takes its place.
fn untouched(ed: &Editor) -> bool {
    ed.path().is_none() && !ed.is_dirty() && !ed.can_undo()
}

impl Tabs {
    /// The map in front, if any.
    pub fn current(&self) -> Option<&Editor> {
        self.list.get(self.active).map(|t| &t.editor)
    }

    pub fn current_mut(&mut self) -> Option<&mut Editor> {
        self.list.get_mut(self.active).map(|t| &mut t.editor)
    }

    /// The id of the tab in front.
    pub fn active_id(&self) -> Option<u64> {
        self.list.get(self.active).map(|t| t.id)
    }

    /// All tabs, in order.
    pub fn iter(&self) -> impl Iterator<Item = &Tab> {
        self.list.iter()
    }

    /// Whether any open map has changes not yet saved.
    pub fn any_dirty(&self) -> bool {
        self.list.iter().any(|t| t.editor.is_dirty())
    }

    /// A new tab with this map, in front. Returns its id.
    pub fn add(&mut self, editor: Editor) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.list.push(Tab { id, editor });
        self.active = self.list.len() - 1;
        id
    }

    /// A map read from a file: it takes the place of an untouched new map in front, or gets a tab of its own.
    pub fn open(&mut self, editor: Editor) -> u64 {
        if self.current().is_some_and(untouched) {
            self.replace_current(editor);
            return self.active_id().unwrap_or_default();
        }
        self.add(editor)
    }

    /// The tab that already has this file open, if any.
    pub fn find_path(&self, path: &Path) -> Option<u64> {
        let wanted = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        self.list
            .iter()
            .find(|t| {
                t.editor
                    .path()
                    .is_some_and(|p| p.canonicalize().unwrap_or_else(|_| p.to_path_buf()) == wanted)
            })
            .map(|t| t.id)
    }

    /// Put another map in the tab in front, keeping the tab (reloading the file an agent saved).
    pub fn replace_current(&mut self, editor: Editor) {
        match self.list.get_mut(self.active) {
            Some(tab) => tab.editor = editor,
            None => {
                self.add(editor);
            }
        }
    }

    /// Bring a tab to the front. `false` if there is no such tab.
    pub fn switch(&mut self, id: u64) -> bool {
        match self.list.iter().position(|t| t.id == id) {
            Some(i) => {
                self.active = i;
                true
            }
            None => false,
        }
    }

    /// Close a tab (the interface asked about unsaved changes first). The one to its right comes to the front, or
    /// the one to its left when it was the last. `false` if there is no such tab.
    pub fn close(&mut self, id: u64) -> bool {
        let Some(i) = self.list.iter().position(|t| t.id == id) else {
            return false;
        };
        self.list.remove(i);
        if self.active > i || self.active >= self.list.len() {
            self.active = self.active.saturating_sub(1);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nodo_core::undo::Action;

    fn edited(title: &str) -> Editor {
        let mut ed = Editor::new(title);
        ed.apply(Action::AddChild {
            parent: 0,
            text: "Filho".into(),
        })
        .unwrap();
        ed
    }

    fn titles(t: &Tabs) -> Vec<String> {
        t.iter().map(|t| t.editor.map().root.text.clone()).collect()
    }

    #[test]
    fn new_maps_get_their_own_tab_in_front() {
        let mut t = Tabs::default();
        assert!(t.current().is_none());
        let a = t.add(Editor::new("A"));
        let b = t.add(Editor::new("B"));
        assert_ne!(a, b);
        assert_eq!(t.active_id(), Some(b));
        assert_eq!(titles(&t), ["A", "B"]);
    }

    #[test]
    fn opening_a_file_reuses_an_untouched_new_map_but_not_one_with_work() {
        let mut t = Tabs::default();
        let first = t.add(Editor::new("Tópico central"));
        assert_eq!(
            t.open(Editor::new("Do arquivo")),
            first,
            "o mapa novo intocado dá lugar ao arquivo"
        );
        assert_eq!(titles(&t), ["Do arquivo"]);

        let mut t = Tabs::default();
        t.add(edited("Rascunho"));
        let opened = t.open(Editor::new("Do arquivo"));
        assert_eq!(
            titles(&t),
            ["Rascunho", "Do arquivo"],
            "o que tem trabalho fica na aba dele"
        );
        assert_eq!(t.active_id(), Some(opened));
    }

    #[test]
    fn closing_brings_the_neighbour_to_the_front() {
        let mut t = Tabs::default();
        let a = t.add(Editor::new("A"));
        let b = t.add(Editor::new("B"));
        let c = t.add(Editor::new("C"));
        assert!(t.switch(b));
        assert!(t.close(b));
        assert_eq!(t.active_id(), Some(c), "a da direita");
        assert!(t.close(c));
        assert_eq!(t.active_id(), Some(a), "a última fechada: a da esquerda");
        assert!(t.switch(a) && t.close(a));
        assert!(t.current().is_none());
        assert!(!t.close(a) && !t.switch(a), "aba que não existe");
    }

    #[test]
    fn closing_a_tab_behind_keeps_the_one_in_front() {
        let mut t = Tabs::default();
        let a = t.add(Editor::new("A"));
        let b = t.add(Editor::new("B"));
        assert!(t.close(a));
        assert_eq!(t.active_id(), Some(b));
        assert_eq!(t.current().map(|e| e.map().root.text.as_str()), Some("B"));
    }

    #[test]
    fn a_file_already_open_is_found_and_unsaved_work_is_seen_in_any_tab() {
        let dir = std::env::temp_dir().join(format!("nodo-tabs-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("viagem.nodo");
        let mut ed = Editor::new("Viagem");
        ed.save(Some(&path)).unwrap();
        let mut t = Tabs::default();
        let id = t.add(ed);
        t.add(edited("Outro"));
        assert_eq!(t.find_path(&path), Some(id));
        assert_eq!(t.find_path(&dir.join("nao-existe.nodo")), None);
        assert!(t.any_dirty(), "a aba de trás tem alteração");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
