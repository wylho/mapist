//! Attachments of the open maps for the window: bringing files in (from the card, or by path when dropped), the list
//! the card asks for, saving a copy, opening one in the system viewer, and the `att://localhost/<hash>` protocol that
//! shows and plays them (`convertFileSrc(hash, 'att')`), with ranges, so a video can be skipped through. The bytes
//! come from the `.nodo` package (`nodo_core::package`); the last one served stays in memory, because a player asks
//! for many pieces of the same file.

use std::path::Path;
use std::sync::{Arc, Mutex};

use nodo_core::attachments::{Attachment, Data};
use tauri::http::{header, Request, Response, StatusCode};
use tauri::{Manager, State};

use crate::{editor, lock, AppState};

/// The last attachment served, whole, by hash.
#[derive(Default)]
pub struct Served(Mutex<Option<(String, Arc<Vec<u8>>)>>);

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// A file from the card (a photo, a recording): the bytes come raw in the body, the name in `x-name`.
#[tauri::command]
pub fn import_file(
    state: State<'_, AppState>,
    request: tauri::ipc::Request<'_>,
) -> Result<Attachment, String> {
    let tauri::ipc::InvokeBody::Raw(bytes) = request.body() else {
        return Err("o arquivo não chegou".to_string());
    };
    let name = request
        .headers()
        .get("x-name")
        .and_then(|v| v.to_str().ok())
        .map(percent_decode)
        .unwrap_or_else(|| "anexo".to_string());
    let mut doc = lock(&state)?;
    Ok(editor(&mut doc)?.add_attachment(&name, bytes.clone(), now()))
}

/// A file by its path on disk (dropped on the window).
#[tauri::command]
pub fn import_path(state: State<'_, AppState>, path: String) -> Result<Attachment, String> {
    let p = Path::new(&path);
    let bytes = std::fs::read(p).map_err(|e| format!("não foi possível ler {path}: {e}"))?;
    let name = p
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "anexo".to_string());
    let mut doc = lock(&state)?;
    Ok(editor(&mut doc)?.add_attachment(&name, bytes, now()))
}

/// The attachments with these hashes in the map in front.
#[tauri::command]
pub fn get_attachments(
    state: State<'_, AppState>,
    hashes: Vec<String>,
) -> Result<Vec<Attachment>, String> {
    let mut doc = lock(&state)?;
    Ok(editor(&mut doc)?
        .attachments()
        .list(hashes.iter().map(String::as_str)))
}

/// Where an attachment's bytes are, in any open map (its card can be in a tab behind).
fn source(state: &AppState, hash: &str) -> Option<Data> {
    let doc = lock(state).ok()?;
    let found = doc.iter().find_map(|t| t.editor.attachments().source(hash));
    found
}

/// Save a copy of an attachment where the person chose.
#[tauri::command]
pub fn save_attachment(
    state: State<'_, AppState>,
    hash: String,
    path: String,
) -> Result<(), String> {
    let data = source(&state, &hash).ok_or("esse anexo não está no mapa")?;
    let bytes = data.read().map_err(|e| e.to_string())?;
    nodo_core::write_atomic(Path::new(&path), &bytes)
        .map_err(|e| format!("não foi possível salvar: {e}"))
}

/// Open an attachment in the program the system uses for it (a copy in the temporary folder).
#[tauri::command]
pub fn open_attachment(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    hash: String,
) -> Result<(), String> {
    let (data, name) = {
        let doc = lock(&state)?;
        let found = doc.iter().find_map(|t| {
            let a = t.editor.attachments();
            Some((a.source(&hash)?, a.get(&hash)?.name.clone()))
        });
        found.ok_or("esse anexo não está no mapa")?
    };
    let dir = std::env::temp_dir()
        .join("nodo-anexos")
        .join(&hash[..hash.len().min(8)]);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let file = dir.join(name);
    if !file.exists() {
        std::fs::write(&file, data.read().map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    }
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_path(file.to_string_lossy(), None::<&str>)
        .map_err(|e| e.to_string())
}

/// Answer `att://localhost/<hash>` with the file, whole or the piece asked for (Range).
pub fn serve(app: &tauri::AppHandle, req: &Request<Vec<u8>>) -> Response<Vec<u8>> {
    let hash: String = req
        .uri()
        .path()
        .trim_start_matches('/')
        .chars()
        .filter(char::is_ascii_hexdigit)
        .collect();
    if hash.is_empty() {
        return status_only(StatusCode::NOT_FOUND);
    }
    let state = app.state::<AppState>();
    let served = app.state::<Served>();
    let cached = served.0.lock().ok().and_then(|c| {
        c.as_ref()
            .filter(|(h, _)| *h == hash)
            .map(|(_, b)| b.clone())
    });
    let mime = {
        let Ok(doc) = lock(&state) else {
            return status_only(StatusCode::SERVICE_UNAVAILABLE);
        };
        let found = doc
            .iter()
            .find_map(|t| t.editor.attachments().get(&hash).map(|a| a.mime.clone()));
        found
    };
    let bytes = match cached {
        Some(b) => b,
        None => {
            // A trava do mapa só para saber onde está; ler o arquivo (um vídeo grande) é sem ela.
            let Some(data) = source(&state, &hash) else {
                return status_only(StatusCode::NOT_FOUND);
            };
            let Ok(b) = data.read() else {
                return status_only(StatusCode::INTERNAL_SERVER_ERROR);
            };
            let b = Arc::new(b);
            if let Ok(mut c) = served.0.lock() {
                *c = Some((hash.clone(), b.clone()));
            }
            b
        }
    };
    let len = bytes.len() as u64;
    let mime = mime
        .filter(|m| header::HeaderValue::from_str(m).is_ok())
        .unwrap_or_else(|| "application/octet-stream".into());
    let range = req
        .headers()
        .get(header::RANGE)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| parse_range(v, len));
    let base = Response::builder()
        .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .header(header::CONTENT_TYPE, mime)
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::CACHE_CONTROL, "max-age=31536000, immutable");
    let res = match range {
        Some((start, end)) => base
            .status(StatusCode::PARTIAL_CONTENT)
            .header(header::CONTENT_RANGE, format!("bytes {start}-{end}/{len}"))
            .body(bytes[start as usize..=end as usize].to_vec()),
        None => base.status(StatusCode::OK).body(bytes.as_ref().clone()),
    };
    res.unwrap_or_else(|_| status_only(StatusCode::INTERNAL_SERVER_ERROR))
}

/// Largest piece for an open request ("bytes=N-"): the player asks for the rest in steps.
const CHUNK: u64 = 4 * 1024 * 1024;

/// "bytes=a-b", "bytes=a-" (up to `CHUNK`) or "bytes=-n" (the last n) → (start, end), inclusive.
fn parse_range(v: &str, len: u64) -> Option<(u64, u64)> {
    let (a, b) = v
        .strip_prefix("bytes=")?
        .split(',')
        .next()?
        .split_once('-')?;
    let last = len.checked_sub(1)?;
    let (start, end) = match (a.trim(), b.trim()) {
        ("", n) => (len.saturating_sub(n.parse().ok()?), last),
        (a, "") => {
            let a: u64 = a.parse().ok()?;
            (a, a.saturating_add(CHUNK - 1).min(last))
        }
        (a, b) => (a.parse().ok()?, b.parse::<u64>().ok()?.min(last)),
    };
    (start <= end && start < len).then_some((start, end))
}

fn status_only(code: StatusCode) -> Response<Vec<u8>> {
    let mut res = Response::new(Vec::new());
    *res.status_mut() = code;
    res
}

/// `encodeURIComponent` back to text (the name of a file can have accents and spaces).
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(b) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(b);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges() {
        assert_eq!(parse_range("bytes=0-99", 1000), Some((0, 99)));
        assert_eq!(parse_range("bytes=900-", 1000), Some((900, 999)));
        assert_eq!(
            parse_range("bytes=0-", 100 * 1024 * 1024),
            Some((0, CHUNK - 1))
        );
        assert_eq!(parse_range("bytes=-10", 1000), Some((990, 999)));
        assert_eq!(parse_range("bytes=0-5000", 1000), Some((0, 999)));
        assert_eq!(parse_range("bytes=1000-", 1000), None);
        assert_eq!(parse_range("lixo", 1000), None);
    }

    #[test]
    fn names_with_accents_come_back() {
        assert_eq!(
            percent_decode("F%C3%A9rias%20na%20praia.jpg"),
            "Férias na praia.jpg"
        );
        assert_eq!(percent_decode("sem-codigo.png"), "sem-codigo.png");
        assert_eq!(percent_decode("cortado%2"), "cortado%2");
    }
}
