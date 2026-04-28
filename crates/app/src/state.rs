//! Per-process registry of open files and their tailers.
//!
//! We hand the frontend opaque `FileId`s instead of paths so:
//! * Reopening the same file as a second tab works without aliasing.
//! * The frontend can't accidentally start tailing arbitrary paths via IPC.
//! * Closing one tab cleanly shuts down only that tab's watcher.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use dashmap::DashMap;
use logspike_core::{LogFile, Session, Tailer, View};
use parking_lot::Mutex;
use serde::Serialize;

/// Opaque handle. Treat as a u64 elsewhere.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct FileId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct SessionId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct ViewId(pub u64);

pub struct OpenFile {
    pub log: LogFile,
    /// `None` until tailing is started. Wrapped in a `Mutex` so a single
    /// tail can be started/stopped through `&AppState` without needing
    /// `&mut`.
    pub tailer: Mutex<Option<Tailer>>,
}

pub struct AppState {
    next_id: AtomicU64,
    files: DashMap<FileId, OpenFile>,
    sessions: DashMap<SessionId, Arc<Session>>,
    views: DashMap<ViewId, Arc<View>>,
    pub folder_path: Mutex<Option<PathBuf>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            next_id: AtomicU64::new(1),
            files: DashMap::new(),
            sessions: DashMap::new(),
            views: DashMap::new(),
            folder_path: Mutex::new(None),
        }
    }

    pub fn set_folder(&self, path: PathBuf) {
        *self.folder_path.lock() = Some(path);
    }

    pub fn insert(&self, log: LogFile) -> FileId {
        let id = FileId(self.next_id.fetch_add(1, Ordering::Relaxed));
        self.files.insert(id, OpenFile { log, tailer: Mutex::new(None) });
        id
    }

    pub fn get(&self, id: FileId) -> Option<dashmap::mapref::one::Ref<'_, FileId, OpenFile>> {
        self.files.get(&id)
    }

    pub fn remove(&self, id: FileId) -> Option<OpenFile> {
        self.files.remove(&id).map(|(_, v)| v)
    }

    pub fn create_session(&self, file_ids: Vec<FileId>) -> Option<SessionId> {
        let mut logs = Vec::new();
        for id in file_ids {
            if let Some(f) = self.get(id) {
                logs.push(f.log.clone());
            }
        }
        if logs.is_empty() { return None; }
        
        let session = Arc::new(Session::new(logs));
        let id = SessionId(self.next_id.fetch_add(1, Ordering::Relaxed));
        self.sessions.insert(id, session);
        Some(id)
    }

    pub fn get_session(&self, id: SessionId) -> Option<Arc<Session>> {
        self.sessions.get(&id).map(|r| r.value().clone())
    }

    pub fn create_view(&self, session_id: SessionId, levels: Vec<logspike_core::LogLevel>) -> Option<ViewId> {
        let session = self.get_session(session_id)?;
        let view = Arc::new(View::new(session, levels));
        let id = ViewId(self.next_id.fetch_add(1, Ordering::Relaxed));
        self.views.insert(id, view);
        Some(id)
    }

    pub fn get_view(&self, id: ViewId) -> Option<Arc<View>> {
        self.views.get(&id).map(|r| r.value().clone())
    }

    pub fn sort_view(&self, id: ViewId, column: String, direction: String) -> Option<()> {
        let view_arc = self.get_view(id)?;
        // We need to mutate the View, but it's behind an Arc and inside a DashMap
        // In a real app we'd probably use a Mutex inside the View or similar
        // For this spike, we'll try to get a mutable reference if possible or just replace it
        let mut view = (*view_arc).clone();
        view.sort(column, direction);
        self.views.insert(id, Arc::new(view));
        Some(())
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
