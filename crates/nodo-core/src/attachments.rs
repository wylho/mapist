//! Attachments of a map: photos, audio, video and other files placed in topic notes. A note refers to one by the hash
//! of its content (`ideario://att/<hash>`, the same Markdown the Ideario writes, through `atrio::markdown`). The bytes
//! live inside the `.nodo` package (`package.rs`) and are read from it only when asked for; the ones added since the
//! last save are kept in memory until the next one.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::model::MindMap;

/// What the map knows about an attachment (kept in `mapa.json`, and what the card receives).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Attachment {
    pub hash: String,
    pub name: String,
    pub mime: String,
    /// `image`, `pdf`, `doc`, `sheet`, `audio`, `video` or `other`, as the Ideario sorts them.
    pub kind: String,
    pub bytes: u64,
    /// When it came in, Unix time in milliseconds.
    pub added_at: i64,
}

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
}

/// Hash of a file's content: the attachment's name in the notes. Equal files are stored once.
fn hash_of(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .take(16)
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// MIME type by the file's extension (what the card and the system viewer need).
pub fn mime_of(name: &str) -> &'static str {
    let ext = name
        .rsplit_once('.')
        .map(|(_, e)| e.to_ascii_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "heic" => "image/heic",
        "svg" => "image/svg+xml",
        "pdf" => "application/pdf",
        "mp3" => "audio/mpeg",
        "m4a" => "audio/mp4",
        "ogg" | "oga" => "audio/ogg",
        "wav" => "audio/wav",
        "webm" => "video/webm",
        "mp4" | "m4v" => "video/mp4",
        "mov" => "video/quicktime",
        "doc" => "application/msword",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "odt" => "application/vnd.oasis.opendocument.text",
        "xls" => "application/vnd.ms-excel",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "ods" => "application/vnd.oasis.opendocument.spreadsheet",
        "csv" => "text/csv",
        "txt" | "md" => "text/plain",
        _ => "application/octet-stream",
    }
}

/// The Ideario's kinds of attachment, by MIME type and extension.
pub fn kind_of(mime: &str, name: &str) -> &'static str {
    let ext = name
        .rsplit_once('.')
        .map(|(_, e)| e.to_ascii_lowercase())
        .unwrap_or_default();
    if mime.starts_with("image/") {
        "image"
    } else if mime.starts_with("audio/") {
        "audio"
    } else if mime.starts_with("video/") {
        "video"
    } else if mime == "application/pdf" {
        "pdf"
    } else if matches!(
        ext.as_str(),
        "doc" | "docx" | "odt" | "rtf" | "txt" | "md" | "pages"
    ) {
        "doc"
    } else if matches!(ext.as_str(), "xls" | "xlsx" | "ods" | "csv" | "numbers") {
        "sheet"
    } else {
        "other"
    }
}

impl Attachments {
    /// Store a file and return its attachment (the same one when this content is already here).
    pub fn add(&mut self, name: &str, bytes: Vec<u8>, now: i64) -> Attachment {
        let hash = hash_of(&bytes);
        if let Some(a) = self.meta.get(&hash) {
            return a.clone();
        }
        let name = clean_name(name);
        let mime = mime_of(&name).to_string();
        let a = Attachment {
            kind: kind_of(&mime, &name).to_string(),
            bytes: bytes.len() as u64,
            hash: hash.clone(),
            name,
            mime,
            added_at: now,
        };
        self.meta.insert(hash.clone(), a.clone());
        self.data.insert(hash, Data::Memory(Arc::new(bytes)));
        a
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

    /// Known from the package just read: the bytes stay in `file` until asked for.
    pub(crate) fn packed(&mut self, a: Attachment, file: &Path, entry: String) {
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

/// A name that is safe inside the package and on any system (no folders, no characters Windows refuses).
fn clean_name(name: &str) -> String {
    let base = name.rsplit(['/', '\\']).next().unwrap_or(name);
    let clean: String = base
        .chars()
        .map(|c| {
            if "<>:\"|?*".contains(c) || c.is_control() {
                '_'
            } else {
                c
            }
        })
        .collect();
    let clean = clean.trim().trim_matches('.').to_string();
    if clean.is_empty() {
        "anexo".to_string()
    } else {
        clean
    }
}

/// Where an attachment goes inside the package: readable, and unique by the start of the hash.
pub(crate) fn entry_name(a: &Attachment) -> String {
    format!("anexos/{}-{}", &a.hash[..8.min(a.hash.len())], a.name)
}

/// The attachments the map uses: every `ideario://att/<hash>` in topic and loose notes.
pub fn referenced(map: &MindMap) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut scan = |text: &str| {
        for part in text.split(atrio::markdown::ATT).skip(1) {
            let hash: String = part.chars().take_while(char::is_ascii_hexdigit).collect();
            if !hash.is_empty() {
                out.insert(hash);
            }
        }
    };
    for n in map.nodes() {
        scan(&n.note);
    }
    for f in &map.notes {
        scan(&f.text);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_content_is_stored_once_and_kinds_follow_the_ideario() {
        let mut a = Attachments::default();
        let foto = a.add("Praia.JPG", vec![1, 2, 3], 10);
        let again = a.add("outra.jpg", vec![1, 2, 3], 20);
        assert_eq!(foto, again, "mesmo conteúdo, mesmo anexo");
        assert_eq!(foto.kind, "image");
        assert_eq!(foto.mime, "image/jpeg");
        assert_eq!(foto.hash.len(), 32);
        assert_eq!(a.bytes(&foto.hash).unwrap(), vec![1, 2, 3]);
        assert_eq!(kind_of(mime_of("aula.m4a"), "aula.m4a"), "audio");
        assert_eq!(kind_of(mime_of("plano.xlsx"), "plano.xlsx"), "sheet");
        assert_eq!(kind_of(mime_of("x.zip"), "x.zip"), "other");
        assert!(a.bytes("ffff").is_err());
    }

    #[test]
    fn names_are_safe_inside_the_package() {
        assert_eq!(clean_name("../../etc/passwd"), "passwd");
        assert_eq!(clean_name("C:\\fotos\\a:b?.png"), "a_b_.png");
        assert_eq!(clean_name("..."), "anexo");
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
