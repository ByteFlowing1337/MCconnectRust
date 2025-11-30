use crate::config::{BUFFER_SIZE, MC_SERVER_PORT};
use crate::metrics;
use std::collections::HashMap;
use std::io::{ErrorKind, Read, Write};
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};
use steamworks::networking_sockets::NetConnection;
use steamworks::networking_types::{ListenSocketEvent, SendFlags};
use steamworks::{Client, LobbyType, SteamId};


static RUNNING: AtomicBool = AtomicBool::new(true);

struct PeerState {
    connection: NetConnection,
    // Channel to send data to the MC server bridge thread
    to_mc_tx: Sender<Vec<u8>>,
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


    // Peer management: SteamId -> NetConnection
    let listen_socket = client
        .networking_sockets()
        .create_listen_socket_p2p(0, vec![])
        .map_err(|err| format!("无法创建 Steam NetworkingSockets 监听端口: {err:?}"))?;
    println!("📡 NetworkingSockets 监听已启动 (虚拟端口 0)");

    let mut peers: HashMap<SteamId, PeerState> = HashMap::new();
    
    // Channel to receive data from MC server threads: (steam_id, data)
    let (from_mc_tx, from_mc_rx): (Sender<(SteamId, Vec<u8>)>, Receiver<(SteamId, Vec<u8>)>) =
        mpsc::channel();

    println!("");
    println!("┌─────────────────────────────────────────────────────────┐");
    println!("│  🎮 P2P 转发服务已启动                                  │");
    println!("├─────────────────────────────────────────────────────────┤");
    println!("│  本地 MC 服务器: 127.0.0.1:{}                       │", MC_SERVER_PORT);
    println!("│  确保你的 Minecraft 服务器正在运行!                     │");
    println!("└─────────────────────────────────────────────────────────┘");
    println!("");

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
                        let connection = connected.take_connection();
                        
                        // Create channel for sending data to MC server
                        let (to_mc_tx, to_mc_rx): (Sender<Vec<u8>>, Receiver<Vec<u8>>) =
                            mpsc::channel();
                        
                        // Spawn thread to bridge this peer to MC server
                        let from_mc_tx_clone = from_mc_tx.clone();
                        let steam_id_clone = steam_id;
                        thread::spawn(move || {
                            if let Err(e) = bridge_to_mc_server(steam_id_clone, to_mc_rx, from_mc_tx_clone) {
                                println!("⚠️ MC 服务器连接断开 ({:?}): {}", steam_id_clone, e);
                            }
                        });
                        
                        peers.insert(
                            steam_id,
                            PeerState { connection, to_mc_tx },
                        );

                        println!("┌─────────────────────────────────────");
                        println!("│ [新玩家] Steam ID: {:?}", steam_id);
                        println!("│ 已建立连接并桥接到 MC 服务器");
                        println!("└─────────────────────────────────────");
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



        // Process data from MC server -> Send to peers via Steam
        while let Ok((steam_id, data)) = from_mc_rx.try_recv() {
            if let Some(peer) = peers.get(&steam_id) {
                if let Err(err) = peer.connection.send_message(&data, SendFlags::RELIABLE_NO_NAGLE) {
                    println!("✗ 发送数据到客户端失败: {err:?}");
                    metrics::record_packet_dropped();
                } else {
                    metrics::record_packet_sent(data.len() as u64);
                }
            }
        }

        // Process Steam packets from peers -> Forward to MC server
        let peers_to_remove: Vec<SteamId> = peers
            .iter_mut()
            .filter_map(|(steam_id, peer)| {
                match peer.connection.receive_messages(64) {
                    Ok(messages) => {
                        for message in messages {
                            let data = message.data();
                            if data.is_empty() {
                                continue;
                            }
                            metrics::record_packet_received(data.len() as u64);
                            if peer.to_mc_tx.send(data.to_vec()).is_err() {
                                // MC connection closed
                                return Some(*steam_id);
                            }
                        }
                    }
                    Err(_) => {
                        return Some(*steam_id);
                    }
                }
                None
            })
            .collect();

        for steam_id in peers_to_remove {
            peers.remove(&steam_id);
            println!("🔌 移除断开的玩家: {:?}", steam_id);
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

/// Bridge thread: connects to local MC server, forwards data bidirectionally
fn bridge_to_mc_server(
    steam_id: SteamId,
    to_mc_rx: Receiver<Vec<u8>>,
    from_mc_tx: Sender<(SteamId, Vec<u8>)>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = format!("127.0.0.1:{}", MC_SERVER_PORT);
    println!("🔗 为 {:?} 连接 MC 服务器 {}...", steam_id, addr);

    let mut stream = TcpStream::connect(&addr)?;
    stream.set_nonblocking(true)?;
    stream.set_nodelay(true)?;

    println!("✅ {:?} 已连接到 MC 服务器", steam_id);

    let mut read_buf = [0u8; BUFFER_SIZE];

    loop {
        // Send data from Steam to MC server
        while let Ok(data) = to_mc_rx.try_recv() {
            if let Err(e) = stream.write_all(&data) {
                println!("✗ 写入 MC 服务器失败: {:?}", e);
                return Ok(());
            }
        }

        // Read data from MC server
        match stream.read(&mut read_buf) {
            Ok(0) => {
                println!("MC 服务器关闭连接 ({:?})", steam_id);
                return Ok(());
            }
            Ok(n) => {
                if from_mc_tx.send((steam_id, read_buf[..n].to_vec())).is_err() {
                    return Ok(());
                }
            }
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                // No data available, continue
            }
            Err(e) => {
                println!("✗ 读取 MC 服务器失败: {:?}", e);
                return Ok(());
            }
        }

        thread::sleep(Duration::from_micros(100));
    }
}
