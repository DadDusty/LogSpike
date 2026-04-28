# Contributing to logspike

Thanks for interest in logspike! This guide covers dev setup and PR expectations.

## Development Setup

Prerequisites:
- Rust 1.78+ (managed by `rust-toolchain.toml`)
- Node.js 20+
- Tauri 2 system prerequisites: https://v2.tauri.app/start/prerequisites/
- Install Tauri CLI: `cargo install tauri-cli --version "^2.0"`

### First-time setup

```bash
# Frontend dependencies
cd frontend
npm install

# Run in dev mode (Tauri starts Vite automatically)
cd ../crates/app
cargo tauri dev
```

The app opens in a native window. Hot-reload works for both Rust and frontend code.

## Testing & Linting

Before pushing, run the full suite:

```bash
# Core tests (fast)
cargo test -p logspike-core

# Lint Rust
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check

# Lint frontend
cd frontend && npm run check
```

All checks must pass. Add tests for non-trivial behavior in `crates/core/src/`.

## Building a Release

```bash
cd crates/app
cargo tauri build
```

Outputs platform-specific binaries to `src-tauri/target/release/bundle/`.

## Code Style

- **Rust:** Use `cargo fmt` — don't override the style.
- **Frontend:** Use `npm run format` for TypeScript/Svelte.
- No comments unless the *why* is non-obvious.
- Delete dead code rather than commenting it out.

## Pull Requests

- One logical change per PR.
- Commit message: imperative, lowercase ("add log level filter" not "Add Log Level Filter").
- If fixing a bug, include a test that would have caught it.
- If adding a feature, update the README roadmap if relevant.

## Architecture Notes

See `crates/core/src/` for the indexing and search core. The `crates/app/` layer is Tauri boilerplate; the real work happens in `core/`. The frontend is in `frontend/src/`.

**Key files:**
- `crates/core/src/parser.rs` — format detection and line parsing
- `crates/core/src/index.rs` — mmap + byte-offset table
- `crates/core/src/search.rs` — parallel search
- `crates/app/src/commands.rs` — IPC surface
- `frontend/src/lib/VirtualLogList.svelte` — virtualised renderer

Questions? Open an issue or reach out on GitHub.
