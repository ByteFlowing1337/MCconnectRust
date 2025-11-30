#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod callbacks;
mod client_mode;
mod commands;
mod config;
mod host;
mod lan_discovery;
mod metrics;
mod minecraft_discovery;

use env_logger::Env;
use std::io::Write;
use std::sync::Once;

fn main() {
    init_logging();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_steam_name,
            commands::get_lobby_id,
            commands::get_performance_metrics,
            commands::detect_minecraft_server,
            commands::start_host,
            commands::join_lobby
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn init_logging() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        env_logger::Builder::from_env(Env::default().default_filter_or("info"))
            .format(|buf, record| writeln!(buf, "[{:<5}] {}", record.level(), record.args()))
            .init();
    });
}
