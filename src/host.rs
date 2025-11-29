use crate::metrics;
use crate::vpn::VpnDevice;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};
use steamworks::{Client, LobbyType, SteamId, SendType};

static RUNNING: AtomicBool = AtomicBool::new(true);

// Virtual IP configuration
const HOST_IP: &str = "10.10.10.1";
const NETMASK: &str = "255.255.255.0";

pub fn run_host(client: Client, _port: u16) -> Result<(), Box<dyn std::error::Error>> {
    println!(" 正在创建 Steam 大厅...");

    client.matchmaking().create_lobby(LobbyType::Public, 10, |result| {
        match result {
            Ok(lobby_id) => {
                println!("┌─────────────────────────────────────");
                println!("│ ✓ 房间创建成功!");
                println!("│ 房间 ID: {}", lobby_id.raw());
                println!("│ 好友可通过此 ID 加入游戏");
                println!("│ 虚拟 IP: {}", HOST_IP);
                println!("└─────────────────────────────────────");
            }
            Err(e) => println!("✗ 房间创建失败: {:?}", e),
        }
    });

    // Initialize TUN device
    let vpn = VpnDevice::new(HOST_IP, NETMASK)?;
    // VpnDevice now handles reading/writing in a background thread via channels.
    // vpn.rx: Packets FROM TUN -> We send to Steam
    // vpn.tx: Packets TO TUN <- We get from Steam

    // Peer management: SteamId -> Virtual IP
    let mut peers: HashMap<SteamId, String> = HashMap::new();
    let mut next_ip_octet = 2;

    println!(" VPN 服务已启动，等待玩家加入...");

    // Performance metrics
    let session_metrics = metrics::SessionMetrics::new();
    let mut last_report_time = Instant::now();

    while RUNNING.load(Ordering::Relaxed) {
        client.run_callbacks();

        // 1. Process TUN packets -> Send to Peers
        // We read from vpn.rx (non-blocking try_recv)
        while let Ok(packet) = vpn.rx.try_recv() {
             let len = packet.len();
             // Basic routing logic
             // TODO: Real routing. For now, broadcast to all clients.
             for peer_id in peers.keys() {
                 client.networking().send_p2p_packet(*peer_id, SendType::Unreliable, &packet);
                 metrics::record_packet_sent(len as u64);
             }
        }

        // 2. Process Steam P2P packets -> Write to TUN
        while let Some(size) = client.networking().is_p2p_packet_available() {
            let mut buf = vec![0; size];
            if let Some((steam_id, len)) = client.networking().read_p2p_packet(&mut buf) {
                if len == 0 { continue; }

                // Handle new peers
                if !peers.contains_key(&steam_id) {
                    let peer_ip = format!("10.10.10.{}", next_ip_octet);
                    next_ip_octet += 1;
                    peers.insert(steam_id, peer_ip.clone());
                    
                    println!("┌─────────────────────────────────────");
                    println!("│ [新玩家] Steam ID: {:?}", steam_id);
                    println!("│ 分配 IP: {}", peer_ip);
                    println!("└─────────────────────────────────────");

                    let hello_msg = format!("IP:{}", peer_ip);
                    client.networking().send_p2p_packet(steam_id, SendType::Reliable, hello_msg.as_bytes());
                }

                if buf.len() > 0 {
                    // Write to TUN via channel
                    // We only send the actual data slice
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

    Ok(())
}
