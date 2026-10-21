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

use crate::model::{MindMap, Node};

/// A user-visible edit.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// Add a child under `parent` with the given text.
    AddChild { parent: u64, text: String },
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
                node.pos.x = *x;
                node.pos.y = *y;
                Ok(Some(*id))
            }
            Action::SetColor { id, color } => {
                let node = map.find_mut(*id).ok_or(crate::Error::NodeNotFound(*id))?;
                node.color = color.clone();
                Ok(Some(*id))
            }
        }
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
