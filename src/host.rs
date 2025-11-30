use crate::metrics;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use steamworks::networking_sockets::NetConnection;
use steamworks::networking_types::ListenSocketEvent;
use steamworks::{Client, LobbyType, SteamId};


static RUNNING: AtomicBool = AtomicBool::new(true);

struct PeerState {
    connection: NetConnection,
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

    println!("P2P 转发服务已启动，等待玩家加入...");

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
                        peers.insert(
                            steam_id,
                            PeerState { connection },
                        );

                        println!("┌─────────────────────────────────────");
                        println!("│ [新玩家] Steam ID: {:?}", steam_id);
                        println!("│ 已建立连接");
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



        // Periodic reporting
        if last_report_time.elapsed() > Duration::from_secs(5) {
            session_metrics.print_report();
            last_report_time = Instant::now();
        }

        thread::sleep(Duration::from_micros(100)); // 100μs for higher throughput
    }

    Ok(())
}
