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

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(
            tauri_plugin_log::Builder::default()
                .targets([
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::LogDir { file_name: None }).filter(|_| true),
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Webview),
                ])
                .build(),
        )
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
