use crate::metrics;
use crate::vpn::VpnDevice;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use steamworks::networking_sockets::NetConnection;
use steamworks::networking_types::{ListenSocketEvent, SendFlags};
use steamworks::{Client, LobbyType, SteamId};

static RUNNING: AtomicBool = AtomicBool::new(true);

// Virtual IP configuration
const HOST_IP: &str = "10.10.10.1";
const NETMASK: &str = "255.255.255.0";

struct PeerState {
    connection: NetConnection,
    virtual_ip: String,
}

pub fn run_host(client: Client, _port: u16) -> Result<(), Box<dyn std::error::Error>> {
    println!("🏗 正在创建 Steam 大厅...");

    // Create channel to receive lobby creation result
    let (tx, rx) = mpsc::channel();
    client.matchmaking().create_lobby(LobbyType::Public, 10, move |result| {
        let _ = tx.send(result);
    });

    // Wait for lobby creation result
    let _lobby_id = loop {
        client.run_callbacks();
        if let Ok(result) = rx.try_recv() {
            match result {
                Ok(id) => {
                    println!("┌─────────────────────────────────────");
                    println!("│ ✓ 房间创建成功!");
                    println!("│ 房间 ID: {}", id.raw());
                    println!("│ 好友可通过此 ID 加入游戏");
                    println!("│ 虚拟 IP: {}", HOST_IP);
                    println!("└─────────────────────────────────────");
                    break id;
                }
                Err(e) => {
                    return Err(format!("✗ 房间创建失败: {:?}", e).into());
                }
            }
        }
        thread::sleep(Duration::from_millis(10));
    };

    // Initialize TUN device (lobby is confirmed created)
    println!("🔧 正在初始化 VPN 设备...");
    let vpn = VpnDevice::new(HOST_IP, NETMASK)?;

    // Peer management: SteamId -> NetConnection + Virtual IP
    let listen_socket = client
        .networking_sockets()
        .create_listen_socket_p2p(0, vec![])
        .map_err(|err| format!("无法创建 Steam NetworkingSockets 监听端口: {err:?}"))?;
    println!("📡 NetworkingSockets 监听已启动 (虚拟端口 0)");

    let mut peers: HashMap<SteamId, PeerState> = HashMap::new();
    let mut next_ip_octet = 2u8;

    println!("VPN 服务已启动，等待玩家加入...");

    // Performance metrics
    let session_metrics = metrics::SessionMetrics::new();
    let mut last_report_time = Instant::now();

    println!("🔄 开始主循环，监听 NetworkingSockets 事件...");

    while RUNNING.load(Ordering::Relaxed) {
        client.run_callbacks();

        // Handle listen socket events first so connections are ready before data flows
        while let Some(event) = listen_socket.try_receive_event() {
            println!("📥 收到 ListenSocket 事件");
            match event {
                ListenSocketEvent::Connecting(request) => {
                    let remote = request.remote();
                    println!("🔔 收到 NetworkingSockets 连接请求: {}", remote.debug_string());
                    if let Err(err) = request.accept() {
                        println!("✗ 无法接受连接: {err:?}");
                    } else {
                        println!("✓ 连接请求已接受，等待 Connected 事件...");
                    }
                }
                ListenSocketEvent::Connected(connected) => {
                    let remote = connected.remote();
                    if let Some(steam_id) = remote.steam_id() {
                        if next_ip_octet >= 255 {
                            println!("⚠️ 虚拟网段地址已耗尽，拒绝 {}", remote.debug_string());
                            continue;
                        }
                        let peer_ip = format!("10.10.10.{}", next_ip_octet);
                        next_ip_octet = next_ip_octet.wrapping_add(1);

                        let connection = connected.take_connection();
                        peers.insert(
                            steam_id,
                            PeerState {
                                connection,
                                virtual_ip: peer_ip.clone(),
                            },
                        );

                        println!("┌─────────────────────────────────────");
                        println!("│ [新玩家] Steam ID: {:?}", steam_id);
                        println!("│ 分配 IP: {}", peer_ip);
                        println!("└─────────────────────────────────────");

                        // Send IP assignment
                        if let Some(peer) = peers.get(&steam_id) {
                            let hello_msg = format!("IP:{}", peer.virtual_ip);
                            if let Err(err) = peer
                                .connection
                                .send_message(hello_msg.as_bytes(), SendFlags::RELIABLE)
                            {
                                println!("✗ 发送 IP 分配信息失败: {err:?}");
                            } else {
                                println!("✓ 已发送 IP 分配给 {:?}", steam_id);
                            }
                        }
                    } else {
                        println!(
                            "⚠️ 收到未知身份连接，无法映射 Steam ID: {}",
                            connected.remote().debug_string()
                        );
                    }
                }
                ListenSocketEvent::Disconnected(disconnected) => {
                    if let Some(steam_id) = disconnected.remote().steam_id() {
                        peers.remove(&steam_id);
                        println!("👋 玩家离开: {:?}", steam_id);
                    }
                }
            }
        }

        // 1. Process TUN packets -> Send to Peers (Batch processing)
        let mut packet_count = 0u32;
        while let Ok(packet) = vpn.rx.try_recv() {
            let len = packet.len();
            // Basic routing logic: broadcast to all connected peers
            for peer in peers.values() {
                if let Err(err) = peer
                    .connection
                    .send_message(&packet, SendFlags::UNRELIABLE_NO_NAGLE)
                {
                    println!("✗ VPN 数据发送失败: {err:?}");
                } else {
                    metrics::record_packet_sent(len as u64);
                }
            }
            packet_count += 1;
            if packet_count >= 100 {
                break;
            }
        }

        // 2. Process Steam P2P packets -> Write to TUN (Batch processing)
        let mut packet_count = 0u32;
        for peer in peers.values_mut() {
            match peer.connection.receive_messages(64) {
                Ok(messages) => {
                    for message in messages {
                        let data = message.data();
                        if data.is_empty() {
                            continue;
                        }
                        // Ignore handshake markers
                        if data.starts_with(b"HELLO") {
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
                    println!("⚠️ 无法读取来自客户端的数据: {err:?}");
                }
            }
            if packet_count >= 100 {
                break;
            }
        }

        // Periodic reporting
        if last_report_time.elapsed() > Duration::from_secs(5) {
            session_metrics.print_report();
            last_report_time = Instant::now();
        }

        thread::sleep(Duration::from_micros(100)); // 100μs for higher throughput
    }

    Ok(())
}
