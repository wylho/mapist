//! Attachments of the open maps for the window: bringing files in (from the card, or by path when dropped), the list
//! the card asks for, saving a copy, opening one in the system viewer, and the `att://localhost/<hash>` protocol that
//! shows and plays them (`convertFileSrc(hash, 'att')`), with ranges, so a video can be skipped through. The bytes
//! come from the `.nodo` package (`nodo_core::package`); the last one served stays in memory, because a player asks
//! for many pieces of the same file.

use std::path::Path;
use std::sync::{Arc, Mutex};

use nodo_core::atrio::attach::{hash_from_path, parse_range, percent_decode, wants_thumb};
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
    let hash = hash_from_path(req.uri().path());
    if hash.is_empty() {
        return status_only(StatusCode::NOT_FOUND);
    }
    let state = app.state::<AppState>();
    // `?thumb`: the thumbnail of a photo (WebP), as in the Ideario; a photo without one gets the photo itself.
    if wants_thumb(req.uri().query()) {
        let thumb = lock(&state).ok().and_then(|doc| {
            let found = doc.iter().find_map(|t| t.editor.attachments().thumb(&hash));
            found
        });
        match thumb.map(|d| d.read()) {
            Some(Ok(bytes)) => {
                return Response::builder()
                    .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
                    .header(header::CONTENT_TYPE, "image/webp")
                    .header(header::CACHE_CONTROL, "max-age=31536000, immutable")
                    .body(bytes)
                    .unwrap_or_else(|_| status_only(StatusCode::INTERNAL_SERVER_ERROR));
            }
            _ => {
                let is_image = lock(&state).ok().is_some_and(|doc| {
                    doc.iter().any(|t| {
                        t.editor
                            .attachments()
                            .get(&hash)
                            .is_some_and(|a| a.kind == "image")
                    })
                });
                if !is_image {
                    return status_only(StatusCode::NOT_FOUND);
                }
            }
        }
    }
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

fn status_only(code: StatusCode) -> Response<Vec<u8>> {
    let mut res = Response::new(Vec::new());
    *res.status_mut() = code;
    res
}
