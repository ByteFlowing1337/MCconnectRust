use crate::lan_discovery::LanBroadcaster;
use crate::metrics;
use std::io::{ErrorKind, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};
use steamworks::networking_types::{NetworkingConnectionState, NetworkingIdentity, SendFlags};
use steamworks::{Client, LobbyId};

const BUFFER_SIZE: usize = 4096;
const CLIENT_LISTEN_PORT: u16 = 25565;
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
        return Err("无法连接自己".into());
    }

    // 使用新版 NetworkingSockets API 连接房主
    println!("📡 正在建立 NetworkingSockets 连接...");
    let sockets = client.networking_sockets();
    let host_identity = NetworkingIdentity::new_steam_id(host_id);
    
    let mut connection = sockets
        .connect_p2p(host_identity, 0, vec![])
        .map_err(|_| "无法向房主发起连接，Steam NetworkingSockets 初始化失败")?;

    // 等待连接建立
    let connect_deadline = Instant::now() + Duration::from_secs(15);
    loop {
        client.run_callbacks();
        if let Ok(info) = sockets.get_connection_info(&connection) {
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

    // 启动本地监听
    let listener = TcpListener::bind(format!("0.0.0.0:{}", CLIENT_LISTEN_PORT))?;
    listener.set_nonblocking(true)?;
    println!(">>> 请在 Minecraft 中连接: 127.0.0.1:{}", CLIENT_LISTEN_PORT);

    // 启动LAN发现广播
    let broadcaster = LanBroadcaster::new(None, CLIENT_LISTEN_PORT)?;
    let _broadcast_handle = broadcaster.start();
    println!("✓ Minecraft LAN发现广播已启动");

    println!("");
    println!("┌─────────────────────────────────────────────────────────┐");
    println!("│  ✅ 已连接到房主!                                       │");
    println!("├─────────────────────────────────────────────────────────┤");
    println!("│  🎮 Minecraft 连接方式:                                 │");
    println!("│     多人游戏 -> 添加服务器 -> 输入: 127.0.0.1:{}    │", CLIENT_LISTEN_PORT);
    println!("└─────────────────────────────────────────────────────────┘");
    println!("");

    // Channel: MC读取线程 -> 主循环 (发送到Steam)
    let (from_mc_tx, from_mc_rx): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = mpsc::channel();
    
    let mut mc_stream: Option<TcpStream> = None;
    let mut mc_read_thread_started = false;

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

    println!("");
    println!("┌─────────────────────────────────────────────────────────┐");
    println!("│  ✅ VPN 连接成功!                                       │");
    println!("├─────────────────────────────────────────────────────────┤");
    println!("│  你的虚拟 IP: {}                              │", assigned_ip);
    println!("│  房主虚拟 IP: 10.10.10.1                                │");
    println!("├─────────────────────────────────────────────────────────┤");
    println!("│  🎮 Minecraft 连接方式:                                 │");
    println!("│     多人游戏 -> 添加服务器 -> 输入: 10.10.10.1:25565    │");
    println!("│     (如果房主使用其他端口，请替换 25565)                │");
    println!("└─────────────────────────────────────────────────────────┘");
    println!("");

    // Performance metrics
    let session_metrics = metrics::SessionMetrics::new();
    let mut last_report_time = Instant::now();
    let mut send_failures = 0u32;
    const MAX_FAILURES: u32 = 50;
    let mut last_connection_check = Instant::now();

    loop {
        client.run_callbacks();

        // Check connection state periodically (every 100ms)
        if last_connection_check.elapsed() > Duration::from_millis(100) {
            if let Ok(info) = sockets.get_connection_info(&host_connection) {
                if let Ok(state) = info.state() {
                    match state {
                        NetworkingConnectionState::Connected => {
                            // Connection is healthy, reset failure counter
                            if send_failures > 0 {
                                send_failures = 0;
                                println!("✅ 连接已恢复");
                            }
                        }
                        NetworkingConnectionState::ClosedByPeer
                        | NetworkingConnectionState::ProblemDetectedLocally => {
                            println!("⚠️ 连接已断开: {:?}", state);
                            return Err("Steam P2P 连接中断".into());
                        }
                        NetworkingConnectionState::Connecting => {
                            println!("🔄 正在重新连接...");
                        }
                        _ => {}
                    }
                }
            }
            last_connection_check = Instant::now();
        }

        // 1. Process TUN packets -> Send to Host (Batch processing)
        let mut packet_count = 0;
        while let Ok(packet) = vpn.rx.try_recv() {
            let len = packet.len();
            
            // Only send if we haven't hit max failures
            if send_failures < MAX_FAILURES {
                match host_connection.send_message(&packet, SendFlags::UNRELIABLE_NO_NAGLE) {
                    Ok(_) => {
                        metrics::record_packet_sent(len as u64);
                        send_failures = 0; // Reset on success
                    }
                    Err(err) => {
                        send_failures += 1;
                        if send_failures == 1 || send_failures % 10 == 0 {
                            println!("✗ VPN 数据发送失败 ({}/{}): {err:?}", send_failures, MAX_FAILURES);
                        }
                        if send_failures >= MAX_FAILURES {
                            println!("❌ 连接失败次数过多，停止发送数据包");
                        }
                    }
                }
            } else {
                // Drop packets silently when connection is bad
                metrics::record_packet_dropped();
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

        // 检查是否有新的 MC 客户端连接
        if mc_stream.is_none() {
            match listener.accept() {
                Ok((stream, addr)) => {
                    println!("┌─────────────────────────────────────");
                    println!("│ [连接] MC 客户端已连接: {}", addr);
                    println!("└─────────────────────────────────────");
                    
                    stream.set_nodelay(true)?;
                    
                    // 启动 MC -> Steam 读取线程
                    if !mc_read_thread_started {
                        let mut read_stream = stream.try_clone()?;
                        let from_mc_tx_clone = from_mc_tx.clone();
                        thread::spawn(move || {
                            let mut buffer = [0u8; BUFFER_SIZE];
                            loop {
                                match read_stream.read(&mut buffer) {
                                    Ok(0) => {
                                        println!("[读取线程] MC 客户端断开连接");
                                        break;
                                    }
                                    Ok(n) => {
                                        if from_mc_tx_clone.send(buffer[..n].to_vec()).is_err() {
                                            break;
                                        }
                                    }
                                    Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                                        thread::sleep(Duration::from_micros(100));
                                    }
                                    Err(e) => {
                                        println!("✗ 读取 MC 失败: {:?}", e);
                                        break;
                                    }
                                }
                            }
                        });
                        mc_read_thread_started = true;
                    }

                    mc_stream = Some(stream);
                }
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => {}
                Err(e) => {
                    println!("等待 MC 连接时发生错误: {:?}", e);
                }
            }
        }

        // 从 MC 读取数据 -> 发送到 Steam
        while let Ok(data) = from_mc_rx.try_recv() {
            match connection.send_message(&data, SendFlags::RELIABLE_NO_NAGLE) {
                Ok(_) => {
                    metrics::record_packet_sent(data.len() as u64);
                }
                Err(err) => {
                    println!("✗ 发送到房主失败: {:?}", err);
                    metrics::record_packet_dropped();
                }
            }
        }

        // 从 Steam 接收数据 -> 写入 MC
        match connection.receive_messages(64) {
            Ok(messages) => {
                for message in messages {
                    let data = message.data();
                    if data.is_empty() {
                        continue;
                    }
                    metrics::record_packet_received(data.len() as u64);
                    
                    // 直接写入 MC stream
                    if let Some(ref mut stream) = mc_stream {
                        if let Err(e) = stream.write_all(data) {
                            println!("✗ 写入 MC 失败: {:?}", e);
                            mc_stream = None;
                        }
                    }
                }
            }
            Err(err) => {
                println!("⚠️ 从房主接收数据失败: {:?}", err);
            }
        }

        thread::sleep(Duration::from_micros(100));
    }
}
