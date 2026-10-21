//! Undo/redo.
//!
//! Minder models undo as one command object per user-visible action (110 files
//! under `src/undo_actions/`). This crate takes the simpler, equivalent route
//! for the first phase: an [`Action`] describes an intent, [`Action::apply`]
//! performs it, and [`History`] keeps snapshot pairs so that undo is exact
//! rather than an inverse operation that might drift.
//!
//! Snapshots are cheap at this scale — a mind map is kilobytes — and they make
//! correctness obvious. A command-inverse scheme can replace this later without
//! changing the public surface.

use crate::model::{FreeNote, MindMap, Node, NodeSide, Point, FREE_NOTE_HEIGHT, FREE_NOTE_WIDTH};

/// A user-visible edit.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// Add a child under `parent` with the given text.
    AddChild { parent: u64, text: String },
    /// Add a child that already has a note (a card pasted from the Ideario, say), as one undo step.
    AddTopic {
        parent: u64,
        text: String,
        note: String,
    },
    /// Remove a node and its subtree.
    RemoveNode { id: u64 },
    /// Replace a node's text.
    SetText { id: u64, text: String },
    /// Replace a node's note.
    SetNote { id: u64, note: String },
    /// Move a node to an absolute position.
    MoveNode { id: u64, x: f64, y: f64 },
    /// Set a node's branch colour, or clear it with `None`.
    SetColor { id: u64, color: Option<String> },
    /// Place a branch on one side of the root; its whole subtree follows.
    SetSide { id: u64, side: NodeSide },
    /// Fold or unfold a node's children.
    ToggleFold { id: u64 },
    /// Discard hand-placed positions and lay the whole map out again.
    Relayout,
    /// Choose the map's theme (one of [`crate::MAP_STYLES`]), or the default with `None`.
    SetMapStyle { style: Option<String> },
    /// Move a node among its siblings: `delta < 0` up (earlier), `delta > 0` down. Under the root it swaps with the
    /// nearest branch on the same side, since each side reads as its own list.
    Reorder { id: u64, delta: i64 },
    /// Arrange the whole map (one of [`crate::STRUCTURES`]), or the classic map with `None`. Also discards hand-placed
    /// positions, like [`Action::Relayout`].
    SetStructure { structure: Option<String> },
    /// Room between nodes (one of [`crate::SPACINGS`]), or normal with `None`.
    SetSpacing { spacing: Option<String> },
    /// Put a loose note on the canvas with its top-left corner at `(x, y)`. Returns the new note's id.
    AddFreeNote { x: f64, y: f64, text: String },
    /// Replace a loose note's text.
    SetFreeNote { id: u64, text: String },
    /// Move or resize a loose note.
    PlaceFreeNote {
        id: u64,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    },
    /// Take a loose note off the canvas.
    RemoveFreeNote { id: u64 },
}

impl Action {
    /// Apply this action to a map. Returns the affected node id when the
    /// action created or targeted one.
    pub fn apply(&self, map: &mut MindMap) -> crate::Result<Option<u64>> {
        match self {
            Action::AddChild { parent, text } => {
                if map.find(*parent).is_none() {
                    return Err(crate::Error::NodeNotFound(*parent));
                }
                Ok(map.add_child(*parent, text.clone()))
            }
            Action::AddTopic { parent, text, note } => {
                let id = map
                    .add_child(*parent, text.clone())
                    .ok_or(crate::Error::NodeNotFound(*parent))?;
                if let Some(n) = map.find_mut(id) {
                    n.note = note.clone();
                }
                Ok(Some(id))
            }
            Action::RemoveNode { id } => {
                if *id == map.root.id {
                    return Err(crate::Error::CannotRemoveRoot);
                }
                map.remove_node(*id)
                    .map(|_| Some(*id))
                    .ok_or(crate::Error::NodeNotFound(*id))
            }
            Action::SetText { id, text } => {
                let node = map.find_mut(*id).ok_or(crate::Error::NodeNotFound(*id))?;
                node.text = text.clone();
                Ok(Some(*id))
            }
            Action::SetNote { id, note } => {
                let node = map.find_mut(*id).ok_or(crate::Error::NodeNotFound(*id))?;
                node.note = note.clone();
                Ok(Some(*id))
            }
            Action::MoveNode { id, x, y } => {
                let node = map.find_mut(*id).ok_or(crate::Error::NodeNotFound(*id))?;
                let dx = *x - node.pos.x;
                let dy = *y - node.pos.y;
                // Move the whole subtree rigidly, so children follow their parent
                // rather than staying behind at their old positions.
                node.pos.x = *x;
                node.pos.y = *y;
                shift_subtree(node, dx, dy);
                Ok(Some(*id))
            }
            Action::SetColor { id, color } => {
                let node = map.find_mut(*id).ok_or(crate::Error::NodeNotFound(*id))?;
                node.color = color.clone();
                Ok(Some(*id))
            }
            Action::SetSide { id, side } => {
                let node = map.find_mut(*id).ok_or(crate::Error::NodeNotFound(*id))?;
                crate::layout::for_each_mut(node, &mut |n| n.side = *side);
                Ok(Some(*id))
            }
            Action::ToggleFold { id } => {
                let node = map.find_mut(*id).ok_or(crate::Error::NodeNotFound(*id))?;
                node.folded = !node.folded;
                Ok(Some(*id))
            }
            Action::Relayout => {
                crate::layout::layout(map);
                Ok(None)
            }
            Action::SetMapStyle { style } => {
                if let Some(s) = style {
                    if !crate::MAP_STYLES.contains(&s.as_str()) {
                        return Err(crate::Error::UnknownStyle(s.clone()));
                    }
                }
                map.theme.style = style.clone();
                Ok(None)
            }
            Action::SetStructure { structure } => {
                if let Some(s) = structure {
                    if !crate::STRUCTURES.contains(&s.as_str()) {
                        return Err(crate::Error::UnknownStructure(s.clone()));
                    }
                }
                map.theme.structure = structure.clone();
                // O Minder não conhece a estrutura: cada nó leva o layout dele mais parecido.
                let kind = crate::layout::minder_layout(structure.as_deref());
                crate::layout::for_each_mut(&mut map.root, &mut |n| n.layout = kind);
                Ok(None)
            }
            Action::SetSpacing { spacing } => {
                if let Some(s) = spacing {
                    if !crate::SPACINGS.contains(&s.as_str()) {
                        return Err(crate::Error::UnknownSpacing(s.clone()));
                    }
                }
                map.theme.spacing = spacing.clone();
                Ok(None)
            }
            Action::AddFreeNote { x, y, text } => {
                let id = map.notes.iter().map(|n| n.id).max().map_or(1, |m| m + 1);
                map.notes.push(FreeNote {
                    id,
                    pos: Point { x: *x, y: *y },
                    width: FREE_NOTE_WIDTH,
                    height: FREE_NOTE_HEIGHT,
                    text: text.clone(),
                });
                Ok(Some(id))
            }
            Action::SetFreeNote { id, text } => {
                free_note_mut(map, *id)?.text = text.clone();
                Ok(Some(*id))
            }
            Action::PlaceFreeNote {
                id,
                x,
                y,
                width,
                height,
            } => {
                let n = free_note_mut(map, *id)?;
                n.pos = Point { x: *x, y: *y };
                // Um card menor que isto some de vista; maior que isto não é mais uma nota.
                n.width = width.clamp(120.0, 1600.0);
                n.height = height.clamp(80.0, 1600.0);
                Ok(Some(*id))
            }
            Action::RemoveFreeNote { id } => {
                let before = map.notes.len();
                map.notes.retain(|n| n.id != *id);
                if map.notes.len() == before {
                    return Err(crate::Error::NoteNotFound(*id));
                }
                Ok(Some(*id))
            }
            Action::Reorder { id, delta } => {
                let root = map.root.id;
                let parent = parent_of_mut(&mut map.root, *id).ok_or(if *id == root {
                    crate::Error::NoSiblingThere
                } else {
                    crate::Error::NodeNotFound(*id)
                })?;
                let at = parent
                    .children
                    .iter()
                    .position(|c| c.id == *id)
                    .ok_or(crate::Error::NodeNotFound(*id))?;
                let side = parent.children[at].side;
                let same_side = parent.id == root;
                let fits = |n: &Node| !same_side || n.side == side;
                let to = match delta.signum() {
                    -1 => parent.children[..at].iter().rposition(fits),
                    1 => parent.children[at + 1..]
                        .iter()
                        .position(fits)
                        .map(|i| at + 1 + i),
                    _ => None,
                }
                .ok_or(crate::Error::NoSiblingThere)?;
                parent.children.swap(at, to);
                Ok(Some(*id))
            }
        }
    }
}

fn free_note_mut(map: &mut MindMap, id: u64) -> crate::Result<&mut FreeNote> {
    map.notes
        .iter_mut()
        .find(|n| n.id == id)
        .ok_or(crate::Error::NoteNotFound(id))
}

/// The node whose direct children include `id`.
fn parent_of_mut(node: &mut Node, id: u64) -> Option<&mut Node> {
    if node.children.iter().any(|c| c.id == id) {
        return Some(node);
    }
    node.children
        .iter_mut()
        .find_map(|child| parent_of_mut(child, id))
}

/// Shift a node's descendants by `(dx, dy)`, relative to the node itself.
///
/// Used by [`Action::MoveNode`] so that dragging a branch carries its children
/// with it. The node itself is repositioned by the caller.
fn shift_subtree(node: &mut Node, dx: f64, dy: f64) {
    for child in &mut node.children {
        child.pos.x += dx;
        child.pos.y += dy;
        shift_subtree(child, dx, dy);
    }
}

/// Bounded undo/redo stack over map snapshots.
#[derive(Debug, Clone)]
pub struct History {
    past: Vec<MindMap>,
    present: MindMap,
    future: Vec<MindMap>,
    limit: usize,
}

impl History {
    /// Create a history with the default depth of 200 steps.
    pub fn new(initial: MindMap) -> Self {
        Self {
            past: Vec::new(),
            present: initial,
            future: Vec::new(),
            limit: 200,
        }
    }

    /// Create a history with an explicit depth limit.
    pub fn with_limit(initial: MindMap, limit: usize) -> Self {
        Self {
            past: Vec::new(),
            present: initial,
            future: Vec::new(),
            limit: limit.max(1),
        }
    }

    /// The current map.
    pub fn current(&self) -> &MindMap {
        &self.present
    }

    /// The current map, mutably. Bypasses history — callers that use this must
    /// record a snapshot themselves via [`History::commit`].
    pub fn current_mut(&mut self) -> &mut MindMap {
        &mut self.present
    }

    /// Apply an action, recording it for undo. Returns the affected node id.
    pub fn apply(&mut self, action: Action) -> crate::Result<Option<u64>> {
        let snapshot = self.present.clone();
        let result = action.apply(&mut self.present)?;
        self.past.push(snapshot);
        if self.past.len() > self.limit {
            self.past.remove(0);
        }
        self.future.clear();
        Ok(result)
    }

    /// Record an arbitrary mutation already made through [`History::current_mut`].
    ///
    /// Pass the map as it was *before* the mutation.
    pub fn commit(&mut self, before: MindMap) {
        self.past.push(before);
        if self.past.len() > self.limit {
            self.past.remove(0);
        }
        self.future.clear();
    }

    /// Whether an undo step is available.
    pub fn can_undo(&self) -> bool {
        !self.past.is_empty()
    }

    /// Whether a redo step is available.
    pub fn can_redo(&self) -> bool {
        !self.future.is_empty()
    }

    /// Step back one action.
    pub fn undo(&mut self) -> bool {
        match self.past.pop() {
            Some(prev) => {
                let cur = std::mem::replace(&mut self.present, prev);
                self.future.push(cur);
                true
            }
            None => false,
        }
    }

    /// Step forward one action.
    pub fn redo(&mut self) -> bool {
        match self.future.pop() {
            Some(next) => {
                let cur = std::mem::replace(&mut self.present, next);
                self.past.push(cur);
                true
            }
            None => false,
        }
    }

    /// Number of undo steps available.
    pub fn undo_depth(&self) -> usize {
        self.past.len()
    }

    /// Number of redo steps available.
    pub fn redo_depth(&self) -> usize {
        self.future.len()
    }

    /// Replace the map and clear all history.
    pub fn reset(&mut self, map: MindMap) {
        self.past.clear();
        self.future.clear();
        self.present = map;
    }
}

/// Convenience: snapshot-and-mutate for callers holding a `&mut MindMap`.
pub fn edit<F, R>(history: &mut History, f: F) -> R
where
    F: FnOnce(&mut MindMap) -> R,
{
    let before = history.current().clone();
    let out = f(history.current_mut());
    history.commit(before);
    out
}

/// Find a node by id in a map, or `None`.
pub fn lookup(map: &MindMap, id: u64) -> Option<&Node> {
    map.find(id)
}
