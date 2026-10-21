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

/// How the map grows, from its structure (`nodo-structure`).
#[derive(Clone, Copy, PartialEq)]
enum Mode {
    /// Each node's own Minder layout: branches on both sides of the root.
    PerNode,
    /// Everything to the right of its parent (logic chart).
    Right,
    /// Everything below its parent (org chart).
    Down,
    /// An indented list, in reading order.
    Tree,
}

/// Gaps for one layout pass, scaled by the map's spacing.
#[derive(Clone, Copy)]
struct Params {
    mode: Mode,
    h_gap: f64,
    v_gap: f64,
}

impl Params {
    fn of(map: &MindMap) -> Self {
        let mode = match map.theme.structure.as_deref() {
            Some("logico") => Mode::Right,
            Some("organograma") => Mode::Down,
            Some("arvore") => Mode::Tree,
            _ => Mode::PerNode,
        };
        let k = match map.theme.spacing.as_deref() {
            Some("compacto") => 0.6,
            Some("largo") => 1.6,
            _ => 1.0,
        };
        Params {
            mode,
            h_gap: H_GAP * k,
            v_gap: V_GAP * k,
        }
    }
}

/// The Minder layout closest to a structure, written on every node so Minder shows a similar shape.
pub fn minder_layout(structure: Option<&str>) -> LayoutKind {
    match structure {
        Some("logico") | Some("arvore") => LayoutKind::Right,
        Some("organograma") => LayoutKind::Down,
        _ => LayoutKind::Horizontal,
    }
}

/// Assign positions using the box sizes already on each node.
///
/// Use this when sizes came from somewhere better than [`measure`] — the
/// renderer measures text with real font metrics and reports back, so the core
/// must not overwrite those sizes with its own estimate.
pub fn reposition(map: &mut MindMap) {
    let p = Params::of(map);
    let mut root = std::mem::replace(&mut map.root, Node::new(0, ""));
    if p.mode == Mode::Tree {
        place_tree(&mut root, 0.0, 0.0, &p);
    } else {
        let ax = axis(&root, &p);
        let top = -extent(&root, &ax, &p) / 2.0;
        place(&mut root, 0.0, top, &p);
    }
    map.root = root;
}

/// Indented list: each node on its own row, children one step to the right. Returns where the next row starts.
fn place_tree(node: &mut Node, x: f64, y: f64, p: &Params) -> f64 {
    node.pos = Point { x, y };
    let mut next = y + node.height + p.v_gap * 0.5;
    if node.folded {
        return next;
    }
    let indent = (p.h_gap * 0.6).max(18.0);
    for child in &mut node.children {
        next = place_tree(child, x + indent, next, p);
    }
    next
}

/// Extent of a subtree across the growth axis: how much room it needs, so
/// siblings never overlap. A folded node only needs its own box.
fn extent(node: &Node, axis: &Axis, p: &Params) -> f64 {
    let own = match axis {
        Axis::Horizontal => node.height,
        Axis::Vertical => node.width,
    };
    if node.children.is_empty() || node.folded {
        return own;
    }
    let (before, after) = split(node, p);
    let band = |group: &[&Node]| -> f64 {
        if group.is_empty() {
            return 0.0;
        }
        group.iter().map(|c| extent(c, axis, p)).sum::<f64>() + p.v_gap * (group.len() - 1) as f64
    };
    own.max(band(&before)).max(band(&after))
}

/// Does this child go before its parent (left, or up) rather than after it?
fn goes_before(parent: &Node, child: &Node, p: &Params) -> bool {
    match p.mode {
        Mode::Right | Mode::Down | Mode::Tree => false,
        Mode::PerNode => match axis(parent, p) {
            Axis::Horizontal => child.side == NodeSide::Left,
            Axis::Vertical => parent.layout == LayoutKind::Up,
        },
    }
}

/// Children split by the side they grow towards: `Left` (or `Up`) first.
///
/// Horizontal layouts use each child's own side, so a root can carry branches
/// on both sides as in Minder. Vertical layouts grow one way only.
fn split<'a>(node: &'a Node, p: &Params) -> (Vec<&'a Node>, Vec<&'a Node>) {
    node.children.iter().partition(|c| goes_before(node, c, p))
}

/// Position `node` inside a band that starts at `top` (across the growth
/// axis) with its near edge at `x` (along it), then place its children.
fn place(node: &mut Node, x: f64, top: f64, p: &Params) {
    let ax = axis(node, p);
    let ext = extent(node, &ax, p);
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
        .map(|c| goes_before(node, c, p))
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
            group.iter().map(|c| extent(c, &ax, p)).sum::<f64>()
                + p.v_gap * (group.len() - 1) as f64
        };
        let mut cursor = top + (ext - total) / 2.0;
        for (child, _) in node
            .children
            .iter_mut()
            .zip(&before)
            .filter(|(_, b)| **b == want_before)
        {
            let child_ext = extent(child, &ax, p);
            let near = match (&ax, want_before) {
                (Axis::Horizontal, true) => px - p.h_gap - child.width,
                (Axis::Horizontal, false) => px + w + p.h_gap,
                (Axis::Vertical, true) => py - p.h_gap - child.height,
                (Axis::Vertical, false) => py + h + p.h_gap,
            };
            place(child, near, cursor, p);
            cursor += child_ext + p.v_gap;
        }
    }
}

/// The direction a subtree grows in.
enum Axis {
    Horizontal,
    Vertical,
}

fn axis(node: &Node, p: &Params) -> Axis {
    match p.mode {
        Mode::Down => Axis::Vertical,
        Mode::Right | Mode::Tree => Axis::Horizontal,
        Mode::PerNode => match node.layout {
            LayoutKind::Vertical | LayoutKind::Up | LayoutKind::Down => Axis::Vertical,
            _ => Axis::Horizontal,
        },
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
