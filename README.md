# LogSpike

A fast, modern, multi-platform log viewer in the spirit of CMTrace. Built on Rust + Tauri 2 with a Svelte 5 frontend.

**Author:** [DadDusty](https://github.com/DadDusty)

## Why this stack

A log viewer's perceived speed is dominated by three things: how fast you can ingest bytes, how fast you can search and filter them, and how smoothly you can scroll through millions of lines. The stack is chosen around those constraints.

- **Rust core.** Memory-mapped file I/O via `memmap2`, SIMD-accelerated newline scanning via `memchr` (~5–10 GiB/s on modern CPUs), Aho-Corasick literal search, regex search via the `regex` crate, parallel scans via `rayon`, and live tailing via `notify`. No GC pauses, predictable memory.
- **Tauri 2 shell.** ~5–10 MiB binaries, native OS webview (WebKit / WebView2), no bundled Chromium, real native menus and dialogs. The Rust side does the heavy lifting; the webview just renders.
- **Svelte 5 + TypeScript frontend.** Smallest runtime in the modern UI ecosystem, ideal for the windowed/virtualised list that drives the whole UX.

## Project layout

```
logspike/
├── Cargo.toml              # Workspace manifest, shared dep versions, release profile
├── rust-toolchain.toml     # Pinned compiler version
├── rustfmt.toml / clippy.toml
├── crates/
│   ├── core/               # Pure Rust: indexing, search, tail. No Tauri dep.
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── error.rs
│   │       ├── index.rs    # mmap + offset table
│   │       ├── parser.rs   # Best-effort level/timestamp detection
│   │       ├── search.rs   # Parallel literal + regex search
│   │       └── tail.rs     # FS watcher → growth events
│   └── app/                # Tauri shell
│       ├── Cargo.toml
│       ├── tauri.conf.json
│       ├── build.rs
│       ├── capabilities/default.json
│       ├── icons/          # Drop platform icons here (see `tauri icon`)
│       └── src/
│           ├── main.rs
│           ├── lib.rs      # Builder, plugins, tracing
│           ├── state.rs    # File registry (DashMap)
│           └── commands.rs # Tauri command surface
└── frontend/               # Svelte 5 + TypeScript
    ├── package.json
    ├── vite.config.ts
    ├── svelte.config.js
    ├── tsconfig.json / tsconfig.node.json
    ├── index.html
    └── src/
        ├── main.ts
        ├── app.css
        ├── App.svelte
        └── lib/
            ├── api.ts            # invoke() wrappers
            ├── types.ts          # Mirrors of Rust types
            ├── lineCache.ts      # LRU chunk cache
            ├── VirtualLogList.svelte
            └── Toolbar.svelte
```

The split between `crates/core` and `crates/app` is deliberate. Core has no Tauri dependency, so it compiles fast, unit-tests in milliseconds, and can be reused later from a CLI or a different shell (mobile, server-side log indexer, etc.).

## Scaling decisions worth knowing

The choices that matter most as files grow into the multi-GiB range:

1. **Lines are indexed as a byte-offset table, not as `String`s.** ~8 bytes per line regardless of line length. A 10 M-line file costs ~80 MiB of index, not gigabytes of heap.
2. **Mmap, not `read_to_string`.** The kernel pages cold bytes in only when touched. Opening a 10 GiB file is effectively instantaneous.
3. **`memchr::memchr_iter` for newline scanning.** Dispatches to AVX2 / NEON at runtime; this is the loop that sets the indexing throughput ceiling.
4. **Parallel search with chunk-aligned line boundaries.** `rayon` parallelises across ~1 MiB chunks so search scales near-linearly with cores. An atomic short-circuits when `max_results` is reached.
5. **Aho-Corasick for literal patterns, regex DFA for the rest.** Literal substring is the common case and AC beats a regex DFA by 2–5x for single needles. We pick the right tool per query automatically.
6. **CPU-bound work runs on `tokio::task::spawn_blocking`.** The Tauri runtime never stalls. State lookups are O(1) on a `DashMap`.
7. **Live tail does incremental indexing.** Each filesystem event re-stats the file, re-mmaps if it grew, and only scans the new bytes. Single-digit microseconds per tick on multi-MB-per-second log streams.
8. **The frontend renders only the visible viewport.** A `LineCache` (LRU, chunked at 256 lines) absorbs scroll churn; range fetches are debounced into one IPC round-trip per pause.
9. **The virtual spacer is clamped at 30 M px.** Browsers cap absolute element heights around 33 M; past that, scrollTop is remapped linearly to line index. Same approach Klogg / LogExpert use for huge files.
10. **Release profile uses fat LTO + single codegen unit + `panic = "abort"` + symbol stripping.** Costs build time, buys binary size and runtime speed.

## Development

Prerequisites:

- Rust 1.81+ (managed by `rust-toolchain.toml`).
- Node.js 20+ (for the frontend dev server).
- Tauri 2 system prerequisites: <https://v2.tauri.app/start/prerequisites/>.
- The `tauri-cli`: `cargo install tauri-cli --version "^2.0"`.

Install frontend deps:

```sh
cd frontend
npm install
```

Run the desktop app in dev mode (Tauri starts the Vite dev server itself):

```sh
cd crates/app
cargo tauri dev
```

Run the core test suite:

```sh
cargo test -p logspike-core
```

Lint:

```sh
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
( cd frontend && npm run check )
```

Build a release bundle (writes `.app` / `.dmg` / `.msi` / `.AppImage` per host):

```sh
cd crates/app
cargo tauri build
```

Generate icons from a 1024x1024 PNG into `crates/app/icons/`:

```sh
cargo tauri icon path/to/source.png
```

## Roadmap

The scaffold gets you to a working viewer with file-open, virtualised render, level coloring, regex search, live tail, search navigation with keyboard cycling, and bookmarks. Reasonable next steps in rough priority order:

- Multi-tab support (the `AppState` already keys files by id, the UI just needs tabs).
- Multi-file view with source column showing which file each line came from.
- Saved filter presets and colour rules per format.
- Multi-file tail with merge-by-timestamp.
- "Jump to time" navigation.
- Pluggable parsers for application-specific log formats (CMTrace XML-ish, JSON-per-line, logfmt, syslog, IIS, etc.).
- Mobile target via Tauri 2's iOS/Android pipeline.

## License

Licensed under the MIT License. See [LICENSE](LICENSE) for details.

Copyright © 2026 DadDusty. You're free to use, modify, and distribute this software — just keep the author attribution.
