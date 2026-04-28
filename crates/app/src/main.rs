// Suppress the extra console window on Windows in release builds. Has no
// effect on macOS or Linux.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    logspike_lib::run();
}
