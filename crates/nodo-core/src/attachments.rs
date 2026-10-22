//! Attachments of a map: photos, audio, video and other files placed in topic notes. A note refers to one by the hash
//! of its content (`ideario://att/<hash>`, the same Markdown the Ideario writes, through `atrio::markdown`). What an
//! attachment is and how it comes in are Atrio's (`atrio::attach`, the same as the Ideario: photos go through the
//! pipeline, WebP with a thumbnail, palette and tone). The bytes live inside the `.nodo` package (`package.rs`) and
//! are read from it only when asked for; the ones added since the last save are kept in memory until the next one.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub use atrio::attach::Attachment;
use atrio::media::Quality;

use crate::model::MindMap;

/// Where the bytes of an attachment are. Cheap to clone, so the app can take it under its lock and read the bytes
/// after releasing it (a big video does not hold the map).
#[derive(Debug, Clone)]
pub enum Data {
    /// Added since the last save.
    Memory(Arc<Vec<u8>>),
    /// Inside a saved package.
    Packed { file: PathBuf, entry: String },
}

/// The attachments of an open map.
#[derive(Debug, Clone, Default)]
pub struct Attachments {
    meta: BTreeMap<String, Attachment>,
    pub(crate) data: BTreeMap<String, Data>,
    /// Thumbnails (WebP) of the photos, by hash.
    pub(crate) thumbs: BTreeMap<String, Data>,
}

impl Attachments {
    /// Bring a file in, the Atrio way (a photo becomes WebP with a thumbnail; the rest stays as it came), and return
    /// its attachment (the same one when this content is already here).
    pub fn add(&mut self, name: &str, bytes: Vec<u8>, now: i64) -> Attachment {
        let p = atrio::attach::prepare(bytes, name, "", Quality::Balanced, now);
        if let Some(a) = self.meta.get(&p.att.hash) {
            return a.clone();
        }
        let hash = p.att.hash.clone();
        if let Some(t) = p.thumb {
            self.thumbs.insert(hash.clone(), Data::Memory(Arc::new(t)));
        }
        self.meta.insert(hash.clone(), p.att.clone());
        self.data.insert(hash, Data::Memory(Arc::new(p.bytes)));
        p.att
    }

    pub fn get(&self, hash: &str) -> Option<&Attachment> {
        self.meta.get(hash)
    }

    /// The attachments with these hashes that are here, in the order asked.
    pub fn list<'a>(&self, hashes: impl IntoIterator<Item = &'a str>) -> Vec<Attachment> {
        hashes
            .into_iter()
            .filter_map(|h| self.meta.get(h).cloned())
            .collect()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Attachment> {
        self.meta.values()
    }

    pub fn is_empty(&self) -> bool {
        self.meta.is_empty()
    }

    /// The content of an attachment, from memory or read from the package.
    pub fn bytes(&self, hash: &str) -> crate::Result<Vec<u8>> {
        match self.data.get(hash) {
            Some(d) => d.read(),
            None => Err(crate::Error::Format(format!(
                "anexo {hash} não está no mapa"
            ))),
        }
    }

    /// Where the bytes of an attachment are, to read them later with [`Data::read`].
    pub fn source(&self, hash: &str) -> Option<Data> {
        self.data.get(hash).cloned()
    }

    /// Where the thumbnail of a photo is.
    pub fn thumb(&self, hash: &str) -> Option<Data> {
        self.thumbs.get(hash).cloned()
    }

    /// Known from the package just read: the bytes stay in `file` until asked for.
    pub(crate) fn packed(
        &mut self,
        a: Attachment,
        file: &Path,
        entry: String,
        thumb: Option<String>,
    ) {
        if let Some(t) = thumb {
            self.thumbs.insert(
                a.hash.clone(),
                Data::Packed {
                    file: file.to_path_buf(),
                    entry: t,
                },
            );
        }
        self.data.insert(
            a.hash.clone(),
            Data::Packed {
                file: file.to_path_buf(),
                entry,
            },
        );
        self.meta.insert(a.hash.clone(), a);
    }
}

impl Data {
    /// The bytes, from memory or from the package.
    pub fn read(&self) -> crate::Result<Vec<u8>> {
        match self {
            Data::Memory(b) => Ok(b.as_ref().clone()),
            Data::Packed { file, entry } => read_entry(file, entry),
        }
    }
}

/// Read one entry of a package.
pub(crate) fn read_entry(file: &Path, entry: &str) -> crate::Result<Vec<u8>> {
    let mut zip = zip::ZipArchive::new(std::fs::File::open(file)?).map_err(zip_error)?;
    let mut f = zip.by_name(entry).map_err(zip_error)?;
    let mut out = Vec::with_capacity(usize::try_from(f.size()).unwrap_or(0));
    f.read_to_end(&mut out)?;
    Ok(out)
}

pub(crate) fn zip_error(e: zip::result::ZipError) -> crate::Error {
    match e {
        zip::result::ZipError::Io(io) => crate::Error::Io(io),
        other => crate::Error::Format(other.to_string()),
    }
}

/// Where an attachment goes inside the package: readable, and unique by the start of the hash.
pub(crate) fn entry_name(a: &Attachment) -> String {
    format!(
        "anexos/{}-{}",
        &a.hash[..8.min(a.hash.len())],
        atrio::attach::safe_name(&a.name)
    )
}

/// Where the thumbnail of a photo goes inside the package.
pub(crate) fn thumb_entry(hash: &str) -> String {
    format!("miniaturas/{hash}.webp")
}

/// The attachments the map uses: every `ideario://att/<hash>` in topic and loose notes.
pub fn referenced(map: &MindMap) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for n in map.nodes() {
        out.extend(atrio::attach::referenced(&n.note));
    }
    for f in &map.notes {
        out.extend(atrio::attach::referenced(&f.text));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_content_is_stored_once_and_photos_get_the_pipeline() {
        let mut a = Attachments::default();
        let doc = a.add("Roteiro.TXT", vec![1, 2, 3], 10);
        let again = a.add("outro.txt", vec![1, 2, 3], 20);
        assert_eq!(doc, again, "mesmo conteúdo, mesmo anexo");
        assert_eq!(doc.kind, "doc");
        assert_eq!(doc.hash.len(), 64, "o hash do Atrio: SHA-256 inteiro");
        assert_eq!(a.bytes(&doc.hash).unwrap(), vec![1, 2, 3]);
        assert!(a.thumb(&doc.hash).is_none());
        let jpg = atrio::media::tests::camera_jpeg(1200, 900, 1);
        let photo = a.add("IMG_1.JPG", jpg, 30);
        assert_eq!(
            (photo.kind.as_str(), photo.mime.as_str()),
            ("image", "image/webp")
        );
        assert_eq!((photo.width, photo.height), (Some(1200), Some(900)));
        assert!(a.thumb(&photo.hash).is_some(), "foto ganha miniatura");
        assert!(a.bytes("ffff").is_err());
    }

    #[test]
    fn referenced_finds_attachments_in_topic_and_loose_notes() {
        let mut map = MindMap::new("Mapa");
        let id = map.add_child(0, "Viagem").unwrap();
        map.find_mut(id).unwrap().note =
            "Foto:\n\n![praia](ideario://att/abc123)\n\n[📎 roteiro.pdf](ideario://att/def456)"
                .into();
        map.notes.push(crate::model::FreeNote {
            id: 1,
            pos: Default::default(),
            width: 200.0,
            height: 100.0,
            text: "![](ideario://att/0f0f)".into(),
        });
        let refs = referenced(&map);
        assert_eq!(
            refs.into_iter().collect::<Vec<_>>(),
            ["0f0f", "abc123", "def456"]
        );
    }
}
