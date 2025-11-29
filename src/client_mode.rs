use crate::metrics;
use crate::vpn::VpnDevice;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use steamworks::{Client, LobbyId, SendType};

const NETMASK: &str = "255.255.255.0";

pub fn run_client(client: Client, lobby_id: LobbyId) -> Result<(), Box<dyn std::error::Error>> {
    println!("正在加入房间: {}", lobby_id.raw());

    let (tx, rx) = mpsc::channel();
    client.matchmaking().join_lobby(lobby_id, move |result| {
        let _ = tx.send(result);
    });

    loop {
        client.run_callbacks();
        if let Ok(result) = rx.try_recv() {
            match result {
                Ok(_) => {
                    println!(">>> 加入成功! <<<");
                    break;
                }
                Err(e) => {
                    println!("加入失败: {:?}", e);
                    return Ok(());
                }
            }
        }
        thread::sleep(Duration::from_millis(50));
    }

    let host_id = client.matchmaking().lobby_owner(lobby_id);
    println!("房主 Steam ID: {:?}", host_id);

    if host_id == client.user().steam_id() {
        println!("!!! 错误: 无法连接自己，请使用两个不同的账号测试 !!!");
    }

    println!("等待房主分配 IP...");

    // Send HELLO to trigger Host's peer detection
    client.networking().send_p2p_packet(host_id, SendType::Reliable, b"HELLO");

    // Wait for IP assignment
    let assigned_ip = loop {
        client.run_callbacks();
        if let Some(size) = client.networking().is_p2p_packet_available() {
            let mut buf = vec![0; size];
            if let Some((steam_id, len)) = client.networking().read_p2p_packet(&mut buf) {
                if steam_id == host_id {
                    let msg = String::from_utf8_lossy(&buf[..len]);
                    if msg.starts_with("IP:") {
                        let ip = msg[3..].to_string();
                        println!(">>> 收到 IP 分配: {} <<<", ip);
                        break ip;
                    }
                }
            }
        }
        thread::sleep(Duration::from_millis(10));
    };

    // Initialize TUN
    let vpn = VpnDevice::new(&assigned_ip, NETMASK)?;
    // VpnDevice now handles reading/writing in a background thread via channels.

    println!("VPN 已启动! 你现在的虚拟 IP 是: {}", assigned_ip);
    println!("请告诉房主你的 IP，或者直接连接房主 IP (通常是 10.10.10.1)");

    // Performance metrics
    let session_metrics = metrics::SessionMetrics::new();
    let mut last_report_time = Instant::now();

    loop {
        client.run_callbacks();

        // 1. Process TUN packets -> Send to Host
        // We read from vpn.rx (non-blocking)
        while let Ok(packet) = vpn.rx.try_recv() {
            let len = packet.len();
            client.networking().send_p2p_packet(host_id, SendType::Unreliable, &packet);
            metrics::record_packet_sent(len as u64);
        }

        // 2. Process Steam P2P packets -> Write to TUN
        while let Some(size) = client.networking().is_p2p_packet_available() {
            let mut buf = vec![0; size];
            if let Some((steam_id, len)) = client.networking().read_p2p_packet(&mut buf) {
                if steam_id != host_id { continue; } // Ignore others for now

                if len > 0 {
                    // Write to TUN via channel
                    let packet = buf[..len].to_vec();
                    if let Err(e) = vpn.tx.send(packet) {
                        println!("Error sending to TUN: {:?}", e);
                    }
                    metrics::record_packet_received(len as u64);
                }
            }
        }

        // Periodic reporting
        if last_report_time.elapsed() > Duration::from_secs(5) {
            session_metrics.print_report();
            last_report_time = Instant::now();
        }

        thread::sleep(Duration::from_millis(1));
    }
}
