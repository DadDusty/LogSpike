//! logspike-core
//!
//! Pure-Rust log indexing, searching, and tailing primitives. Deliberately has
//! no Tauri dependency so it can be unit-tested in isolation, reused from a
//! CLI, or embedded in a different shell later.
//!
//! Scaling design notes
//! --------------------
//! * Files are memory-mapped (`memmap2::Mmap`) instead of read into a `Vec<u8>`.
//!   Cold pages are paged in by the kernel only when actually touched, so
//!   opening a 10 GiB file is effectively free.
//! * We index lines as a `Vec<u64>` of byte offsets to the start of each line,
//!   not as a `Vec<String>`. Memory cost is ~8 bytes per line regardless of
//!   line length (a 10 M line file costs ~80 MiB of index, not gigabytes).
//! * Newline scanning uses `memchr::memchr_iter`, which dispatches to AVX2 /
//!   NEON at runtime. On modern CPUs we index ~5–10 GiB/s of plain ASCII.
//! * Search supports both literal (Aho-Corasick) and regex paths. Both are
//!   parallelised with `rayon` over coarse-grained line chunks; the byte-offset
//!   index makes random access O(1) so chunking is trivial.
//! * Live tail is built on `notify` plus an incremental indexer that only
//!   scans bytes appended since the last tick — a 1 GB/min log stream costs
//!   single-digit microseconds per indexer pass.
//!
//! Public API surface is intentionally narrow; the Tauri shell wraps these
//! types behind async commands.

#![warn(missing_debug_implementations, rust_2018_idioms, unreachable_pub)]
#![deny(unsafe_op_in_unsafe_fn)]

pub mod error;
pub mod index;
pub mod parser;
pub mod search;
pub mod tail;

pub use error::{Error, Result};
pub use index::{LineRange, LogFile, LogLine, Session, View, SessionLine};
pub use parser::{LogLevel, ParsedLine, FileFormat};
pub use search::{Match, SearchOptions, Searcher};
pub use tail::{TailEvent, Tailer};
