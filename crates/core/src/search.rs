//! Parallel search over an indexed log file.
//!
//! Strategy
//! --------
//! * Literal patterns (the common case) compile to an Aho-Corasick automaton,
//!   which on modern CPUs ranges from 2–5x faster than a regex DFA for
//!   single-needle searches and scales to many needles for free.
//! * Regex patterns use the `regex` crate's lazy DFA. We compile once.
//! * The byte buffer is split into roughly equal chunks at line boundaries
//!   and farmed out to `rayon`. Workers walk the offset table to translate
//!   byte hits into line numbers, so we never materialise intermediate
//!   `Vec<String>`s.
//! * `max_results` short-circuits when an atomic counter says we're done. A
//!   global LIMIT prevents accidental OOM on a "match everything" query.

use std::sync::atomic::{AtomicUsize, Ordering};

use aho_corasick::{AhoCorasickBuilder, MatchKind};
use rayon::prelude::*;
use regex::bytes::Regex as ByteRegex;
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::index::LogFile;

/// User-supplied search parameters.
#[derive(Debug, Clone, Deserialize)]
pub struct SearchOptions {
    pub pattern: String,
    #[serde(default)]
    pub case_sensitive: bool,
    /// Treat `pattern` as a regex. Defaults to literal substring search.
    #[serde(default)]
    pub regex: bool,
    /// Cap on returned matches. `None` defaults to `DEFAULT_MAX_RESULTS`.
    #[serde(default)]
    pub max_results: Option<usize>,
}

/// Default cap on returned matches. Picked to fit comfortably in a single
/// IPC payload while still being more than any human can scroll through.
pub const DEFAULT_MAX_RESULTS: usize = 100_000;

/// One match. The frontend pulls full line context separately via the
/// `range()` API so search results stay small over IPC.
#[derive(Debug, Clone, Serialize)]
pub struct Match {
    pub line_number: u64,
    pub byte_offset: u64,
    pub match_start: u32,
    pub match_end: u32,
}

/// Stateless façade — `run()` owns the lifetime of the compiled matcher.
#[derive(Debug, Default)]
pub struct Searcher;

impl Searcher {
    pub fn run(file: &LogFile, opts: SearchOptions) -> Result<Vec<Match>> {
        let limit = opts.max_results.unwrap_or(DEFAULT_MAX_RESULTS);

        if opts.regex {
            run_regex(file, &opts.pattern, opts.case_sensitive, limit)
        } else {
            run_literal(file, &opts.pattern, opts.case_sensitive, limit)
        }
    }
}

fn run_literal(file: &LogFile, pattern: &str, case_sensitive: bool, limit: usize) -> Result<Vec<Match>> {
    let ac = AhoCorasickBuilder::new()
        .ascii_case_insensitive(!case_sensitive)
        .match_kind(MatchKind::LeftmostFirst)
        .build([pattern])
        .expect("aho-corasick build");

    file.with_bytes(|bytes, offsets| {
        Ok(parallel_search(bytes, offsets, limit, |chunk_bytes, chunk_byte_start| {
            ac.find_iter(chunk_bytes)
                .map(move |m| (chunk_byte_start + m.start() as u64, m.len() as u32))
                .collect()
        }))
    })
}

fn run_regex(file: &LogFile, pattern: &str, case_sensitive: bool, limit: usize) -> Result<Vec<Match>> {
    let pattern_full = if case_sensitive {
        pattern.to_owned()
    } else {
        format!("(?i){pattern}")
    };
    let re = ByteRegex::new(&pattern_full)?;

    file.with_bytes(|bytes, offsets| {
        Ok(parallel_search(bytes, offsets, limit, |chunk_bytes, chunk_byte_start| {
            re.find_iter(chunk_bytes)
                .map(move |m| (chunk_byte_start + m.start() as u64, (m.end() - m.start()) as u32))
                .collect()
        }))
    })
}

/// Split the file into chunks at line boundaries and let `find_in_chunk` do
/// the per-chunk work. Returns globally sorted matches.
///
/// `find_in_chunk(chunk_bytes, chunk_byte_start) -> Vec<(absolute_byte, len)>`
fn parallel_search<F>(bytes: &[u8], offsets: &[u64], limit: usize, find_in_chunk: F) -> Vec<Match>
where
    F: Fn(&[u8], u64) -> Vec<(u64, u32)> + Sync,
{
    // ~1 MiB per chunk balances per-task overhead against load distribution.
    const TARGET_CHUNK_BYTES: u64 = 1 << 20;

    let total_bytes = *offsets.last().unwrap_or(&0);
    if total_bytes == 0 {
        return Vec::new();
    }

    let total_lines = offsets.len().saturating_sub(1);
    if total_lines == 0 {
        return Vec::new();
    }

    // Build chunk boundaries that align to line starts.
    let mut chunk_line_bounds: Vec<(usize, usize)> = Vec::new();
    let mut start_line = 0usize;
    while start_line < total_lines {
        let start_byte = offsets[start_line];
        // Find a line whose start is at least TARGET_CHUNK_BYTES past us.
        let target = start_byte + TARGET_CHUNK_BYTES;
        // Binary search the offset table.
        let end_line = match offsets[start_line..].binary_search(&target) {
            Ok(idx) => start_line + idx,
            Err(idx) => start_line + idx,
        }
        .min(total_lines);
        let end_line = if end_line == start_line { start_line + 1 } else { end_line };
        chunk_line_bounds.push((start_line, end_line));
        start_line = end_line;
    }

    let hit_count = AtomicUsize::new(0);

    let mut chunk_results: Vec<Vec<Match>> = chunk_line_bounds
        .par_iter()
        .map(|&(line_start, line_end)| {
            if hit_count.load(Ordering::Relaxed) >= limit {
                return Vec::new();
            }
            let byte_start = offsets[line_start];
            let byte_end = offsets[line_end];
            let chunk = &bytes[byte_start as usize..byte_end as usize];
            let raw = find_in_chunk(chunk, byte_start);

            // Translate byte hits to (line_number, offset_in_line).
            let mut out: Vec<Match> = Vec::with_capacity(raw.len());
            // Maintain a cursor into the offsets table to avoid repeated
            // binary searches when matches cluster in the same line.
            let mut cursor = line_start;
            for (abs_byte, len) in raw {
                while cursor + 1 < offsets.len() && offsets[cursor + 1] <= abs_byte {
                    cursor += 1;
                }
                let line_byte_start = offsets[cursor];
                out.push(Match {
                    line_number: cursor as u64,
                    byte_offset: line_byte_start,
                    match_start: (abs_byte - line_byte_start) as u32,
                    match_end: (abs_byte - line_byte_start) as u32 + len,
                });
                if hit_count.fetch_add(1, Ordering::Relaxed) + 1 >= limit {
                    break;
                }
            }
            out
        })
        .collect();

    // Stitch and clip. Chunks were processed in order so each sub-vec is
    // already sorted, but global ordering can be skewed by parallelism — so
    // we sort by line number before clipping.
    let mut all: Vec<Match> = chunk_results.iter_mut().flat_map(std::mem::take).collect();
    all.sort_by_key(|m| (m.line_number, m.match_start));
    all.truncate(limit);
    all
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use tempfile::NamedTempFile;

    use super::*;
    use crate::index::LogFile;

    fn write_lines(lines: &[&str]) -> NamedTempFile {
        let mut f = NamedTempFile::new().unwrap();
        for l in lines {
            writeln!(f, "{l}").unwrap();
        }
        f.flush().unwrap();
        f
    }

    #[test]
    fn finds_literal_matches() {
        let f = write_lines(&["alpha apple", "beta", "gamma apple pie"]);
        let lf = LogFile::open(f.path()).unwrap();
        let hits = Searcher::run(
            &lf,
            SearchOptions {
                pattern: "apple".into(),
                case_sensitive: true,
                regex: false,
                max_results: None,
            },
        )
        .unwrap();
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].line_number, 0);
        assert_eq!(hits[1].line_number, 2);
    }

    #[test]
    fn case_insensitive_literal() {
        let f = write_lines(&["ERROR boom", "info ok", "Error again"]);
        let lf = LogFile::open(f.path()).unwrap();
        let hits = Searcher::run(
            &lf,
            SearchOptions {
                pattern: "error".into(),
                case_sensitive: false,
                regex: false,
                max_results: None,
            },
        )
        .unwrap();
        assert_eq!(hits.len(), 2);
    }

    #[test]
    fn regex_match() {
        let f = write_lines(&["id=42 ok", "id=7 fail", "id=999 ok"]);
        let lf = LogFile::open(f.path()).unwrap();
        let hits = Searcher::run(
            &lf,
            SearchOptions {
                pattern: r"id=\d{2,}".into(),
                case_sensitive: true,
                regex: true,
                max_results: None,
            },
        )
        .unwrap();
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].line_number, 0);
        assert_eq!(hits[1].line_number, 2);
    }

    #[test]
    fn respects_max_results() {
        let mut lines: Vec<String> = (0..10_000).map(|i| format!("hit_{i} hit")).collect();
        lines.push("tail".into());
        let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
        let f = write_lines(&refs);
        let lf = LogFile::open(f.path()).unwrap();
        let hits = Searcher::run(
            &lf,
            SearchOptions {
                pattern: "hit".into(),
                case_sensitive: true,
                regex: false,
                max_results: Some(50),
            },
        )
        .unwrap();
        assert_eq!(hits.len(), 50);
    }
}
