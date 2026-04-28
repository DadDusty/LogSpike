//! Live-tail support.
//!
//! `Tailer` watches a single file via the OS-native filesystem notification
//! mechanism (`FSEvents` on macOS, `inotify` on Linux, `ReadDirectoryChangesW`
//! on Windows) and turns growth events into `TailEvent`s on a bounded
//! channel. We deliberately keep the watcher thread doing the bare minimum;
//! the actual reindex is done on the consumer's thread so back-pressure is
//! visible to the caller.
//!
//! Rotation handling
//! -----------------
//! If the file shrinks (truncation, log rotation), we emit `Rotated` and the
//! caller is expected to reopen via `LogFile::open`. We do not attempt to
//! transparently reopen because the caller often wants to surface "file
//! rotated" in the UI explicitly.

use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use crossbeam_channel::{bounded, Receiver, Sender};
use notify::{
    event::{ModifyKind, RemoveKind},
    EventKind, RecommendedWatcher, RecursiveMode, Watcher,
};
use serde::Serialize;

use crate::error::Result;
use crate::index::LogFile;

/// Notifications a tailer emits.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TailEvent {
    /// `count` lines were appended; total now `total_lines`.
    Appended { count: u64, total_lines: u64 },
    /// File was truncated or rotated; caller should reopen.
    Rotated,
    /// File was deleted while watching.
    Removed,
}

/// Owns the watcher thread. Drop the `Tailer` to stop tailing.
pub struct Tailer {
    _watcher: RecommendedWatcher,
    rx: Receiver<TailEvent>,
    path: PathBuf,
}

impl std::fmt::Debug for Tailer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tailer").field("path", &self.path).finish()
    }
}

impl Tailer {
    /// Begin tailing `file`. The returned `Tailer` keeps the watcher alive;
    /// drop it to stop.
    pub fn start(file: LogFile) -> Result<Self> {
        let (tx, rx) = bounded::<TailEvent>(64);
        let path = file.path().to_path_buf();

        // notify gives us raw events on its own thread. We do the indexing
        // here so the consumer just receives ready-to-render counts.
        let path_for_handler = path.clone();
        let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
            let Ok(ev) = res else {
                tracing::warn!("watcher error: {:?}", res.err());
                return;
            };
            if !ev.paths.iter().any(|p| p == &path_for_handler) {
                return;
            }
            handle_event(&ev, &file, &tx);
        })?;
        watcher.watch(&path, RecursiveMode::NonRecursive)?;

        Ok(Self { _watcher: watcher, rx, path })
    }

    /// Drain the next event, blocking up to `timeout`. `None` means no event
    /// arrived in the window.
    pub fn next_event(&self, timeout: Duration) -> Option<TailEvent> {
        self.rx.recv_timeout(timeout).ok()
    }

    /// Non-blocking read of all currently queued events.
    pub fn drain(&self) -> Vec<TailEvent> {
        let mut out = Vec::new();
        while let Ok(ev) = self.rx.try_recv() {
            out.push(ev);
        }
        out
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

fn handle_event(ev: &notify::Event, file: &LogFile, tx: &Sender<TailEvent>) {
    match ev.kind {
        EventKind::Modify(ModifyKind::Data(_)) | EventKind::Modify(ModifyKind::Any) => {
            match file.resync() {
                Ok(0) => {
                    // No growth — could be a metadata-only modify or a
                    // shrink (rotation). Detect rotation by checking if the
                    // file is now smaller than what we last indexed. The
                    // simplest signal: try to re-mmap and compare.
                    if let Ok(meta) = std::fs::metadata(file.path()) {
                        if meta.len() < file.byte_len() {
                            let _ = tx.try_send(TailEvent::Rotated);
                        }
                    }
                }
                Ok(count) => {
                    let _ = tx.try_send(TailEvent::Appended {
                        count,
                        total_lines: file.line_count(),
                    });
                }
                Err(e) => tracing::warn!(?e, "resync failed"),
            }
        }
        EventKind::Remove(RemoveKind::File) | EventKind::Remove(RemoveKind::Any) => {
            let _ = tx.try_send(TailEvent::Removed);
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use std::{io::Write, time::Duration};

    use tempfile::NamedTempFile;

    use super::*;

    #[test]
    fn appended_lines_produce_event() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, "first").unwrap();
        f.flush().unwrap();
        let lf = LogFile::open(f.path()).unwrap();
        let tailer = Tailer::start(lf).unwrap();

        // Give the watcher a moment to register.
        std::thread::sleep(Duration::from_millis(50));

        writeln!(f, "second").unwrap();
        writeln!(f, "third").unwrap();
        f.flush().unwrap();

        // Allow up to 2s for the FS notification to surface (CI machines can
        // be slow). At least one Appended event should arrive.
        let mut got_appended = false;
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while std::time::Instant::now() < deadline {
            if let Some(TailEvent::Appended { .. }) = tailer.next_event(Duration::from_millis(100)) {
                got_appended = true;
                break;
            }
        }
        assert!(got_appended, "expected at least one Appended event");
    }
}
