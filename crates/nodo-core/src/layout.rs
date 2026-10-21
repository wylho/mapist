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
    reposition(map);
}

/// Assign positions using the box sizes already on each node.
///
/// Use this when sizes came from somewhere better than [`measure`] — the
/// renderer measures text with real font metrics and reports back, so the core
/// must not overwrite those sizes with its own estimate.
pub fn reposition(map: &mut MindMap) {
    let mut root = std::mem::replace(&mut map.root, Node::new(0, ""));
    let top = match axis(root.layout) {
        Axis::Horizontal => -extent(&root, &Axis::Horizontal) / 2.0,
        Axis::Vertical => -extent(&root, &Axis::Vertical) / 2.0,
    };
    place(&mut root, 0.0, top);
    map.root = root;
}

/// Extent of a subtree across the growth axis: how much room it needs, so
/// siblings never overlap. A folded node only needs its own box.
fn extent(node: &Node, axis: &Axis) -> f64 {
    let own = match axis {
        Axis::Horizontal => node.height,
        Axis::Vertical => node.width,
    };
    if node.children.is_empty() || node.folded {
        return own;
    }
    let (before, after) = split(node);
    let band = |group: &[&Node]| -> f64 {
        if group.is_empty() {
            return 0.0;
        }
        group.iter().map(|c| extent(c, axis)).sum::<f64>() + V_GAP * (group.len() - 1) as f64
    };
    own.max(band(&before)).max(band(&after))
}

/// Children split by the side they grow towards: `Left` (or `Up`) first.
///
/// Horizontal layouts use each child's own side, so a root can carry branches
/// on both sides as in Minder. Vertical layouts grow one way only.
fn split(node: &Node) -> (Vec<&Node>, Vec<&Node>) {
    match axis(node.layout) {
        Axis::Horizontal => node.children.iter().partition(|c| c.side == NodeSide::Left),
        Axis::Vertical => {
            if node.layout == LayoutKind::Up {
                (node.children.iter().collect(), Vec::new())
            } else {
                (Vec::new(), node.children.iter().collect())
            }
        }
    }
}

/// Position `node` inside a band that starts at `top` (across the growth
/// axis) with its near edge at `x` (along it), then place its children.
fn place(node: &mut Node, x: f64, top: f64) {
    let ax = axis(node.layout);
    let ext = extent(node, &ax);
    match ax {
        Axis::Horizontal => {
            node.pos = Point {
                x,
                y: top + (ext - node.height) / 2.0,
            }
        }
        Axis::Vertical => {
            node.pos = Point {
                x: top + (ext - node.width) / 2.0,
                y: x,
            }
        }
    }
    if node.children.is_empty() || node.folded {
        return;
    }

    // Each side is a column of subtrees centred on the parent.
    let (w, h) = (node.width, node.height);
    let (px, py) = (node.pos.x, node.pos.y);
    let before: Vec<bool> = node
        .children
        .iter()
        .map(|c| match ax {
            Axis::Horizontal => c.side == NodeSide::Left,
            Axis::Vertical => node.layout == LayoutKind::Up,
        })
        .collect();
    for want_before in [true, false] {
        let total: f64 = {
            let group: Vec<&Node> = node
                .children
                .iter()
                .zip(&before)
                .filter(|(_, b)| **b == want_before)
                .map(|(c, _)| c)
                .collect();
            if group.is_empty() {
                continue;
            }
            group.iter().map(|c| extent(c, &ax)).sum::<f64>() + V_GAP * (group.len() - 1) as f64
        };
        let mut cursor = top + (ext - total) / 2.0;
        for (child, _) in node
            .children
            .iter_mut()
            .zip(&before)
            .filter(|(_, b)| **b == want_before)
        {
            let child_ext = extent(child, &ax);
            let near = match (&ax, want_before) {
                (Axis::Horizontal, true) => px - H_GAP - child.width,
                (Axis::Horizontal, false) => px + w + H_GAP,
                (Axis::Vertical, true) => py - H_GAP - child.height,
                (Axis::Vertical, false) => py + h + H_GAP,
            };
            place(child, near, cursor);
            cursor += child_ext + V_GAP;
        }
    }
}

/// The direction a subtree grows in.
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
