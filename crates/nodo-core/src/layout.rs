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
    /// A table: the root on top, a row per main topic (its name in the first column), each subtopic in a column
    /// (its own subtopics listed inside the cell).
    Matrix,
    /// A table that is a tree: each topic's cell spans the cells of its subtopics, one row per level.
    TreeTable,
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
            Some("matriz") => Mode::Matrix,
            Some("tabela") => Mode::TreeTable,
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
        Some("organograma") | Some("tabela") => LayoutKind::Down,
        Some("matriz") => LayoutKind::Right,
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
    for_each_mut(&mut root, &mut |n| n.cell = None);
    if p.mode == Mode::Tree {
        place_tree(&mut root, 0.0, 0.0, &p);
    } else if p.mode == Mode::Matrix {
        place_matrix(&mut root, &p);
    } else if p.mode == Mode::TreeTable {
        place_tree_table(&mut root, &p);
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

/// Room around a node inside its table cell.
fn pad(p: &Params) -> f64 {
    (p.v_gap * 0.5).max(6.0)
}

/// Tree table: one row per level (as tall as the tallest topic of that level); a topic's cell is as wide as the
/// cells of its subtopics together (or its own box, if wider), and a topic without visible subtopics fills its
/// column down to the last row.
fn place_tree_table(root: &mut Node, p: &Params) {
    fn rows(n: &Node, depth: usize, out: &mut Vec<f64>, pad: f64) {
        if out.len() <= depth {
            out.push(0.0);
        }
        out[depth] = out[depth].max(n.height + pad * 2.0);
        if !n.folded {
            for c in &n.children {
                rows(c, depth + 1, out, pad);
            }
        }
    }
    fn span(n: &Node, pad: f64) -> f64 {
        let own = n.width + pad * 2.0;
        if n.folded || n.children.is_empty() {
            return own;
        }
        own.max(n.children.iter().map(|c| span(c, pad)).sum())
    }
    fn place(n: &mut Node, x: f64, y: f64, w: f64, depth: usize, rows: &[f64], pad: f64) {
        let leaf = n.folded || n.children.is_empty();
        let h = if leaf {
            rows[depth..].iter().sum()
        } else {
            rows[depth]
        };
        n.cell = Some((x, y, w, h));
        n.pos = Point {
            x: x + pad,
            y: y + pad,
        };
        if leaf {
            return;
        }
        let spans: Vec<f64> = n.children.iter().map(|c| span(c, pad)).collect();
        let extra = (w - spans.iter().sum::<f64>()).max(0.0) / spans.len() as f64;
        let mut cx = x;
        for (c, s) in n.children.iter_mut().zip(spans) {
            place(c, cx, y + rows[depth], s + extra, depth + 1, rows, pad);
            cx += s + extra;
        }
    }
    let pad = pad(p);
    let mut r = Vec::new();
    rows(root, 0, &mut r, pad);
    let w = span(root, pad);
    place(root, 0.0, 0.0, w, 0, &r, pad);
}

/// Matrix: the root as a title row across the table; below it, a row per main topic, with the topic's name in the
/// first column and each of its subtopics in the next columns (the n-th subtopic of every row in the n-th column).
/// Deeper topics are listed inside their cell, indented, as in the tree structure.
fn place_matrix(root: &mut Node, p: &Params) {
    let pad = pad(p);
    let indent = (p.h_gap * 0.4).max(14.0);
    // Size of what goes in a cell: the subtopic and, below it, its own subtopics indented.
    fn content(n: &Node, indent: f64, gap: f64) -> (f64, f64) {
        let (mut w, mut h) = (n.width, n.height);
        if !n.folded {
            for c in &n.children {
                let (cw, ch) = content(c, indent, gap);
                w = w.max(cw + indent);
                h += gap + ch;
            }
        }
        (w, h)
    }
    fn put(n: &mut Node, x: f64, y: f64, indent: f64, gap: f64) -> f64 {
        n.pos = Point { x, y };
        let mut next = y + n.height;
        if !n.folded {
            for c in &mut n.children {
                next = put(c, x + indent, next + gap, indent, gap);
            }
        }
        next
    }
    let gap = (p.v_gap * 0.4).max(4.0);
    let rows: &[Node] = if root.folded { &[] } else { &root.children };
    let cols = rows
        .iter()
        .map(|r| if r.folded { 0 } else { r.children.len() })
        .max()
        .unwrap_or(0);
    let head_w = rows.iter().map(|r| r.width).fold(0.0, f64::max) + pad * 2.0;
    let mut col_w = vec![0.0f64; cols];
    let mut row_h = Vec::with_capacity(rows.len());
    for r in rows {
        let mut h = r.height;
        if !r.folded {
            for (j, c) in r.children.iter().enumerate() {
                let (cw, ch) = content(c, indent, gap);
                col_w[j] = col_w[j].max(cw + pad * 2.0);
                h = h.max(ch);
            }
        }
        row_h.push(h + pad * 2.0);
    }
    let table_w = (head_w + col_w.iter().sum::<f64>()).max(root.width + pad * 2.0);
    // Sobra de largura (título mais largo que a tabela): vai para a última coluna.
    let spare = table_w - head_w - col_w.iter().sum::<f64>();
    if let Some(last) = col_w.last_mut() {
        *last += spare;
    }
    let title_h = root.height + pad * 2.0;
    root.cell = Some((0.0, 0.0, table_w, title_h));
    root.pos = Point { x: pad, y: pad };
    let mut y = title_h;
    let empty = if cols == 0 { table_w } else { head_w };
    for (r, h) in root.children.iter_mut().zip(row_h) {
        r.cell = Some((0.0, y, empty, h));
        r.pos = Point { x: pad, y: y + pad };
        let mut x = head_w;
        if !r.folded {
            for (j, c) in r.children.iter_mut().enumerate() {
                put(c, x + pad, y + pad, indent, gap);
                c.cell = Some((x, y, col_w[j], h));
                x += col_w[j];
            }
        }
        y += h;
    }
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
        Mode::Right | Mode::Down | Mode::Tree | Mode::Matrix | Mode::TreeTable => false,
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
        Mode::Down | Mode::TreeTable => Axis::Vertical,
        Mode::Right | Mode::Tree | Mode::Matrix => Axis::Horizontal,
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
