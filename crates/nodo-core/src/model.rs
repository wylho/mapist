//! Core mind-map data model.
//!
//! Field names and semantics mirror the `.minder` XML format observed in
//! Minder 2.0.9 (`build.minder`, `<minder version="1.15.0">`) so that import
//! is a mechanical mapping and no behaviour is guessed.

use serde::{Deserialize, Serialize};

/// Which side of the root a node hangs from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum NodeSide {
    #[default]
    Right,
    Left,
    Center,
}

impl NodeSide {
    pub fn as_str(self) -> &'static str {
        match self {
            NodeSide::Right => "right",
            NodeSide::Left => "left",
            NodeSide::Center => "center",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "left" => NodeSide::Left,
            "center" => NodeSide::Center,
            _ => NodeSide::Right,
        }
    }
}

/// Tree layout applied to a node's subtree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LayoutKind {
    #[default]
    Horizontal,
    Vertical,
    Up,
    Down,
    Left,
    Right,
    Manual,
}

impl LayoutKind {
    pub fn as_str(self) -> &'static str {
        match self {
            LayoutKind::Horizontal => "Horizontal",
            LayoutKind::Vertical => "Vertical",
            LayoutKind::Up => "Up",
            LayoutKind::Down => "Down",
            LayoutKind::Left => "Left",
            LayoutKind::Right => "Right",
            LayoutKind::Manual => "Manual",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "Vertical" => LayoutKind::Vertical,
            "Up" => LayoutKind::Up,
            "Down" => LayoutKind::Down,
            "Left" => LayoutKind::Left,
            "Right" => LayoutKind::Right,
            "Manual" => LayoutKind::Manual,
            _ => LayoutKind::Horizontal,
        }
    }
}

/// Stroke style for links and connections.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum DashStyle {
    #[default]
    Solid,
    Dotted,
    Dashed,
}

impl DashStyle {
    pub fn as_str(self) -> &'static str {
        match self {
            DashStyle::Solid => "solid",
            DashStyle::Dotted => "dotted",
            DashStyle::Dashed => "dashed",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "dotted" => DashStyle::Dotted,
            "dashed" => DashStyle::Dashed,
            _ => DashStyle::Solid,
        }
    }
}

/// Arrow head placement on a link.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ArrowStyle {
    None,
    From,
    To,
    #[default]
    FromTo,
}

impl ArrowStyle {
    pub fn as_str(self) -> &'static str {
        match self {
            ArrowStyle::None => "none",
            ArrowStyle::From => "from",
            ArrowStyle::To => "to",
            ArrowStyle::FromTo => "fromto",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "none" => ArrowStyle::None,
            "from" => ArrowStyle::From,
            "to" => ArrowStyle::To,
            _ => ArrowStyle::FromTo,
        }
    }
}

/// Node border shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum BorderStyle {
    #[default]
    Squared,
    Rounded,
    Oval,
    Underline,
}

impl BorderStyle {
    pub fn as_str(self) -> &'static str {
        match self {
            BorderStyle::Squared => "squared",
            BorderStyle::Rounded => "rounded",
            BorderStyle::Oval => "oval",
            BorderStyle::Underline => "underline",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "rounded" => BorderStyle::Rounded,
            "oval" => BorderStyle::Oval,
            "underline" => BorderStyle::Underline,
            _ => BorderStyle::Squared,
        }
    }
}

/// Per-level visual style.
///
/// Defaults below are the values Minder writes at every level for a fresh map,
/// taken verbatim from the observed `<style>` element.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Style {
    pub branch_margin: f64,
    pub branch_radius: f64,
    pub link_type: BorderStyle,
    pub link_width: f64,
    pub link_arrow: bool,
    pub link_dash: DashStyle,
    pub node_border: BorderStyle,
    pub node_width: f64,
    pub node_border_width: f64,
    pub node_fill: bool,
    pub node_margin: f64,
    pub node_padding: f64,
    pub node_font: String,
    pub node_markup: bool,
    pub connection_dash: DashStyle,
    pub connection_line_width: f64,
    pub connection_arrow: ArrowStyle,
    pub connection_padding: f64,
    pub connection_font: String,
    pub connection_text_width: f64,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            branch_margin: 100.0,
            branch_radius: 25.0,
            link_type: BorderStyle::Rounded,
            link_width: 4.0,
            link_arrow: false,
            link_dash: DashStyle::Solid,
            node_border: BorderStyle::Squared,
            node_width: 500.0,
            node_border_width: 4.0,
            node_fill: false,
            node_margin: 10.0,
            node_padding: 10.0,
            node_font: "Sans 11".to_string(),
            node_markup: true,
            connection_dash: DashStyle::Dotted,
            connection_line_width: 2.0,
            connection_arrow: ArrowStyle::FromTo,
            connection_padding: 3.0,
            connection_font: "Sans 10".to_string(),
            connection_text_width: 100.0,
        }
    }
}

/// A point in map space.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

/// A node in the mind map.
///
/// `id` is a stable integer assigned on creation, matching the observed
/// `id="0"` attribute. Children are nested, mirroring the XML tree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub id: u64,
    pub pos: Point,
    pub width: f64,
    pub height: f64,
    pub side: NodeSide,
    pub folded: bool,
    pub tree_size: f64,
    pub layout: LayoutKind,
    pub grouped: bool,
    /// Optional branch colour as `#rrggbb`; absent means inherit.
    pub color: Option<String>,
    /// True when the colour is applied at the root only.
    pub color_root: bool,
    pub style: Style,
    pub text: String,
    pub note: String,
    pub children: Vec<Node>,
}

impl Node {
    /// Create a leaf node with Minder's observed defaults.
    pub fn new(id: u64, text: impl Into<String>) -> Self {
        Self {
            id,
            pos: Point::default(),
            width: 63.0,
            height: 46.0,
            side: NodeSide::Right,
            folded: false,
            tree_size: 46.0,
            layout: LayoutKind::Horizontal,
            grouped: false,
            color: None,
            color_root: false,
            style: Style::default(),
            text: text.into(),
            note: String::new(),
            children: Vec::new(),
        }
    }

    /// Depth-first iterator over this node and all descendants.
    pub fn walk(&self) -> impl Iterator<Item = &Node> {
        Walk { stack: vec![self] }
    }

    /// Total number of nodes in this subtree, including self.
    pub fn subtree_len(&self) -> usize {
        self.walk().count()
    }

    /// Find a node by id anywhere in this subtree.
    pub fn find(&self, id: u64) -> Option<&Node> {
        self.walk().find(|n| n.id == id)
    }

    /// Find a node by id mutably anywhere in this subtree.
    pub fn find_mut(&mut self, id: u64) -> Option<&mut Node> {
        if self.id == id {
            return Some(self);
        }
        for child in &mut self.children {
            if let Some(found) = child.find_mut(id) {
                return Some(found);
            }
        }
        None
    }

    /// Append a child and return its assigned id.
    pub fn push_child(&mut self, mut child: Node) -> u64 {
        let id = child.id;
        child.side = self.side;
        self.children.push(child);
        id
    }

    /// Remove a child by id, returning it if present.
    pub fn remove_child(&mut self, id: u64) -> Option<Node> {
        if let Some(idx) = self.children.iter().position(|c| c.id == id) {
            return Some(self.children.remove(idx));
        }
        for child in &mut self.children {
            if let Some(found) = child.remove_child(id) {
                return Some(found);
            }
        }
        None
    }
}

struct Walk<'a> {
    stack: Vec<&'a Node>,
}

impl<'a> Iterator for Walk<'a> {
    type Item = &'a Node;

    fn next(&mut self) -> Option<Self::Item> {
        let node = self.stack.pop()?;
        // Push in reverse so children come out in document order.
        for child in node.children.iter().rev() {
            self.stack.push(child);
        }
        Some(node)
    }
}

/// A user-drawn connection between two nodes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Connection {
    pub id: u64,
    pub from: u64,
    pub to: u64,
    pub text: String,
    pub note: String,
    pub style: Style,
}

/// Viewport transform of the drawing area.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DrawArea {
    pub x: f64,
    pub y: f64,
    pub scale: f64,
}

impl Default for DrawArea {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            scale: 1.0,
        }
    }
}

/// Theme selection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThemeRef {
    pub name: String,
    pub label: String,
    pub index: u32,
    /// Nodo's map theme (one of [`crate::MAP_STYLES`]), kept in an extra
    /// `nodo-style` attribute that Minder ignores. `None` is the default look.
    pub style: Option<String>,
}

impl Default for ThemeRef {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            label: "Light".to_string(),
            index: 1,
            style: None,
        }
    }
}

/// A complete mind map.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MindMap {
    pub version: String,
    pub theme: ThemeRef,
    pub styles: Vec<Style>,
    pub draw_area: DrawArea,
    pub root: Node,
    pub connections: Vec<Connection>,
}

impl MindMap {
    /// A new single-root map with Minder's default style ladder.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            version: crate::FORMAT_VERSION.to_string(),
            theme: ThemeRef::default(),
            styles: vec![Style::default()],
            draw_area: DrawArea::default(),
            root: Node::new(0, title),
            connections: Vec::new(),
        }
    }

    /// Iterate every node in the map.
    pub fn nodes(&self) -> impl Iterator<Item = &Node> {
        self.root.walk()
    }

    /// Number of nodes in the map.
    pub fn node_count(&self) -> usize {
        self.root.subtree_len()
    }

    /// Highest node id in use, or 0 for an empty map.
    pub fn max_id(&self) -> u64 {
        self.nodes().map(|n| n.id).max().unwrap_or(0)
    }

    /// Next unused node id.
    pub fn next_id(&self) -> u64 {
        self.max_id() + 1
    }

    /// Find a node by id.
    pub fn find(&self, id: u64) -> Option<&Node> {
        self.root.find(id)
    }

    /// Find a node by id mutably.
    pub fn find_mut(&mut self, id: u64) -> Option<&mut Node> {
        self.root.find_mut(id)
    }

    /// Depth of a node, root being 0. Returns `None` if absent.
    pub fn depth_of(&self, id: u64) -> Option<usize> {
        fn rec(node: &Node, target: u64, depth: usize) -> Option<usize> {
            if node.id == target {
                return Some(depth);
            }
            for child in &node.children {
                if let Some(d) = rec(child, target, depth + 1) {
                    return Some(d);
                }
            }
            None
        }
        rec(&self.root, id, 0)
    }

    /// Add a child under `parent_id`. Returns the new node id, or `None` if
    /// the parent does not exist.
    pub fn add_child(&mut self, parent_id: u64, text: impl Into<String>) -> Option<u64> {
        let id = self.next_id();
        let parent = self.root.find_mut(parent_id)?;
        parent.push_child(Node::new(id, text));
        Some(id)
    }

    /// Remove a node by id. The root cannot be removed.
    pub fn remove_node(&mut self, id: u64) -> Option<Node> {
        if id == self.root.id {
            return None;
        }
        self.root.remove_child(id)
    }
}
