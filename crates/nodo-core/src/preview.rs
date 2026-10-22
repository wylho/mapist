//! The card of a topic as the map shows it: the same preview as an Ideario card (`atrio::projection`, the only
//! preview rule of both apps), made from the note (Markdown → the editor's document → projection), with the cover
//! photos' sizes for the right proportion and the counts the pills show.

use serde::Serialize;

use crate::attachments::Attachments;

/// A cover photo: its hash and size (so the card keeps the photo's proportion before it loads).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Cover {
    pub hash: String,
    pub width: Option<i64>,
    pub height: Option<i64>,
}

/// What the card shows besides the title and the fields.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct CardView {
    /// Preview blocks, in the format of Atrio's `PreviewBlock` (`kind`, `text`…).
    pub preview: Vec<atrio::projection::Block>,
    pub cover: Vec<Cover>,
    /// Photos in the note (the cover shows up to 4; the card shows "+N" for the rest).
    pub image_count: usize,
    /// Attachments that are not photos.
    pub file_count: usize,
    /// `#words` in the note: pills, like the tags chosen by hand.
    pub hash_tags: Vec<String>,
}

/// The card of a note. An empty note has no card view.
pub fn card_view(note: &str, atts: &Attachments) -> Option<CardView> {
    if note.trim().is_empty() {
        return None;
    }
    let doc = atrio::markdown::from_markdown(note);
    let p = atrio::projection::project(&doc, &|h| {
        atts.get(h).map(|a| (a.name.clone(), a.kind.clone()))
    });
    let cover = p
        .cover
        .iter()
        .map(|h| {
            let a = atts.get(h);
            Cover {
                hash: h.clone(),
                width: a.and_then(|a| a.width),
                height: a.and_then(|a| a.height),
            }
        })
        .collect();
    Some(CardView {
        preview: p.preview,
        cover,
        image_count: p.images.len(),
        file_count: p.files.len(),
        hash_tags: p.hash_tags,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_card_is_the_ideario_card() {
        let mut atts = Attachments::default();
        let jpg = atrio::media::tests::camera_jpeg(800, 600, 1);
        let photo = atts.add("praia.jpg", jpg, 1);
        let pdf = atts.add("roteiro.pdf", b"%PDF-1.4 x".to_vec(), 1);
        let note = format!(
            "![praia](ideario://att/{})\n\nLevar #protetor e ver o roteiro.\n\n[📎 roteiro.pdf](ideario://att/{})\n\n- [ ] Reservar hotel\n- [x] Comprar passagem",
            photo.hash, pdf.hash
        );
        let v = card_view(&note, &atts).unwrap();
        assert_eq!(
            v.cover,
            vec![Cover {
                hash: photo.hash.clone(),
                width: Some(800),
                height: Some(600)
            }]
        );
        assert_eq!((v.image_count, v.file_count), (1, 1));
        assert_eq!(v.hash_tags, vec!["protetor"]);
        let json = serde_json::to_value(&v.preview).unwrap();
        let kinds: Vec<&str> = json
            .as_array()
            .unwrap()
            .iter()
            .map(|b| b["kind"].as_str().unwrap())
            .collect();
        assert_eq!(kinds, ["text", "file", "task", "task"]);
        assert_eq!(json[1]["fileKind"], "pdf");
        assert!(card_view("  \n", &atts).is_none());
    }
}
