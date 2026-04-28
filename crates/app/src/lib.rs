//! Tauri shell.
//!
//! Responsibilities
//! ----------------
//! * Configure tracing.
//! * Hold the registry of open files / tailers (`AppState`).
//! * Register Tauri commands and plugins.
//! * Bridge core events into the Tauri event bus so the webview can react.

mod commands;
mod state;

use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use crate::state::AppState;

/// Entry point used by `main.rs`. Exposed as a function so tests and a
/// future mobile target can construct the same builder.
pub fn run() {
    init_tracing();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            commands::open_file,
            commands::close_file,
            commands::file_meta,
            commands::read_range,
            commands::search,
            commands::start_tail,
            commands::stop_tail,
            commands::create_session,
            commands::create_view,
            commands::view_meta,
            commands::read_view_range,
            commands::open_folder,
            commands::sort_view,
        ])
        .setup(|app| {
            // Tauri-side bookkeeping that needs the App handle goes here.
            // Currently empty; left as a hook for future menus, tray, etc.
            let _ = app.handle();
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to launch logspike");
}

fn init_tracing() {
    // RUST_LOG=logspike=debug,logspike_core=info, etc.
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_target(false).compact())
        .init();
}
