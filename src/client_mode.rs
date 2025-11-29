use crate::metrics;
use crate::vpn::VpnDevice;
use std::collections::HashMap;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use steamworks::networking_sockets::NetConnection;
use steamworks::networking_types::{NetworkingConnectionState, NetworkingIdentity, SendFlags};
use steamworks::{Client, LobbyId, SteamId};

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

    println!("🔗 等待房主分配 IP...");

    let sockets = client.networking_sockets();
    let host_identity = NetworkingIdentity::new_steam_id(host_id);
    println!("📡 正在建立 NetworkingSockets 连接...");
    let pending_connection = sockets
        .connect_p2p(host_identity.clone(), 0, vec![])
        .map_err(|_| "无法向房主发起连接，Steam NetworkingSockets 初始化失败")?;

    let connect_deadline = Instant::now() + Duration::from_secs(15);
    loop {
        client.run_callbacks();
        if let Ok(info) = sockets.get_connection_info(&pending_connection) {
            if let Ok(state) = info.state() {
                match state {
                    NetworkingConnectionState::Connected => {
                        println!("✅ NetworkingSockets 连接已建立");
                        break;
                    }
                    NetworkingConnectionState::ClosedByPeer
                    | NetworkingConnectionState::ProblemDetectedLocally => {
                        return Err("房主拒绝或关闭了连接".into());
                    }
                    _ => {}
                }
            }
        }

        if Instant::now() > connect_deadline {
            return Err("连接房主超时".into());
        }
        thread::sleep(Duration::from_millis(50));
    }

    let mut connections: HashMap<SteamId, NetConnection> = HashMap::new();
    connections.insert(host_id, pending_connection);

    println!("👋 发送 HELLO 握手包到房主...");
    if let Some(conn) = connections.get(&host_id) {
        let _ = conn.send_message(b"HELLO", SendFlags::RELIABLE);
    }
    let mut last_hello = Instant::now();

    // Wait for IP assignment
    let assigned_ip = loop {
        client.run_callbacks();

        if last_hello.elapsed() > Duration::from_secs(1) {
            println!("🔄 正在重新尝试连接房主...");
            if let Some(conn) = connections.get(&host_id) {
                let _ = conn.send_message(b"HELLO", SendFlags::RELIABLE);
            }
            last_hello = Instant::now();
        }

        let mut newly_assigned: Option<String> = None;
        if let Some(conn) = connections.get_mut(&host_id) {
            match conn.receive_messages(32) {
                Ok(messages) => {
                    for message in messages {
                        let data = message.data();
                        if data.is_empty() {
                            continue;
                        }
                        let text = String::from_utf8_lossy(data);
                        println!("💬 收到消息: {}", text);
                        if let Some(rest) = text.strip_prefix("IP:") {
                            newly_assigned = Some(rest.to_string());
                            break;
                        }
                    }
                }
                Err(err) => {
                    println!("⚠️ 读取房主消息失败: {err:?}");
                }
            }
        }

        if let Some(ip) = newly_assigned {
            println!("✅ 收到 IP 分配: {}", ip);
            break ip;
        }

        thread::sleep(Duration::from_millis(10));
    };

    // Move connection back out for steady-state loops
    let mut host_connection = connections
        .remove(&host_id)
        .expect("host connection missing after handshake");

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

        // 1. Process TUN packets -> Send to Host (Batch processing)
        let mut packet_count = 0;
        while let Ok(packet) = vpn.rx.try_recv() {
            let len = packet.len();
            if let Err(err) = host_connection.send_message(&packet, SendFlags::UNRELIABLE_NO_NAGLE)
            {
                println!("✗ VPN 数据发送失败: {err:?}");
            } else {
                metrics::record_packet_sent(len as u64);
            }
            packet_count += 1;
            if packet_count >= 100 { break; } // Prevent starvation
        }

        // 2. Process Steam P2P packets -> Write to TUN (Batch processing)
        let mut packet_count = 0;
        match host_connection.receive_messages(64) {
            Ok(messages) => {
                for message in messages {
                    let data = message.data();
                    if data.is_empty() || data.starts_with(b"HELLO") {
                        continue;
                    }
                    if let Err(e) = vpn.tx.send(data.to_vec()) {
                        println!("Error sending to TUN: {:?}", e);
                        metrics::record_packet_dropped();
                    } else {
                        metrics::record_packet_received(data.len() as u64);
                    }
                    packet_count += 1;
                    if packet_count >= 100 {
                        break;
                    }
                }
            }
            Err(err) => {
                println!("⚠️ 无法读取来自房主的数据: {err:?}");
            }
        }

        // Periodic reporting
        if last_report_time.elapsed() > Duration::from_secs(5) {
            session_metrics.print_report();
            last_report_time = Instant::now();
        }

        thread::sleep(Duration::from_micros(100)); // 100μs for higher throughput
    }
}
