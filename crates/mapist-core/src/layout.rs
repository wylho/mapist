//! Tree layout.
//!
//! Computes node positions for a subtree. The strategy mirrors Minder's
//! observed behaviour: a parent is centred on the vertical extent of its
//! children, children are spaced by a margin, and the chosen [`LayoutKind`]
//! decides the growth axis.
//!
//! Sizes are estimates from text metrics rather than real font measurement,
//! because the core must stay UI-free. The renderer may refine `width` and
//! `height` afterwards and call [`layout`] again.

use crate::model::*;

/// Approximate per-character width for the default 11pt sans font.
const CHAR_WIDTH: f64 = 7.4;
/// Minimum node width.
const MIN_WIDTH: f64 = 63.0;
/// Minimum node height.
const MIN_HEIGHT: f64 = 46.0;
/// Horizontal gap between a parent and its children.
const H_GAP: f64 = 60.0;
/// Vertical gap between sibling subtrees.
const V_GAP: f64 = 20.0;

/// Estimate a node's box size from its text.
pub fn measure(node: &mut Node) {
    let lines: Vec<&str> = if node.text.is_empty() {
        vec![""]
    } else {
        node.text.lines().collect()
    };
    let longest = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
    let text_w = longest as f64 * CHAR_WIDTH;
    node.width = (text_w + node.style.node_padding * 2.0 + 14.0).max(MIN_WIDTH);
    node.height = (lines.len() as f64 * 18.0 + node.style.node_padding * 2.0).max(MIN_HEIGHT);
}

/// Lay out a whole map, starting from the root.
///
/// `measure` runs first so every box is sized before positions are assigned.
pub fn layout(map: &mut MindMap) {
    for_each_mut(&mut map.root, &mut measure);
    let mut root = std::mem::replace(&mut map.root, Node::new(0, ""));
    place(&mut root, 0.0, 0.0);
    map.root = root;
}

/// Position `node` so its top-left is near (`x`, `y`), then recurse.
///
/// Returns the total extent consumed along the growth axis.
fn place(node: &mut Node, x: f64, y: f64) -> f64 {
    node.pos = Point { x, y };

    if node.children.is_empty() || node.folded {
        return match axis(node.layout) {
            Axis::Horizontal => node.height,
            Axis::Vertical => node.width,
        };
    }

    match axis(node.layout) {
        Axis::Horizontal => {
            let dir = match node.side {
                NodeSide::Left => -1.0,
                _ => 1.0,
            };
            let child_x = x + dir * (node.width + H_GAP);
            let mut cursor = y;
            let start = cursor;
            for child in node.children.iter_mut() {
                let h = place(child, child_x, cursor);
                cursor += h + V_GAP;
            }
            let total = (cursor - V_GAP - start).max(node.height);
            // Centre this node on its children.
            node.pos.y = start + (total - node.height) / 2.0;
            total
        }
        Axis::Vertical => {
            let dir = match node.layout {
                LayoutKind::Up => -1.0,
                _ => 1.0,
            };
            let child_y = y + dir * (node.height + H_GAP);
            let mut cursor = x;
            let start = cursor;
            for child in node.children.iter_mut() {
                let w = place(child, cursor, child_y);
                cursor += w + V_GAP;
            }
            let total = (cursor - V_GAP - start).max(node.width);
            node.pos.x = start + (total - node.width) / 2.0;
            total
        }
    }
}

enum Axis {
    Horizontal,
    Vertical,
}

fn axis(kind: LayoutKind) -> Axis {
    match kind {
        LayoutKind::Vertical | LayoutKind::Up | LayoutKind::Down => Axis::Vertical,
        _ => Axis::Horizontal,
    }
}

/// Apply `f` to this node and every descendant, depth-first, without
/// allocating a vector of aliasing references.
pub fn for_each_mut<F: FnMut(&mut Node)>(node: &mut Node, f: &mut F) {
    f(node);
    for child in &mut node.children {
        for_each_mut(child, f);
    }
}
