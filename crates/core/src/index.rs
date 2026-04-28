//! Memory-mapped line index.
//!
//! `LogFile` is the central type. Construction does one pass over the bytes to
//! build an offset table; everything else is O(1) random access or a windowed
//! scan over a known byte range.
//!
//! Concurrency model
//! -----------------
//! * The mmap and offset table sit behind a `parking_lot::RwLock`.
//! * Read-mostly callers (viewport reads, search workers) take read locks and
//!   never block each other.
//! * The tail path takes a brief write lock to extend the index when the file
//!   grows. Writers are short-lived (tens of microseconds) so reader latency
//!   stays bounded.
//!
//! Why we copy line bytes out of the mmap
//! --------------------------------------
//! Returning `&str` borrowed from the mmap would force every caller to hold
//! the read lock for as long as they hold the reference, which is incompatible
//! with sending data across the Tauri IPC boundary. Copying ~100 visible lines
//! per viewport request is well under a microsecond, so we always return owned
//! strings. Search uses internal scans that *do* borrow from the mmap and
//! never cross the lock boundary.

use std::{
    collections::BinaryHeap,
    cmp::Ordering,
    fs::File,
    ops::Range,
    path::{Path, PathBuf},
    sync::Arc,
};

use memmap2::Mmap;
use parking_lot::RwLock;
use serde::Serialize;

use crate::error::{Error, Result};
use crate::parser::{ParsedLine, FileFormat, LogLevel, detect_format, parse_with_format};

/// One line as exposed to the UI layer. We keep the parsed level/timestamp
/// inline so the frontend doesn't have to re-parse on every render.
#[derive(Debug, Clone, Serialize)]
pub struct LogLine {
    /// 0-based line number within the file.
    pub number: u64,
    /// Byte offset of the line start within the underlying file.
    pub byte_offset: u64,
    /// Raw line text with trailing newline stripped.
    pub text: String,
    /// Best-effort parse result. `None` for unrecognised formats.
    pub parsed: Option<ParsedLine>,
}

/// Inclusive-start, exclusive-end half-open range of lines.
pub type LineRange = Range<u64>;

/// A memory-mapped log file plus its line offset index.
///
/// Cheap to clone (the contents sit behind an `Arc`).
#[derive(Clone)]
pub struct LogFile {
    inner: Arc<RwLock<Inner>>,
    path: PathBuf,
}

struct Inner {
    mmap: Mmap,
    /// `offsets[i]` is the byte offset of line `i`. The last entry is always
    /// the file length, so line `i` spans `offsets[i]..offsets[i+1]`. This
    /// "+1 sentinel" trick removes a branch from the hot read path.
    offsets: Vec<u64>,
    format: FileFormat,
}

impl std::fmt::Debug for LogFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let inner = self.inner.read();
        f.debug_struct("LogFile")
            .field("path", &self.path)
            .field("bytes", &inner.mmap.len())
            .field("lines", &(inner.offsets.len().saturating_sub(1)))
            .finish()
    }
}

impl LogFile {
    /// Open a file and build its initial line index.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let file = File::open(&path).map_err(|e| Error::io(&path, e))?;
        let len = file.metadata().map_err(|e| Error::io(&path, e))?.len();

        if len == 0 {
            return Err(Error::EmptyFile(path));
        }

        // SAFETY: we hold the `File` for the lifetime of the `Mmap`. The file
        // can still be appended to externally; that is fine because we
        // re-mmap on growth in `resync()`.
        let mmap = unsafe { Mmap::map(&file) }.map_err(|e| Error::io(&path, e))?;

        // Hint the kernel about access pattern. On Linux this enables
        // readahead; on other platforms it is a no-op. Failures here are
        // never fatal.
        #[cfg(unix)]
        {
            let _ = mmap.advise(memmap2::Advice::Sequential);
        }

        let offsets = build_offsets(&mmap, 0);
        let line_count = offsets.len().saturating_sub(1);

        // Detect format from first 20 lines
        let format = {
            let mut first_lines = Vec::new();
            for i in 0..line_count.min(20) {
                let start = offsets[i] as usize;
                let end = offsets[i + 1] as usize;
                let mut slice_end = end;
                if slice_end > start && mmap[slice_end - 1] == b'\n' {
                    slice_end -= 1;
                }
                if slice_end > start && mmap[slice_end - 1] == b'\r' {
                    slice_end -= 1;
                }
                let text = String::from_utf8_lossy(&mmap[start..slice_end]);
                first_lines.push(text.into_owned());
            }
            let refs: Vec<&str> = first_lines.iter().map(|s| s.as_str()).collect();
            detect_format(&refs)
        };

        tracing::info!(
            path = %path.display(),
            bytes = mmap.len(),
            lines = line_count,
            format = ?format,
            "indexed log file"
        );

        Ok(Self {
            inner: Arc::new(RwLock::new(Inner { mmap, offsets, format })),
            path,
        })
    }

    /// Filesystem path the file was opened from.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Total number of indexed lines.
    pub fn line_count(&self) -> u64 {
        let inner = self.inner.read();
        (inner.offsets.len() as u64).saturating_sub(1)
    }

    /// Total file length in bytes (snapshot at last index).
    pub fn byte_len(&self) -> u64 {
        self.inner.read().mmap.len() as u64
    }

    /// Detected log file format.
    pub fn format(&self) -> FileFormat {
        self.inner.read().format
    }

    /// Read a single line by 0-based index.
    pub fn line(&self, n: u64) -> Result<LogLine> {
        let inner = self.inner.read();
        let total = (inner.offsets.len() as u64).saturating_sub(1);
        if n >= total {
            return Err(Error::LineOutOfRange { requested: n, total });
        }
        Ok(materialise_line(&inner, n, inner.format))
    }

    /// Read a contiguous range of lines.
    ///
    /// Out-of-range bounds are clamped so the UI can request `(top..top+page)`
    /// without first checking the file length.
    pub fn range(&self, range: LineRange) -> Vec<LogLine> {
        let inner = self.inner.read();
        let total = (inner.offsets.len() as u64).saturating_sub(1);
        let start = range.start.min(total);
        let end = range.end.min(total);

        let mut out = Vec::with_capacity((end - start) as usize);
        let format = inner.format;
        for i in start..end {
            out.push(materialise_line(&inner, i, format));
        }
        out
    }

    /// Re-stat the file and extend the index if it grew. Returns the number of
    /// newly indexed lines (a single previously-incomplete line gaining more
    /// bytes counts as zero added lines).
    ///
    /// Called by the tail loop on each filesystem event.
    ///
    /// We restart the scan from the start of the previous final line, not from
    /// `old_len`. Without this, a file whose last line had no trailing
    /// newline at the time of the prior index would have that line silently
    /// split in two when the next chunk arrived.
    pub fn resync(&self) -> Result<u64> {
        let new_len = std::fs::metadata(&self.path)
            .map_err(|e| Error::io(&self.path, e))?
            .len();

        let old_len = self.byte_len();
        if new_len <= old_len {
            // Truncation or rotation handling lives a layer up; here we just
            // signal "nothing to do".
            return Ok(0);
        }

        let file = File::open(&self.path).map_err(|e| Error::io(&self.path, e))?;
        let new_mmap = unsafe { Mmap::map(&file) }.map_err(|e| Error::io(&self.path, e))?;

        let mut inner = self.inner.write();
        let prior_lines = (inner.offsets.len() as u64).saturating_sub(1);

        // Pop the sentinel.
        if !inner.offsets.is_empty() {
            inner.offsets.pop();
        }
        // Pop the last line's start so we can rescan it; the appended index
        // will re-include it. This is what corrects an unterminated last line.
        let restart_from = inner.offsets.pop().unwrap_or(0);

        let appended_offsets = build_offsets(&new_mmap, restart_from);
        inner.offsets.extend(appended_offsets);
        inner.mmap = new_mmap;
        // format stays the same for tail events

        let now_lines = (inner.offsets.len() as u64).saturating_sub(1);
        Ok(now_lines.saturating_sub(prior_lines))
    }

    /// Borrow the raw bytes for the duration of `f`. Used by the search
    /// engine; not exposed publicly because the borrow is lock-bound.
    pub(crate) fn with_bytes<R>(&self, f: impl FnOnce(&[u8], &[u64]) -> R) -> R {
        let inner = self.inner.read();
        f(&inner.mmap, &inner.offsets)
    }
}

/// A line in a Session or View, including source information.
#[derive(Debug, Clone, Serialize)]
pub struct SessionLine {
    #[serde(flatten)]
    pub line: LogLine,
    pub source_id: u32,
}

/// A Session is a collection of one or more LogFiles, potentially merged chronologically.
#[derive(Debug)]
pub struct Session {
    files: Vec<LogFile>,
    /// Maps global line index to (source_index, local_line_number)
    index: Vec<(u32, u64)>,
}

#[derive(Debug, Eq, PartialEq)]
struct MergeEntry {
    timestamp: Option<String>,
    file_idx: u32,
    local_n: u64,
}

impl Ord for MergeEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse order for min-heap
        match (&self.timestamp, &other.timestamp) {
            (Some(a), Some(b)) => b.cmp(a),
            (Some(_), None) => Ordering::Greater,
            (None, Some(_)) => Ordering::Less,
            (None, None) => Ordering::Equal,
        }
        .then_with(|| other.file_idx.cmp(&self.file_idx))
        .then_with(|| other.local_n.cmp(&self.local_n))
    }
}

impl PartialOrd for MergeEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Session {
    pub fn new(files: Vec<LogFile>) -> Self {
        if files.is_empty() {
            return Self { files, index: Vec::new() };
        }

        if files.len() == 1 {
            let file = &files[0];
            let count = file.line_count();
            let mut index = Vec::with_capacity(count as usize);
            for j in 0..count {
                index.push((0, j));
            }
            return Self { files, index };
        }

        // Multiple files: chronological merge
        let total_approx = files.iter().map(|f| f.line_count()).sum::<u64>() as usize;
        let mut index = Vec::with_capacity(total_approx);
        let mut heap = BinaryHeap::new();

        for (i, file) in files.iter().enumerate() {
            if file.line_count() > 0 {
                if let Ok(line) = file.line(0) {
                    heap.push(MergeEntry {
                        timestamp: line.parsed.and_then(|p| p.timestamp),
                        file_idx: i as u32,
                        local_n: 0,
                    });
                }
            }
        }

        while let Some(entry) = heap.pop() {
            index.push((entry.file_idx, entry.local_n));
            let next_n = entry.local_n + 1;
            let file = &files[entry.file_idx as usize];
            if next_n < file.line_count() {
                if let Ok(line) = file.line(next_n) {
                    heap.push(MergeEntry {
                        timestamp: line.parsed.and_then(|p| p.timestamp),
                        file_idx: entry.file_idx,
                        local_n: next_n,
                    });
                }
            }
        }

        Self { files, index }
    }

    pub fn line_count(&self) -> u64 {
        self.index.len() as u64
    }

    pub fn line(&self, n: u64) -> Result<SessionLine> {
        let (file_idx, local_n) = self.index.get(n as usize)
            .ok_or_else(|| Error::LineOutOfRange { requested: n, total: self.line_count() })?;
        let file = &self.files[*file_idx as usize];
        let mut line = file.line(*local_n)?;
        // Update the line number to be the global session line number
        line.number = n;
        Ok(SessionLine {
            line,
            source_id: *file_idx,
        })
    }

    pub fn range(&self, range: LineRange) -> Vec<SessionLine> {
        let start = range.start.min(self.line_count());
        let end = range.end.min(self.line_count());
        let mut out = Vec::with_capacity((end - start) as usize);
        for i in start..end {
            if let Ok(line) = self.line(i) {
                out.push(line);
            }
        }
        out
    }
}

/// A View is a filtered subset of a Session.
#[derive(Debug, Clone)]
pub struct View {
    session: Arc<Session>,
    /// Indices into session.index that match the filter
    indices: Vec<u64>,
    sort_column: Option<String>,
    sort_direction: Option<String>, // "asc" | "desc"
}

impl View {
    pub fn new(session: Arc<Session>, levels: Vec<LogLevel>) -> Self {
        let mut indices = Vec::new();
        if levels.is_empty() {
            // No filter = all lines
            indices = (0..session.line_count()).collect();
        } else {
            for i in 0..session.line_count() {
                if let Ok(sline) = session.line(i) {
                    if let Some(parsed) = sline.line.parsed {
                        if let Some(level) = parsed.level {
                            if levels.contains(&level) {
                                indices.push(i);
                            }
                        }
                    }
                }
            }
        }
        Self { session, indices, sort_column: None, sort_direction: None }
    }

    pub fn sort(&mut self, column: String, direction: String) {
        // This is expensive: we need to materialise/parse to sort
        // For now, we'll sort the indices based on the data in those lines
        let session = self.session.clone();
        
        self.indices.sort_by(|&a, &b| {
            let line_a = session.line(a).ok();
            let line_b = session.line(b).ok();

            let val_a = get_sort_value(line_a, &column);
            let val_b = get_sort_value(line_b, &column);

            if direction == "asc" {
                val_a.cmp(&val_b)
            } else {
                val_b.cmp(&val_a)
            }
        });

        self.sort_column = Some(column);
        self.sort_direction = Some(direction);
    }

    pub fn line_count(&self) -> u64 {
        self.indices.len() as u64
    }

    pub fn line(&self, n: u64) -> Result<SessionLine> {
        let session_idx = self.indices.get(n as usize)
            .ok_or_else(|| Error::LineOutOfRange { requested: n, total: self.line_count() })?;
        let mut sline = self.session.line(*session_idx)?;
        // Update the line number to be the global view line number
        sline.line.number = n;
        Ok(sline)
    }

    pub fn range(&self, range: LineRange) -> Vec<SessionLine> {
        let start = range.start.min(self.line_count());
        let end = range.end.min(self.line_count());
        let mut out = Vec::with_capacity((end - start) as usize);
        for i in start..end {
            if let Ok(line) = self.line(i) {
                out.push(line);
            }
        }
        out
    }
}

fn get_sort_value(line: Option<SessionLine>, column: &str) -> String {
    let Some(sline) = line else { return String::new(); };
    match column {
        "timestamp" => sline.line.parsed.and_then(|p| p.timestamp).unwrap_or_default(),
        "level" => format!("{:?}", sline.line.parsed.and_then(|p| p.level)),
        "component" => sline.line.parsed.and_then(|p| p.component).unwrap_or_default(),
        "message" => sline.line.parsed.and_then(|p| p.message).unwrap_or(sline.line.text),
        _ => sline.line.number.to_string(),
    }
}

/// Scan `mmap[start..]` for `\n` bytes and produce an offset table covering
/// those lines plus a final sentinel equal to `mmap.len()`.
fn build_offsets(mmap: &[u8], start: u64) -> Vec<u64> {
    // Heuristic: assume ~80 bytes per line. Saves a few reallocations on
    // typical text logs.
    let approx_lines = ((mmap.len() as u64).saturating_sub(start) / 80) as usize + 16;
    let mut offsets = Vec::with_capacity(approx_lines);
    offsets.push(start);

    // memchr is the SIMD-accelerated newline finder; this is the loop that
    // sets the indexing throughput ceiling.
    for nl in memchr::memchr_iter(b'\n', &mmap[start as usize..]) {
        let abs = start + nl as u64 + 1;
        if (abs as usize) < mmap.len() {
            offsets.push(abs);
        }
    }
    // Final sentinel = file length. Lets `line(i)` slice without a bounds
    // branch on `i + 1`.
    offsets.push(mmap.len() as u64);
    offsets
}

fn materialise_line(inner: &Inner, n: u64, format: FileFormat) -> LogLine {
    let start = inner.offsets[n as usize] as usize;
    let end = inner.offsets[n as usize + 1] as usize;
    // Strip a single trailing `\n` and an optional `\r`.
    let mut slice_end = end;
    if slice_end > start && inner.mmap[slice_end - 1] == b'\n' {
        slice_end -= 1;
    }
    if slice_end > start && inner.mmap[slice_end - 1] == b'\r' {
        slice_end -= 1;
    }
    let bytes = &inner.mmap[start..slice_end];
    // Lossy decode is intentional: log files routinely carry stray non-UTF-8
    // bytes (truncated multibyte sequences, binary noise from a misbehaving
    // process). Refusing to display them would be worse than substituting
    // U+FFFD.
    let text = String::from_utf8_lossy(bytes).into_owned();
    let parsed = parse_with_format(&text, format);
    LogLine {
        number: n,
        byte_offset: start as u64,
        text,
        parsed,
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use tempfile::NamedTempFile;

    use super::*;

    fn write_lines(lines: &[&str]) -> NamedTempFile {
        let mut f = NamedTempFile::new().unwrap();
        for l in lines {
            writeln!(f, "{l}").unwrap();
        }
        f.flush().unwrap();
        f
    }

    #[test]
    fn indexes_basic_lines() {
        let f = write_lines(&["alpha", "beta", "gamma"]);
        let lf = LogFile::open(f.path()).unwrap();
        assert_eq!(lf.line_count(), 3);
        assert_eq!(lf.line(0).unwrap().text, "alpha");
        assert_eq!(lf.line(2).unwrap().text, "gamma");
    }

    #[test]
    fn range_clamps_to_file_length() {
        let f = write_lines(&["a", "b", "c"]);
        let lf = LogFile::open(f.path()).unwrap();
        let rows = lf.range(0..1000);
        assert_eq!(rows.len(), 3);
    }

    #[test]
    fn resync_picks_up_appended_lines() {
        let mut f = write_lines(&["one", "two"]);
        let lf = LogFile::open(f.path()).unwrap();
        assert_eq!(lf.line_count(), 2);

        writeln!(f, "three").unwrap();
        writeln!(f, "four").unwrap();
        f.flush().unwrap();

        let added = lf.resync().unwrap();
        assert_eq!(added, 2);
        assert_eq!(lf.line_count(), 4);
        assert_eq!(lf.line(3).unwrap().text, "four");
    }

    #[test]
    fn handles_crlf_line_endings() {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(b"hello\r\nworld\r\n").unwrap();
        f.flush().unwrap();
        let lf = LogFile::open(f.path()).unwrap();
        assert_eq!(lf.line_count(), 2);
        assert_eq!(lf.line(0).unwrap().text, "hello");
        assert_eq!(lf.line(1).unwrap().text, "world");
    }

    #[test]
    fn rejects_empty_file() {
        let f = NamedTempFile::new().unwrap();
        assert!(matches!(LogFile::open(f.path()), Err(Error::EmptyFile(_))));
    }

    #[test]
    fn resync_repairs_unterminated_last_line() {
        // Initial write has no trailing newline — the last "line" is partial.
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(b"first\nsecond_par").unwrap();
        f.flush().unwrap();
        let lf = LogFile::open(f.path()).unwrap();
        assert_eq!(lf.line_count(), 2);
        assert_eq!(lf.line(1).unwrap().text, "second_par");

        // Append the rest of the second line plus a complete third line.
        f.write_all(b"tial\nthird\n").unwrap();
        f.flush().unwrap();
        lf.resync().unwrap();
        assert_eq!(lf.line_count(), 3);
        assert_eq!(lf.line(1).unwrap().text, "second_partial");
        assert_eq!(lf.line(2).unwrap().text, "third");
    }
}
