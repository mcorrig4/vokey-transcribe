// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Load .env file if present (for development convenience)
    // Silently ignore if not found - production uses system env vars
    let _ = dotenvy::dotenv();

    app_lib::run();
}
