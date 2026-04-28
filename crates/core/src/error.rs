//! Crate-level error type. We use `thiserror` for ergonomic conversions while
//! keeping the variants explicit so callers can pattern-match if they care.

use std::{io, path::PathBuf};

use thiserror::Error;

/// Convenience alias used throughout the crate.
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("io error on {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("file is empty: {0}")]
    EmptyFile(PathBuf),

    #[error("invalid line index {requested}, file has {total} lines")]
    LineOutOfRange { requested: u64, total: u64 },

    #[error("invalid utf-8 at byte offset {offset}")]
    InvalidUtf8 { offset: u64 },

    #[error("regex compile error: {0}")]
    Regex(#[from] regex::Error),

    #[error("file watcher error: {0}")]
    Watcher(#[from] notify::Error),
}

impl Error {
    /// Helper to attach a path to a raw `io::Error`.
    pub fn io(path: impl Into<PathBuf>, source: io::Error) -> Self {
        Error::Io { path: path.into(), source }
    }
}
