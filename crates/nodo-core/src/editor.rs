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
    /// Photos, audio and files the notes use (kept in the package; not part of the undo history: an undo that brings
    /// a reference back finds the attachment still here).
    attachments: crate::attachments::Attachments,
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

    /// Open a `.nodo` file.
    pub fn open(path: impl AsRef<Path>) -> crate::Result<Self> {
        let path = path.as_ref();
        let (mut map, attachments) = crate::package::read(path)?;
        layout::layout(&mut map);
        let mut ed = Self::from_map(map, Some(path.to_path_buf()));
        ed.attachments = attachments;
        ed.saved_rev = Some(ed.rev);
        ed.disk_mtime = mtime(path);
        Ok(ed)
    }

    /// Open a `.nodo` map, or import anything else Nodo reads as a new map: a `.minder` from the early versions
    /// (D27), Markdown (`.md`, `.markdown`, `.txt`) or OPML (`.opml`). An import has no path: saving asks where, so the
    /// original file is never overwritten.
    pub fn open_or_import(path: impl AsRef<Path>) -> crate::Result<Self> {
        let path = path.as_ref();
        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let stem = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        match ext.as_str() {
            "md" | "markdown" | "txt" => {
                let text = std::fs::read_to_string(path)?;
                Ok(Self::imported(crate::import::from_markdown(&text, &stem)))
            }
            "opml" => {
                let text = std::fs::read_to_string(path)?;
                Ok(Self::imported(crate::import::from_opml(&text, &stem)?))
            }
            "minder" => {
                let text = std::fs::read_to_string(path)?;
                Ok(Self::imported(crate::document::from_xml(&text)?))
            }
            "nodo" => Self::open(path),
            // Sem extensão conhecida: decide pelo conteúdo (um pacote começa com "PK"; o XML antigo, com `<`).
            _ => {
                let bytes = std::fs::read(path)?;
                if crate::package::is_package(&bytes) {
                    return Self::open(path);
                }
                let text = String::from_utf8_lossy(&bytes);
                if text.trim_start().starts_with('<') {
                    Ok(Self::imported(crate::document::from_xml(&text)?))
                } else {
                    Self::open(path)
                }
            }
        }
    }

    /// A map that came from somewhere else (an imported Markdown): new and not saved yet, so closing it asks and
    /// saving it asks where.
    pub fn imported(mut map: MindMap) -> Self {
        layout::layout(&mut map);
        Self::from_map(map, None)
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
            attachments: crate::attachments::Attachments::default(),
        }
    }

    /// The attachments of this map.
    pub fn attachments(&self) -> &crate::attachments::Attachments {
        &self.attachments
    }

    /// Bring a file into the map (a photo, a recording, a document); the note then refers to it by its hash. Not an
    /// edit by itself: the map changes when a note starts using it.
    pub fn add_attachment(
        &mut self,
        name: &str,
        bytes: Vec<u8>,
        now: i64,
    ) -> crate::attachments::Attachment {
        self.attachments.add(name, bytes, now)
    }

    /// The content of an attachment.
    pub fn attachment_bytes(&self, hash: &str) -> crate::Result<Vec<u8>> {
        self.attachments.bytes(hash)
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
            // Notas soltas não mexem na árvore: nada a reposicionar.
            Action::MoveNode { .. }
            | Action::Relayout
            | Action::AddFreeNote { .. }
            | Action::SetFreeNote { .. }
            | Action::PlaceFreeNote { .. }
            | Action::RemoveFreeNote { .. } => {}
            Action::AddChild { .. } | Action::AddTopic { .. } | Action::SetText { .. } => {
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

    /// Save to `path`, or to the current file when `None`, always as `.nodo` (another extension is swapped, so a
    /// `.minder` or a Markdown is never overwritten with a Nodo map). Returns where it went. The write is atomic:
    /// a crash mid-save leaves the old file intact, never half a map.
    pub fn save(&mut self, path: Option<&Path>) -> crate::Result<PathBuf> {
        let mut target = path
            .map(Path::to_path_buf)
            .or_else(|| self.path.clone())
            .ok_or(crate::Error::NoPath)?;
        if !target
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case(crate::EXTENSION))
        {
            target.set_extension(crate::EXTENSION);
        }
        let map = self.history.current().clone();
        crate::package::write(&target, &map, &mut self.attachments)?;
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

    fn tree(ed: &mut Editor) -> (u64, u64, u64) {
        let a = add(ed, 0, "A");
        let b = add(ed, 0, "B");
        let a1 = add(ed, a, "A1");
        add(ed, a, "A2");
        add(ed, b, "B1");
        ed.apply(Action::SetSide {
            id: b,
            side: NodeSide::Left,
        })
        .unwrap();
        (a, b, a1)
    }

    fn node(ed: &Editor, id: u64) -> &crate::model::Node {
        ed.map().find(id).unwrap()
    }

    #[test]
    fn card_fields_are_cleaned_and_undo_in_one_step() {
        let mut ed = Editor::new("Mapa");
        let id = add(&mut ed, 0, "Ideias");
        let card = crate::model::CardFields {
            tags: vec!["#Viagem".into(), "viagem".into()],
            emoji: Some("✈️".into()),
            status_on: true,
            reminder_doing: true,
            ..Default::default()
        };
        ed.apply(Action::SetCard { id, card }).unwrap();
        let got = &node(&ed, id).card;
        assert_eq!(got.tags, vec!["viagem".to_string()]);
        assert_eq!(got.emoji.as_deref(), Some("✈️"));
        assert!(got.status_on && got.reminder_doing);
        assert!(ed.is_dirty());
        assert!(ed.undo());
        assert_eq!(node(&ed, id).card, crate::model::CardFields::default());
        assert!(matches!(
            ed.apply(Action::SetCard {
                id: 99,
                card: Default::default()
            }),
            Err(crate::Error::NodeNotFound(99))
        ));
    }

    #[test]
    fn a_topic_with_a_note_is_one_undo_step() {
        let mut ed = Editor::new("Mapa");
        let id = ed
            .apply(Action::AddTopic {
                parent: 0,
                text: "Ideias".into(),
                note: "- [ ] Revisar".into(),
                color: Some("#6eb0f7".into()),
                card: crate::model::CardFields {
                    tags: vec!["#Casa".into()],
                    ..Default::default()
                },
            })
            .unwrap()
            .unwrap();
        assert_eq!(node(&ed, id).note, "- [ ] Revisar");
        assert_eq!(node(&ed, id).color.as_deref(), Some("#6eb0f7"));
        assert_eq!(node(&ed, id).card.tags, vec!["casa".to_string()]);
        assert!(ed.undo());
        assert!(ed.map().find(id).is_none());
        assert!(matches!(
            ed.apply(Action::AddTopic {
                parent: 99,
                text: String::new(),
                note: String::new(),
                color: None,
                card: Default::default(),
            }),
            Err(crate::Error::NodeNotFound(99))
        ));
    }

    #[test]
    fn free_notes_are_added_edited_moved_and_removed_through_the_history() {
        let mut ed = Editor::new("Mapa");
        let before = ed.map().root.clone();
        let id = ed
            .apply(Action::AddFreeNote {
                x: 400.0,
                y: -80.0,
                text: String::new(),
            })
            .unwrap()
            .unwrap();
        ed.apply(Action::SetFreeNote {
            id,
            text: "Lembrar do **prazo**".into(),
        })
        .unwrap();
        ed.apply(Action::PlaceFreeNote {
            id,
            x: 420.0,
            y: -60.0,
            width: 320.0,
            height: 10.0,
        })
        .unwrap();
        let n = ed.map().free_note(id).unwrap();
        assert_eq!((n.pos.x, n.pos.y, n.width), (420.0, -60.0, 320.0));
        assert_eq!(n.height, 80.0, "a note never shrinks out of sight");
        assert_eq!(n.text, "Lembrar do **prazo**");
        // Uma nota solta não mexe na árvore.
        assert_eq!(ed.map().root, before);
        assert!(ed.is_dirty());

        let second = ed
            .apply(Action::AddFreeNote {
                x: 0.0,
                y: 0.0,
                text: "Outra".into(),
            })
            .unwrap()
            .unwrap();
        assert_ne!(second, id);
        ed.apply(Action::RemoveFreeNote { id }).unwrap();
        assert!(ed.map().free_note(id).is_none());
        assert!(matches!(
            ed.apply(Action::SetFreeNote {
                id,
                text: "x".into()
            }),
            Err(crate::Error::NoteNotFound(_))
        ));

        ed.undo();
        assert_eq!(ed.map().free_note(id).unwrap().text, "Lembrar do **prazo**");
        while ed.undo() {}
        assert!(ed.map().notes.is_empty());
        assert!(!ed.is_dirty());
    }

    #[test]
    fn logic_structure_grows_everything_to_the_right_and_keeps_the_sides() {
        let mut ed = Editor::new("Raiz");
        let (_, b, _) = tree(&mut ed);
        ed.apply(Action::SetStructure {
            structure: Some("logico".into()),
        })
        .unwrap();
        let root = node(&ed, 0).clone();
        for n in ed.map().nodes().filter(|n| n.id != 0) {
            assert!(
                n.pos.x > root.pos.x + root.width - 1e-6,
                "{} não está à direita",
                n.text
            );
        }
        // O lado escolhido continua guardado: voltar para Mapa devolve B à esquerda.
        assert_eq!(node(&ed, b).side, NodeSide::Left);
        ed.apply(Action::SetStructure { structure: None }).unwrap();
        assert!(node(&ed, b).pos.x + node(&ed, b).width <= root.pos.x + 1e-6);
    }

    #[test]
    fn org_chart_puts_children_below_their_parent_on_one_row() {
        let mut ed = Editor::new("Raiz");
        let (a, b, _) = tree(&mut ed);
        ed.apply(Action::SetStructure {
            structure: Some("organograma".into()),
        })
        .unwrap();
        let root = node(&ed, 0).clone();
        let (na, nb) = (node(&ed, a).clone(), node(&ed, b).clone());
        assert!(na.pos.y >= root.pos.y + root.height);
        assert!((na.pos.y - nb.pos.y).abs() < 1e-6, "irmãos na mesma linha");
        assert!(na.pos.x + na.width <= nb.pos.x + 1e-6 || nb.pos.x + nb.width <= na.pos.x + 1e-6);
        for c in &na.children {
            assert!(c.pos.y >= na.pos.y + na.height);
        }
    }

    #[test]
    fn tree_structure_is_an_indented_list_in_reading_order() {
        let mut ed = Editor::new("Raiz");
        let (a, _, a1) = tree(&mut ed);
        ed.apply(Action::SetStructure {
            structure: Some("arvore".into()),
        })
        .unwrap();
        let order: Vec<_> = ed.map().nodes().map(|n| (n.id, n.pos.y)).collect();
        for w in order.windows(2) {
            assert!(w[0].1 < w[1].1, "a ordem de leitura desce na tela");
        }
        assert!(node(&ed, a1).pos.x > node(&ed, a).pos.x);
        assert!(node(&ed, a).pos.x > node(&ed, 0).pos.x);
    }

    #[test]
    fn spacing_widens_the_gaps_and_undoes() {
        let mut ed = Editor::new("Raiz");
        let (a, _, _) = tree(&mut ed);
        let gap = |ed: &Editor| node(ed, a).pos.x - (node(ed, 0).pos.x + node(ed, 0).width);
        let normal = gap(&ed);
        ed.apply(Action::SetSpacing {
            spacing: Some("largo".into()),
        })
        .unwrap();
        assert!(gap(&ed) > normal);
        ed.apply(Action::SetSpacing {
            spacing: Some("compacto".into()),
        })
        .unwrap();
        assert!(gap(&ed) < normal);
        ed.undo();
        ed.undo();
        assert!((gap(&ed) - normal).abs() < 1e-6);
        assert!(ed
            .apply(Action::SetSpacing {
                spacing: Some("enorme".into())
            })
            .is_err());
        assert!(ed
            .apply(Action::SetStructure {
                structure: Some("espiral".into())
            })
            .is_err());
    }

    #[test]
    fn importing_markdown_gives_an_unsaved_map_that_keeps_the_original_file() {
        let dir = std::env::temp_dir().join(format!("nodo-import-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let md = dir.join("Compras.md");
        std::fs::write(&md, "- Leite\n- Pão\n").unwrap();
        let ed = Editor::open_or_import(&md).unwrap();
        assert_eq!(ed.map().root.text, "Compras");
        assert_eq!(ed.map().root.children.len(), 2);
        assert!(ed.path().is_none(), "salvar tem de perguntar onde");
        assert!(ed.is_dirty(), "fechar sem salvar pergunta");
        assert_eq!(std::fs::read_to_string(&md).unwrap(), "- Leite\n- Pão\n");
        let _ = std::fs::remove_dir_all(&dir);
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
    fn an_old_minder_opens_as_an_import_and_is_never_overwritten() {
        let dir = std::env::temp_dir().join(format!("nodo-minder-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let old = dir.join("antigo.minder");
        let mut map = crate::MindMap::new("Plano");
        map.add_child(0, "Ideia").unwrap();
        let xml = crate::document::to_xml(&map).unwrap();
        std::fs::write(&old, &xml).unwrap();

        let mut ed = Editor::open_or_import(&old).unwrap();
        assert_eq!(ed.map().root.children[0].text, "Ideia");
        assert!(ed.path().is_none(), "import: salvar pergunta onde");
        assert!(ed.is_dirty());

        // Mesmo pedindo para salvar em cima do .minder, vai para um .nodo ao lado.
        let saved = ed.save(Some(&old)).unwrap();
        assert_eq!(saved, dir.join("antigo.nodo"));
        assert_eq!(std::fs::read_to_string(&old).unwrap(), xml);
        let reopened = Editor::open_or_import(&saved).unwrap();
        assert_eq!(reopened.path(), Some(saved.as_path()));
        assert!(!reopened.is_dirty());
        assert_eq!(reopened.map().root.children[0].text, "Ideia");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn dirty_follows_undo_back_to_the_saved_state() {
        let dir = std::env::temp_dir().join(format!("nodo-editor-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("mapa.nodo");
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
