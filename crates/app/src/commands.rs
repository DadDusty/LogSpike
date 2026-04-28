//! Tauri command surface.
//!
//! All CPU-bound work (initial indexing, search) is dispatched to
//! `tokio::task::spawn_blocking` so the Tauri main thread / async runtime
//! never blocks. State lookups are O(1) on a `DashMap`.
//!
//! Errors are converted to `String` at the boundary because Tauri IPC
//! requires `Serialize` errors and a flat string is the friendliest thing
//! for the JS side to render.

use std::{path::PathBuf, time::Duration};

use logspike_core::{
    FileFormat, LogFile, LogLevel, LogLine, Match, SearchOptions, Searcher, SessionLine, TailEvent,
    Tailer,
};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::state::{AppState, FileId, SessionId, ViewId};

/// Convert any error into a UI-friendly string.
fn to_msg<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct FileMeta {
    pub id: FileId,
    pub path: PathBuf,
    pub byte_len: u64,
    pub line_count: u64,
    pub format: FileFormat,
    pub last_timestamp: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ViewMeta {
    pub id: ViewId,
    pub line_count: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct FolderInfo {
    pub path: PathBuf,
    pub files: Vec<FileMeta>,
}

#[tauri::command]
pub async fn open_folder(
    _app: AppHandle,
    state: State<'_, AppState>,
    path: PathBuf,
) -> Result<FolderInfo, String> {
    state.inner().set_folder(path.clone());

    let mut files = Vec::new();
    // Simple recursive scan
    let mut stack = vec![path.clone()];
    while let Some(dir) = stack.pop() {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    stack.push(p);
                } else if is_log_file(&p) {
                    // Reuse open_file logic but within the task
                    if let Ok(log) = LogFile::open(&p) {
                        let line_count = log.line_count();
                        let format = log.format();
                        let last_ts = if line_count > 0 {
                            log.line(line_count - 1).ok().and_then(|l| l.parsed).and_then(|p| p.timestamp)
                        } else {
                            None
                        };
                        let meta = FileMeta {
                            id: state.inner().insert(log),
                            path: p,
                            byte_len: entry.metadata().map(|m| m.len()).unwrap_or(0),
                            line_count,
                            format,
                            last_timestamp: last_ts,
                        };
                        files.push(meta);
                    }
                }
            }
        }
    }

    Ok(FolderInfo { path, files })
}

fn is_log_file(path: &PathBuf) -> bool {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    matches!(ext, "log" | "txt" | "out" | "err" | "trace")
}

#[tauri::command]
pub async fn open_file(state: State<'_, AppState>, path: PathBuf) -> Result<FileMeta, String> {
    // Initial index can scan multiple GiB; never run it on the runtime.
    let log = tokio::task::spawn_blocking(move || LogFile::open(&path))
        .await
        .map_err(to_msg)?
        .map_err(to_msg)?;

    let path = log.path().to_path_buf();
    let byte_len = log.byte_len();
    let line_count = log.line_count();
    let format = log.format();
    let last_ts = if line_count > 0 {
        log.line(line_count - 1).ok().and_then(|l| l.parsed).and_then(|p| p.timestamp)
    } else {
        None
    };
    let id = state.inner().insert(log);

    Ok(FileMeta { id, path, byte_len, line_count, format, last_timestamp: last_ts })
}

#[tauri::command]
pub fn close_file(state: State<'_, AppState>, id: FileId) -> Result<(), String> {
    state.inner().remove(id);
    Ok(())
}

#[tauri::command]
pub fn file_meta(state: State<'_, AppState>, id: FileId) -> Result<FileMeta, String> {
    let entry = state.inner().get(id).ok_or_else(|| "file not open".to_owned())?;
    let line_count = entry.log.line_count();
    let last_ts = if line_count > 0 {
        entry.log.line(line_count - 1).ok().and_then(|l| l.parsed).and_then(|p| p.timestamp)
    } else {
        None
    };
    Ok(FileMeta {
        id,
        path: entry.log.path().to_path_buf(),
        byte_len: entry.log.byte_len(),
        line_count,
        format: entry.log.format(),
        last_timestamp: last_ts,
    })
}

#[tauri::command]
pub fn read_range(
    state: State<'_, AppState>,
    id: FileId,
    start: u64,
    end: u64,
) -> Result<Vec<LogLine>, String> {
    let entry = state.inner().get(id).ok_or_else(|| "file not open".to_owned())?;
    Ok(entry.log.range(start..end))
}

#[tauri::command]
pub async fn search(
    state: State<'_, AppState>,
    id: FileId,
    options: SearchOptions,
) -> Result<Vec<Match>, String> {
    // Clone the LogFile (cheap — it's an Arc internally) so we can move it
    // into the blocking task and release the DashMap reference immediately.
    let log = {
        let entry = state.inner().get(id).ok_or_else(|| "file not open".to_owned())?;
        entry.log.clone()
    };
    tokio::task::spawn_blocking(move || Searcher::run(&log, options))
        .await
        .map_err(to_msg)?
        .map_err(to_msg)
}

#[tauri::command]
pub fn start_tail(app: AppHandle, state: State<'_, AppState>, id: FileId) -> Result<(), String> {
    // Idempotent: if a tailer already exists, do nothing.
    let log = {
        let entry = state.inner().get(id).ok_or_else(|| "file not open".to_owned())?;
        if entry.tailer.lock().is_some() {
            return Ok(());
        }
        entry.log.clone()
    };

    let tailer = Tailer::start(log).map_err(to_msg)?;
    {
        let entry = state.inner().get(id).ok_or_else(|| "file not open".to_owned())?;
        *entry.tailer.lock() = Some(tailer);
    }

    // Spawn a forwarder. It re-fetches the AppState on every tick via the
    // AppHandle, so it stays valid for the lifetime of the app and exits
    // cleanly when the file is closed or the tailer removed.
    let app_for_task = app.clone();
    std::thread::Builder::new()
        .name(format!("logspike-tail-{}", id.0))
        .spawn(move || forward_tail(app_for_task, id))
        .map_err(to_msg)?;

    Ok(())
}

#[tauri::command]
pub fn stop_tail(state: State<'_, AppState>, id: FileId) -> Result<(), String> {
    if let Some(entry) = state.inner().get(id) {
        // Dropping the Tailer kills the OS watcher; the forwarder loop sees
        // `None` on the next tick and exits.
        entry.tailer.lock().take();
    }
    Ok(())
}

#[tauri::command]
pub fn create_session(
    state: State<'_, AppState>,
    file_ids: Vec<FileId>,
) -> Result<SessionId, String> {
    state.inner()
        .create_session(file_ids)
        .ok_or_else(|| "could not create session".to_owned())
}

#[tauri::command]
pub fn create_view(
    state: State<'_, AppState>,
    session_id: SessionId,
    levels: Vec<LogLevel>,
) -> Result<ViewId, String> {
    state.inner()
        .create_view(session_id, levels)
        .ok_or_else(|| "could not create view".to_owned())
}

#[tauri::command]
pub fn sort_view(
    state: State<'_, AppState>,
    id: ViewId,
    column: String,
    direction: String,
) -> Result<(), String> {
    state
        .inner()
        .sort_view(id, column, direction)
        .ok_or_else(|| "view not found".to_owned())
}

#[tauri::command]
pub fn view_meta(state: State<'_, AppState>, id: ViewId) -> Result<ViewMeta, String> {
    let view = state.inner().get_view(id).ok_or_else(|| "view not found".to_owned())?;
    Ok(ViewMeta {
        id,
        line_count: view.line_count(),
    })
}

#[tauri::command]
pub fn read_view_range(
    state: State<'_, AppState>,
    id: ViewId,
    start: u64,
    end: u64,
) -> Result<Vec<SessionLine>, String> {
    let view = state.inner().get_view(id).ok_or_else(|| "view not found".to_owned())?;
    Ok(view.range(start..end))
}

/// Background loop that forwards tail events to the webview.
fn forward_tail(app: AppHandle, id: FileId) {
    let event_name = format!("tail:{}", id.0);
    let state = app.state::<AppState>();
    loop {
        let event = {
            let Some(entry) = state.inner().get(id) else { break };
            let guard = entry.tailer.lock();
            let Some(tailer) = guard.as_ref() else { break };
            tailer.next_event(Duration::from_millis(250))
        };
        let Some(ev) = event else { continue };
        if app.emit(&event_name, &ev).is_err() {
            break;
        }
        if matches!(ev, TailEvent::Removed | TailEvent::Rotated) {
            break;
        }
    }
}
