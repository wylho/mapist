//! The `.nodo` package: a zip with the map's text (`mapa.json`, the format of `nodofile.rs`) and the files the notes
//! use (`anexos/`). One file to send or keep, and still open: any unzip tool shows the map and the attachments as
//! ordinary files. Photos and videos go in as they came (they are compressed already), so saving is little more than
//! copying their bytes; only the text is compressed. Maps saved before attachments are plain JSON and still open.

use std::fs::File;
use std::io::Write;
use std::path::Path;

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::attachments::{entry_name, referenced, thumb_entry, zip_error, Attachments, Data};
use crate::model::MindMap;
use crate::nodofile;

/// Name of the map's text inside the package.
pub const MAP_ENTRY: &str = "mapa.json";

/// Whether these first bytes are a zip (every zip starts with "PK").
pub fn is_package(head: &[u8]) -> bool {
    head.starts_with(b"PK\x03\x04")
}

/// Read a `.nodo`: a package, or the plain JSON of the maps saved before attachments.
pub fn read(path: &Path) -> crate::Result<(MindMap, Attachments)> {
    let bytes = std::fs::read(path)?;
    if !is_package(&bytes) {
        let text = String::from_utf8(bytes)
            .map_err(|_| crate::Error::Format("not a Nodo map".to_string()))?;
        return Ok((nodofile::from_nodo(&text)?, Attachments::default()));
    }
    let mut zip = ZipArchive::new(std::io::Cursor::new(bytes)).map_err(zip_error)?;
    let text = {
        let mut f = zip.by_name(MAP_ENTRY).map_err(|_| {
            crate::Error::Format(format!("{MAP_ENTRY} is missing from the package"))
        })?;
        let mut s = String::new();
        std::io::Read::read_to_string(&mut f, &mut s)?;
        s
    };
    let (map, list) = nodofile::from_nodo_with(&text)?;
    let mut atts = Attachments::default();
    for a in list {
        let entry = entry_name(&a);
        // Listed but missing (a hand-edited package): left out, the note just shows a missing file.
        if zip.index_for_name(&entry).is_some() {
            let thumb = Some(thumb_entry(&a.hash)).filter(|t| zip.index_for_name(t).is_some());
            atts.packed(a, path, entry, thumb);
        }
    }
    Ok((map, atts))
}

/// Write a `.nodo` package atomically: everything goes to a temporary file beside the target, which then takes its
/// place, so a crash mid-save leaves the old file intact. Only the attachments some note still uses go in; the others
/// stay readable in this session (kept in memory) in case an undo brings them back.
pub fn write(path: &Path, map: &MindMap, atts: &mut Attachments) -> crate::Result<()> {
    let used = referenced(map);
    let list: Vec<_> = atts
        .iter()
        .filter(|a| used.contains(&a.hash))
        .cloned()
        .collect();
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let tmp = path.with_file_name(format!(".{name}.nodo-tmp"));
    let result = write_to(&tmp, map, &list, atts);
    if let Err(e) = result {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }
    // What lives only in the file being replaced and is not going in: keep it in memory first.
    let gone: Vec<String> = atts
        .data
        .iter()
        .filter(|(h, d)| {
            !used.contains(*h) && matches!(d, Data::Packed { file, .. } if file == path)
        })
        .map(|(h, _)| h.clone())
        .collect();
    for h in &gone {
        if let Some(Ok(t)) = atts.thumbs.get(h).map(Data::read) {
            atts.thumbs
                .insert(h.clone(), Data::Memory(std::sync::Arc::new(t)));
        }
    }
    for h in gone {
        if let Ok(bytes) = atts.bytes(&h) {
            atts.data
                .insert(h, Data::Memory(std::sync::Arc::new(bytes)));
        }
    }
    if let Err(e) = std::fs::rename(&tmp, path) {
        let _ = std::fs::remove_file(&tmp);
        return Err(e.into());
    }
    // From now on the saved ones are read from the new file.
    for a in &list {
        atts.data.insert(
            a.hash.clone(),
            Data::Packed {
                file: path.to_path_buf(),
                entry: entry_name(a),
            },
        );
        if atts.thumbs.contains_key(&a.hash) {
            atts.thumbs.insert(
                a.hash.clone(),
                Data::Packed {
                    file: path.to_path_buf(),
                    entry: thumb_entry(&a.hash),
                },
            );
        }
    }
    Ok(())
}

fn write_to(
    tmp: &Path,
    map: &MindMap,
    list: &[crate::attachments::Attachment],
    atts: &Attachments,
) -> crate::Result<()> {
    let mut zip = ZipWriter::new(File::create(tmp)?);
    let text = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let stored = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Stored)
        .large_file(true);
    zip.start_file(MAP_ENTRY, text).map_err(zip_error)?;
    zip.write_all(nodofile::to_nodo_with(map, list)?.as_bytes())?;
    for a in list {
        let entry = entry_name(a);
        match atts.data.get(&a.hash) {
            // Already in a package: copied as it is, without reading it into memory or compressing it again.
            Some(Data::Packed { file, entry: from }) => {
                let mut src = ZipArchive::new(File::open(file)?).map_err(zip_error)?;
                let f = src.by_name(from).map_err(zip_error)?;
                if *from == entry {
                    zip.raw_copy_file(f).map_err(zip_error)?;
                } else {
                    zip.raw_copy_file_rename(f, &entry).map_err(zip_error)?;
                }
            }
            Some(Data::Memory(bytes)) => {
                zip.start_file(entry, stored).map_err(zip_error)?;
                zip.write_all(bytes)?;
            }
            None => {}
        }
        // The thumbnail of a photo, beside it (`miniaturas/<hash>.webp`).
        let thumb = thumb_entry(&a.hash);
        match atts.thumbs.get(&a.hash) {
            Some(Data::Packed { file, entry: from }) => {
                let mut src = ZipArchive::new(File::open(file)?).map_err(zip_error)?;
                let f = src.by_name(from).map_err(zip_error)?;
                zip.raw_copy_file_rename(f, &thumb).map_err(zip_error)?;
            }
            Some(Data::Memory(bytes)) => {
                zip.start_file(thumb, stored).map_err(zip_error)?;
                zip.write_all(bytes)?;
            }
            None => {}
        }
    }
    let file = zip.finish().map_err(zip_error)?;
    file.sync_all()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::undo::Action;
    use crate::Editor;
    use std::io::Read;

    fn dir(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("nodo-pkg-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// A map with one topic whose note shows a photo.
    fn with_photo(ed: &mut Editor, bytes: Vec<u8>) -> (u64, String) {
        let a = ed.add_attachment("praia.jpg", bytes, 1);
        let id = ed
            .apply(Action::AddTopic {
                parent: 0,
                text: "Viagem".into(),
                note: format!("![praia](ideario://att/{})", a.hash),
                color: None,
                card: Default::default(),
            })
            .unwrap()
            .unwrap();
        (id, a.hash)
    }

    #[test]
    fn a_real_photo_goes_in_as_webp_with_its_thumbnail_and_comes_back() {
        let d = dir("miniatura");
        let path = d.join("foto.nodo");
        let mut ed = Editor::new("Fotos");
        let jpg = atrio::media::tests::camera_jpeg(1600, 1200, 1);
        let (_, hash) = with_photo(&mut ed, jpg);
        ed.save(Some(&path)).unwrap();
        let zip = ZipArchive::new(std::fs::File::open(&path).unwrap()).unwrap();
        assert!(zip
            .index_for_name(&format!("miniaturas/{hash}.webp"))
            .is_some());
        assert!(
            zip.index_for_name(&format!("anexos/{}-praia.webp", &hash[..8]))
                .is_some(),
            "a foto vira WebP"
        );
        let back = Editor::open(&path).unwrap();
        let a = back.attachments().get(&hash).unwrap().clone();
        assert_eq!(
            (a.width, a.height),
            (Some(1600), Some(1200)),
            "a proporção vai junto (a capa usa)"
        );
        assert_eq!(a.palette.as_ref().map(Vec::len), Some(5));
        let thumb = back.attachments().thumb(&hash).unwrap().read().unwrap();
        assert!(thumb.starts_with(b"RIFF"), "a miniatura volta");
        // Salvar de novo copia a miniatura também.
        let mut back = back;
        let again = d.join("copia.nodo");
        back.save(Some(&again)).unwrap();
        let zip = ZipArchive::new(std::fs::File::open(&again).unwrap()).unwrap();
        assert!(zip
            .index_for_name(&format!("miniaturas/{hash}.webp"))
            .is_some());
    }

    #[test]
    fn a_map_with_a_photo_goes_and_comes_back_in_one_open_zip() {
        let d = dir("ida");
        let path = d.join("viagem.nodo");
        let mut ed = Editor::new("Férias");
        let photo: Vec<u8> = (0..50_000u32).map(|i| (i % 251) as u8).collect();
        let (_, hash) = with_photo(&mut ed, photo.clone());
        ed.save(Some(&path)).unwrap();

        let bytes = std::fs::read(&path).unwrap();
        assert!(is_package(&bytes), "o .nodo é um zip");
        // Aberto com um descompactador qualquer: o mapa é texto legível e a foto é um arquivo comum.
        let mut zip = ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
        let mut text = String::new();
        zip.by_name(MAP_ENTRY)
            .unwrap()
            .read_to_string(&mut text)
            .unwrap();
        assert!(text.contains("\"format\": \"nodo\"") && text.contains("praia.jpg"));
        let entry = format!("anexos/{}-praia.jpg", &hash[..8]);
        let mut f = zip.by_name(&entry).unwrap();
        assert_eq!(
            f.compression(),
            CompressionMethod::Stored,
            "a foto entra como veio"
        );
        let mut inside = Vec::new();
        f.read_to_end(&mut inside).unwrap();
        assert_eq!(inside, photo);

        let back = Editor::open(&path).unwrap();
        assert_eq!(back.attachment_bytes(&hash).unwrap(), photo);
        assert_eq!(back.attachments().get(&hash).unwrap().kind, "image");
        assert!(!back.is_dirty());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn maps_saved_before_attachments_still_open() {
        let d = dir("antigo");
        let path = d.join("antigo.nodo");
        std::fs::write(
            &path,
            crate::nodofile::to_nodo(&crate::model::MindMap::new("Antigo")).unwrap(),
        )
        .unwrap();
        let ed = Editor::open(&path).unwrap();
        assert_eq!(ed.map().root.text, "Antigo");
        assert!(ed.attachments().is_empty());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn an_attachment_no_note_uses_leaves_the_file_but_an_undo_brings_it_back() {
        let d = dir("desfazer");
        let path = d.join("mapa.nodo");
        let mut ed = Editor::new("Mapa");
        let (id, hash) = with_photo(&mut ed, vec![7; 1000]);
        ed.save(Some(&path)).unwrap();
        ed.apply(Action::SetNote {
            id,
            note: String::new(),
        })
        .unwrap();
        ed.save(None).unwrap();
        let in_file = |p: &Path| {
            ZipArchive::new(File::open(p).unwrap())
                .unwrap()
                .file_names()
                .any(|n| n.contains(&hash[..8]))
        };
        assert!(!in_file(&path), "ninguém usa: sai do arquivo");
        assert_eq!(
            ed.attachment_bytes(&hash).unwrap(),
            vec![7; 1000],
            "mas continua aqui nesta sessão"
        );
        assert!(ed.undo());
        ed.save(None).unwrap();
        assert!(in_file(&path), "o desfazer trouxe a foto de volta");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn save_as_takes_the_attachments_to_the_new_file() {
        let d = dir("como");
        let (a, b) = (d.join("a.nodo"), d.join("b.nodo"));
        let mut ed = Editor::new("Mapa");
        let (_, hash) = with_photo(&mut ed, vec![9; 300]);
        ed.save(Some(&a)).unwrap();
        let mut ed = Editor::open(&a).unwrap();
        ed.save(Some(&b)).unwrap();
        std::fs::remove_file(&a).unwrap();
        assert_eq!(
            Editor::open(&b).unwrap().attachment_bytes(&hash).unwrap(),
            vec![9; 300]
        );
        let _ = std::fs::remove_dir_all(&d);
    }
}
