use crate::client_mode::run_client;
use crate::host::run_host;
use crate::metrics;
use crate::minecraft_discovery;
use lazy_static::lazy_static;
use log::info;
use serde::Serialize;
use std::sync::{mpsc, Mutex};
use std::thread;
use steamworks::{Client, LobbyId};
use tauri::command;

lazy_static! {
    static ref LOBBY_ID: Mutex<Option<u64>> = Mutex::new(None);
}

#[derive(Serialize)]
pub struct PerformanceMetrics {
    packets_sent: u64,
    packets_received: u64,
    bytes_sent: u64,
    bytes_received: u64,
    packets_dropped: u64,
    send_rate_mbps: f32,
    recv_rate_mbps: f32,
    send_rate_pps: f32,
    recv_rate_pps: f32,
}

#[command]
pub fn get_steam_name() -> String {
    let client = Client::init().unwrap();
    client.friends().name()
}

#[command]
pub fn get_lobby_id() -> Option<u64> {
    *LOBBY_ID.lock().unwrap()
}

#[derive(Serialize)]
pub struct MinecraftServerInfo {
    port: u16,
    motd: String,
}

#[command]
pub fn detect_minecraft_server() -> Option<MinecraftServerInfo> {
    info!("Tauri: 收到自动检测 Minecraft 服务器请求");
    
    match minecraft_discovery::discover_minecraft_server() {
        Some(server) => {
            info!("Tauri: 检测到服务器 - {} (端口: {})", server.motd, server.port);
            Some(MinecraftServerInfo {
                port: server.port,
                motd: server.motd,
            })
        }
        None => {
            info!("Tauri: 未检测到 Minecraft 服务器");
            None
        }
    }
}

#[command]
pub fn get_performance_metrics() -> PerformanceMetrics {
    let snapshot = metrics::get_snapshot();
    
    // Return absolute values - frontend will calculate deltas if needed
    let send_rate_mbps = (snapshot.bytes_sent as f32) / 1024.0 / 1024.0;
    let recv_rate_mbps = (snapshot.bytes_received as f32) / 1024.0 / 1024.0;
    let send_rate_pps = snapshot.packets_sent as f32;
    let recv_rate_pps = snapshot.packets_received as f32;

    PerformanceMetrics {
        packets_sent: snapshot.packets_sent,
        packets_received: snapshot.packets_received,
        bytes_sent: snapshot.bytes_sent,
        bytes_received: snapshot.bytes_received,
        packets_dropped: snapshot.packets_dropped,
        send_rate_mbps,
        recv_rate_mbps,
        send_rate_pps,
        recv_rate_pps,
    }
}

#[command]
pub async fn start_host(port: u16) -> Result<(), String> {
    // Create channel to receive lobby ID
    let (tx, rx) = mpsc::channel();
    
    // This runs in a separate thread to avoid blocking the UI
    thread::spawn(move || {
        let client = Client::init().unwrap();
        if let Err(e) = run_host(client, port, tx) {
            eprintln!("Host error: {}", e);
        }
    });

    // Wait for lobby ID and store it
    if let Ok(lobby_id) = rx.recv() {
        *LOBBY_ID.lock().unwrap() = Some(lobby_id);
    }
    
    Ok(())
}

#[command]
pub async fn join_lobby(lobby_id_str: String) -> Result<(), String> {
    let lobby_id_u64 = lobby_id_str
        .parse::<u64>()
        .map_err(|_| "Invalid Lobby ID")?;
    let lobby_id = LobbyId::from_raw(lobby_id_u64);

    // Store the lobby ID we're joining
    *LOBBY_ID.lock().unwrap() = Some(lobby_id_u64);

    thread::spawn(move || {
        let client = Client::init().unwrap();
        if let Err(e) = run_client(client, lobby_id) {
            eprintln!("Client error: {}", e);
        }
    });
    Ok(())
}
