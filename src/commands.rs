use crate::client_mode::run_client;
use crate::host::run_host;
use std::thread;
use steamworks::{Client, LobbyId};
use tauri::command;

#[command]
pub fn get_steam_name() -> String {
    let client = Client::init().unwrap();
    client.friends().name()
}

#[command]
pub async fn start_host(port: u16) -> Result<(), String> {
    // This runs in a separate thread to avoid blocking the UI
    thread::spawn(move || {
        let client = Client::init().unwrap();
        if let Err(e) = run_host(client, port) {
            eprintln!("Host error: {}", e);
        }
    });
    Ok(())
}

#[command]
pub async fn join_lobby(lobby_id_str: String) -> Result<(), String> {
    let lobby_id_u64 = lobby_id_str
        .parse::<u64>()
        .map_err(|_| "Invalid Lobby ID")?;
    let lobby_id = LobbyId::from_raw(lobby_id_u64);

    thread::spawn(move || {
        let client = Client::init().unwrap();
        if let Err(e) = run_client(client, lobby_id) {
            eprintln!("Client error: {}", e);
        }
    });
    Ok(())
}
