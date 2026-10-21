//! The `.nodo` file: Nodo's own format (D27).
//!
//! JSON, pretty-printed, so a map stays readable and diffable without the app. It holds only what Nodo models —
//! topics, notes, branch colours, sides, folds, the map's theme, structure and spacing, and loose notes. Positions
//! and box sizes are not stored: the layout is recomputed on open and the window measures text itself.
//!
//! Every file names its format and version. A reader accepts any version up to [`VERSION`] and ignores fields it does
//! not know, so a newer Nodo can add fields without breaking an older one.

use crate::layout;
use crate::model::{FreeNote, MindMap, Node, NodeSide, Point, ThemeRef};
use serde::{Deserialize, Serialize};

/// The newest version this build writes and the newest it can read.
pub const VERSION: u32 = 1;
/// Value of the `format` field, so a `.nodo` is recognisable at a glance.
const FORMAT: &str = "nodo";

#[derive(Serialize, Deserialize)]
struct File {
    format: String,
    version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    theme: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    structure: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    spacing: Option<String>,
    root: Topic,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    notes: Vec<Loose>,
}

#[derive(Serialize, Deserialize)]
struct Topic {
    id: u64,
    text: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    note: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    color: Option<String>,
    /// Only meaningful for branches of the root; absent means right.
    #[serde(default, skip_serializing_if = "is_right")]
    side: Side,
    #[serde(default, skip_serializing_if = "is_false")]
    folded: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    children: Vec<Topic>,
}

#[derive(Serialize, Deserialize)]
struct Loose {
    id: u64,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    #[serde(default)]
    text: String,
}

#[derive(Serialize, Deserialize, Default, PartialEq, Clone, Copy)]
#[serde(rename_all = "lowercase")]
enum Side {
    Left,
    #[default]
    Right,
}

fn is_right(s: &Side) -> bool {
    *s == Side::Right
}

fn is_false(b: &bool) -> bool {
    !*b
}

/// Write a map as `.nodo` text.
pub fn to_nodo(map: &MindMap) -> crate::Result<String> {
    let file = File {
        format: FORMAT.into(),
        version: VERSION,
        theme: map.theme.style.clone(),
        structure: map.theme.structure.clone(),
        spacing: map.theme.spacing.clone(),
        root: topic(&map.root),
        notes: map
            .notes
            .iter()
            .map(|n| Loose {
                id: n.id,
                x: n.pos.x,
                y: n.pos.y,
                width: n.width,
                height: n.height,
                text: n.text.clone(),
            })
            .collect(),
    };
    let mut text =
        serde_json::to_string_pretty(&file).map_err(|e| crate::Error::Format(e.to_string()))?;
    text.push('\n');
    Ok(text)
}

fn topic(n: &Node) -> Topic {
    Topic {
        id: n.id,
        text: n.text.clone(),
        note: n.note.clone(),
        color: n.color.clone(),
        side: match n.side {
            NodeSide::Left => Side::Left,
            _ => Side::Right,
        },
        folded: n.folded,
        children: n.children.iter().map(topic).collect(),
    }
}

/// Read `.nodo` text. Unknown theme, structure or spacing names count as the default, like the old format did.
pub fn from_nodo(text: &str) -> crate::Result<MindMap> {
    let file: File = serde_json::from_str(text).map_err(|e| crate::Error::Format(e.to_string()))?;
    if file.format != FORMAT {
        return Err(crate::Error::Format(format!(
            "not a Nodo map (format {:?})",
            file.format
        )));
    }
    if file.version > VERSION {
        return Err(crate::Error::Format(format!(
            "this map was saved by a newer Nodo (format version {}); update Nodo to open it",
            file.version
        )));
    }
    let structure = file
        .structure
        .filter(|s| crate::STRUCTURES.contains(&s.as_str()));
    let kind = layout::minder_layout(structure.as_deref());
    let mut root = node(file.root);
    layout::for_each_mut(&mut root, &mut |n| n.layout = kind);
    let mut map = MindMap::new("");
    map.root = root;
    map.theme = ThemeRef {
        style: file
            .theme
            .filter(|s| crate::MAP_STYLES.contains(&s.as_str())),
        structure,
        spacing: file
            .spacing
            .filter(|s| crate::SPACINGS.contains(&s.as_str())),
        ..ThemeRef::default()
    };
    map.notes = file
        .notes
        .into_iter()
        .map(|n| FreeNote {
            id: n.id,
            pos: Point { x: n.x, y: n.y },
            width: n.width,
            height: n.height,
            text: n.text,
        })
        .collect();
    Ok(map)
}

fn node(t: Topic) -> Node {
    let mut n = Node::new(t.id, t.text);
    n.note = t.note;
    n.color = t.color;
    n.side = match t.side {
        Side::Left => NodeSide::Left,
        Side::Right => NodeSide::Right,
    };
    n.folded = t.folded;
    n.children = t.children.into_iter().map(node).collect();
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> MindMap {
        let mut map = MindMap::new("Lançamento");
        let a = map.add_child(0, "Descoberta").unwrap_or_default();
        let b = map.add_child(0, "Divulgação").unwrap_or_default();
        map.add_child(a, "Entrevistas \"com\" usuários & <tags>")
            .unwrap_or_default();
        if let Some(n) = map.find_mut(a) {
            n.note = "- [ ] Roteiro\n- [x] Convites\n\n**Prazo**: sexta".into();
            n.color = Some("#6eb0f7".into());
            n.folded = true;
        }
        if let Some(n) = map.find_mut(b) {
            n.side = NodeSide::Left;
        }
        map.theme.style = Some("vibrante".into());
        map.theme.structure = Some("logico".into());
        map.theme.spacing = Some("largo".into());
        map.notes.push(FreeNote {
            id: 1,
            pos: Point { x: -40.5, y: 120.0 },
            width: 300.0,
            height: 200.0,
            text: "Ideias soltas ✨".into(),
        });
        map
    }

    #[test]
    fn round_trips_everything_nodo_models() {
        let map = sample();
        let back = from_nodo(&to_nodo(&map).unwrap()).unwrap();
        assert_eq!(back.theme.style, map.theme.style);
        assert_eq!(back.theme.structure, map.theme.structure);
        assert_eq!(back.theme.spacing, map.theme.spacing);
        assert_eq!(back.notes, map.notes);
        let pairs: Vec<_> = map.nodes().zip(back.nodes()).collect();
        assert_eq!(pairs.len(), map.node_count());
        for (a, b) in pairs {
            assert_eq!(
                (a.id, &a.text, &a.note, &a.color, a.side, a.folded),
                (b.id, &b.text, &b.note, &b.color, b.side, b.folded)
            );
        }
    }

    #[test]
    fn is_readable_json_without_defaults() {
        let text = to_nodo(&MindMap::new("Só a raiz")).unwrap();
        let v: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(v["format"], "nodo");
        assert_eq!(v["version"], VERSION);
        assert_eq!(
            v["root"],
            serde_json::json!({ "id": 0, "text": "Só a raiz" })
        );
        assert!(v.get("notes").is_none() && v.get("theme").is_none());
    }

    #[test]
    fn refuses_other_files_and_newer_versions() {
        assert!(matches!(
            from_nodo("<minder/>"),
            Err(crate::Error::Format(_))
        ));
        assert!(from_nodo(r#"{"format":"outra","version":1,"root":{"id":0,"text":"x"}}"#).is_err());
        let newer = r#"{"format":"nodo","version":99,"root":{"id":0,"text":"x"}}"#;
        assert!(matches!(from_nodo(newer), Err(crate::Error::Format(m)) if m.contains("newer")));
    }

    #[test]
    fn ignores_unknown_fields_and_names() {
        let text = r#"{"format":"nodo","version":1,"theme":"inventado","futuro":true,
            "root":{"id":0,"text":"R","emoji":"🌱","children":[{"id":1,"text":"A","side":"left"}]}}"#;
        let map = from_nodo(text).unwrap();
        assert_eq!(map.theme.style, None);
        assert_eq!(map.root.children[0].side, NodeSide::Left);
    }
}
