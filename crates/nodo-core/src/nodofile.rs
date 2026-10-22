//! The `.nodo` file: Nodo's own format (D27).
//!
//! JSON, pretty-printed, so a map stays readable without the app: the `mapa.json` inside the `.nodo` package
//! (`package.rs`), or the whole file in maps saved before attachments (still read). It holds only what Nodo models —
//! topics, notes, branch colours, sides, folds, the map's theme, structure and spacing, and loose notes. Positions
//! and box sizes are not stored: the layout is recomputed on open and the window measures text itself.
//!
//! Every file names its format and version. A reader accepts any version up to [`VERSION`] and ignores fields it does
//! not know, so a newer Nodo can add fields without breaking an older one.

use crate::layout;
use crate::model::{CardFields, FreeNote, MindMap, Node, NodeSide, Point, ThemeRef};
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
    /// The files the notes use; the bytes are beside this text in the package.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    attachments: Vec<crate::attachments::Attachment>,
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
    /// Tags, cover emoji, reminder and status, flattened into the topic (absent when unset).
    #[serde(flatten)]
    card: CardFields,
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
    #[serde(default, skip_serializing_if = "String::is_empty")]
    title: String,
    #[serde(default)]
    text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    color: Option<String>,
    /// Tags, cover emoji, reminder and status, as in a topic.
    #[serde(flatten)]
    card: CardFields,
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
    to_nodo_with(map, &[])
}

/// The map's text with the list of its attachments.
pub fn to_nodo_with(
    map: &MindMap,
    attachments: &[crate::attachments::Attachment],
) -> crate::Result<String> {
    let file = File {
        attachments: attachments.to_vec(),
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
                title: n.title.clone(),
                text: n.text.clone(),
                color: n.color.clone(),
                card: n.card.clone(),
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
        card: n.card.clone(),
        children: n.children.iter().map(topic).collect(),
    }
}

/// Read `.nodo` text. Unknown theme, structure or spacing names count as the default, like the old format did.
pub fn from_nodo(text: &str) -> crate::Result<MindMap> {
    from_nodo_with(text).map(|(map, _)| map)
}

/// The map and the list of its attachments.
pub fn from_nodo_with(text: &str) -> crate::Result<(MindMap, Vec<crate::attachments::Attachment>)> {
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
            title: n.title,
            text: n.text,
            color: n
                .color
                .filter(|c| crate::model::NOTE_COLORS.contains(&c.as_str())),
            card: n.card.cleaned(),
        })
        .collect();
    Ok((map, file.attachments))
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
    n.card = t.card.cleaned();
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
            n.card = CardFields {
                tags: vec!["pesquisa".into(), "q3".into()],
                emoji: Some("🔍".into()),
                reminder_at: Some(1_790_000_000_000),
                reminder_repeat: Some("week".into()),
                reminder_doing: true,
                ..CardFields::default()
            };
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
            title: "Rascunho".into(),
            color: Some("sky".into()),
            card: CardFields {
                tags: vec!["ideia".into()],
                status_on: true,
                reminder_doing: true,
                ..Default::default()
            },
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
                (a.id, &a.text, &a.note, &a.color, a.side, a.folded, &a.card),
                (b.id, &b.text, &b.note, &b.color, b.side, b.folded, &b.card)
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
    fn card_fields_sit_in_the_topic_and_are_cleaned_on_read() {
        let text = to_nodo(&sample()).unwrap();
        let v: serde_json::Value = serde_json::from_str(&text).unwrap();
        let a = &v["root"]["children"][0];
        assert_eq!(a["tags"], serde_json::json!(["pesquisa", "q3"]));
        assert_eq!(a["emoji"], "🔍");
        assert_eq!(a["reminder_repeat"], "week");
        assert!(a.get("reminder_done").is_none() && a.get("status_on").is_none());
        let messy = r##"{"format":"nodo","version":1,"root":{"id":0,"text":"R",
            "tags":["#Q3"," q3 ",""],"emoji":"ab","reminder_repeat":"week","reminder_done":true}}"##;
        let card = from_nodo(messy).unwrap().root.card;
        assert_eq!(card.tags, vec!["q3".to_string()]);
        assert_eq!(card.emoji, None);
        assert_eq!(card.reminder_repeat, None, "no repeat without a reminder");
        assert!(
            !card.reminder_done,
            "no status without a reminder or status_on"
        );
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
            "root":{"id":0,"text":"R","pino":"🌱","children":[{"id":1,"text":"A","side":"left"}]}}"#;
        let map = from_nodo(text).unwrap();
        assert_eq!(map.theme.style, None);
        assert_eq!(map.root.children[0].side, NodeSide::Left);
    }
}
