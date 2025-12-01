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
    latency_ms: Option<u32>,
}

#[command]
pub fn get_steam_name() -> Result<String, String> {
    match Client::init() {
        Ok(client) => Ok(client.friends().name()),
        Err(e) => Err(format!("Steam 未运行或初始化失败: {}", e)),
    }
}

#[command]
pub fn get_lobby_id() -> Option<u64> {
    *LOBBY_ID.lock().unwrap()
}

#[command]
pub async fn detect_minecraft_server() -> Option<minecraft_discovery::MinecraftServer> {
    info!("Tauri: 收到自动检测 Minecraft 服务器请求");

    // 使用 spawn_blocking 在单独的线程中运行阻塞操作，避免阻塞 Tauri 主线程
    let result = tauri::async_runtime::spawn_blocking(|| {
        minecraft_discovery::discover_minecraft_server()
    })
    .await
    .ok()
    .flatten();

    match result {
        Some(server) => {
            info!(
                "Tauri: 检测到服务器 - {} ({}:{}) at {:.2}ms",
                server.motd, server.ip, server.port, server.latency_ms
            );
            Some(server)
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

    // 获取延迟信息（如果有多个连接，返回第一个）
    let latency_ms = metrics::get_all_latencies()
        .values()
        .next()
        .copied();

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
        latency_ms,
    }
}

#[command]
pub async fn start_host(port: u16, password: Option<String>) -> Result<(), String> {
    // Create channel to receive lobby ID
    let (tx, rx) = mpsc::channel();
    
    // This runs in a separate thread to avoid blocking the UI
    thread::spawn(move || {
        match Client::init() {
            Ok(client) => {
                if let Err(e) = run_host(client, port, password, tx) {
                    eprintln!("Host error: {}", e);
                }
            }
            Err(e) => {
                eprintln!("Steam 初始化失败: {}", e);
            }
        }
    });

    // Wait for lobby ID and store it
    if let Ok(lobby_id) = rx.recv() {
        *LOBBY_ID.lock().unwrap() = Some(lobby_id);
    }
    
    Ok(())
}

#[command]
pub async fn join_lobby(lobby_id_str: String, password: Option<String>) -> Result<(), String> {
    let lobby_id_u64 = lobby_id_str
        .parse::<u64>()
        .map_err(|_| "Invalid Lobby ID")?;
    let lobby_id = LobbyId::from_raw(lobby_id_u64);

    // Create channel to receive connection result
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        match Client::init() {
            Ok(client) => {
                match run_client(client, lobby_id, password, tx.clone()) {
                    Ok(_) => {}
                    Err(e) => {
                        eprintln!("Client error: {}", e);
                        let _ = tx.send(Err(format!("客户端错误: {}", e)));
                    }
                }
            }
            Err(e) => {
                eprintln!("Steam 初始化失败: {}", e);
                let _ = tx.send(Err(format!("Steam 未运行或初始化失败: {}", e)));
            }
        }
    });

    // Wait for connection result (success or error)
    match rx.recv_timeout(std::time::Duration::from_secs(30)) {
        Ok(Ok(())) => {
            // Store the lobby ID after successful connection
            *LOBBY_ID.lock().unwrap() = Some(lobby_id_u64);
            Ok(())
        }
        Ok(Err(e)) => Err(e),
        Err(_) => Err("连接超时".to_string()),
    }
}
