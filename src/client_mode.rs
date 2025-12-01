use crate::config::{BUFFER_SIZE, CLIENT_LISTEN_PORT};
use crate::lan_discovery::LanBroadcaster;
use crate::metrics;
use log::{error, info, warn};
use std::io::{ErrorKind, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};
use steamworks::networking_types::{NetworkingConnectionState, NetworkingIdentity, SendFlags};
use steamworks::{Client, LobbyId};

pub fn run_client(
    client: Client, 
    lobby_id: LobbyId, 
    password: Option<String>,
    ready_tx: Sender<Result<(), String>>,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("═══════════════════════════════════════════════════════");
    info!("开始加入房间流程");
    info!("目标房间 ID: {}", lobby_id.raw());
    info!("本机 Steam ID: {:?}", client.user().steam_id());
    info!("═══════════════════════════════════════════════════════");

    let (tx, rx) = mpsc::channel();
    info!("📡 正在向 Steam 发送加入房间请求...");
    client.matchmaking().join_lobby(lobby_id, move |result| {
        info!("📩 收到 Steam 加入房间回调: {:?}", result);
        let _ = tx.send(result);
    });

    let join_deadline = Instant::now() + Duration::from_secs(10);
    loop {
        client.run_callbacks();
        if let Ok(result) = rx.try_recv() {
            match result {
                Ok(_) => {
                    info!(">>> 加入成功! <<<");
                    break;
                }
                Err(_) => {
                    // Steam 的 join_lobby 只返回 Err(())，无法获取具体错误原因
                    // 常见原因：房间不存在、已关闭、已满员、Steam服务不可用
                    let err_msg = "加入房间失败 - 请检查: 1) 房间号是否正确 2) 房主是否仍在运行 3) Steam是否正常连接".to_string();
                    error!("{}", err_msg);
                    let _ = ready_tx.send(Err(err_msg));
                    return Ok(());
                }
            }
        }
        
        if Instant::now() > join_deadline {
            let err_msg = "加入房间超时 - Steam服务可能暂时不可用，请稍后重试".to_string();
            error!("{}", err_msg);
            let _ = ready_tx.send(Err(err_msg));
            return Ok(());
        }
        thread::sleep(Duration::from_millis(50));
    }

    // 验证房间密码，增加重试逻辑应对Steam后端数据同步延迟
    let lobby_password = (0..15)
        .find_map(|i| {
            client.run_callbacks();
            if i > 0 {
                thread::sleep(Duration::from_millis(200));
            }
            let pw = client.matchmaking().lobby_data(lobby_id, "password");

            // 如果客户端提供了密码，我们必须等到从lobby元数据中读到密码
            if password.is_some() && pw.is_none() {
                info!("等待房间密码数据同步... (尝试 #{})", i + 1);
                None
            } else {
                Some(pw)
            }
        })
        .flatten();

    // 执行密码验证
    match (password.as_deref(), lobby_password.as_deref()) {
        // 客户端提供了密码
        (Some(client_pwd), Some(lobby_pwd)) => {
            if client_pwd != lobby_pwd {
                let err_msg = "房间密码错误".to_string();
                let _ = ready_tx.send(Err(err_msg.clone()));
                return Err(err_msg.into());
            }
        }
        (Some(_), None) => {
            let err_msg = "验证密码超时，或房主未设置密码".to_string();
            let _ = ready_tx.send(Err(err_msg.clone()));
            return Err(err_msg.into());
        }
        // 客户端未提供密码，但房间有密码 (且不为空)
        (None, Some(lobby_pwd)) if !lobby_pwd.is_empty() => {
            let err_msg = "房间需要密码，但未提供密码".to_string();
            let _ = ready_tx.send(Err(err_msg.clone()));
            return Err(err_msg.into());
        }
        // 其他情况（都无密码，或房间密码为空）均视为通过
        _ => {}
    }
    info!("✓ 密码验证成功");

    let host_id = client.matchmaking().lobby_owner(lobby_id);
    info!("房主 Steam ID: {:?}", host_id);

    if host_id == client.user().steam_id() {
        let err_msg = "无法连接自己，请使用两个不同的账号测试".to_string();
        error!("!!! 错误: {} !!!", err_msg);
        let _ = ready_tx.send(Err(err_msg.clone()));
        return Err(err_msg.into());
    }

    // 使用新版 NetworkingSockets API 连接房主
    info!("📡 正在建立 NetworkingSockets 连接...");
    let sockets = client.networking_sockets();
    let host_identity = NetworkingIdentity::new_steam_id(host_id);

    let mut connection = match sockets.connect_p2p(host_identity, 0, vec![]) {
        Ok(conn) => conn,
        Err(_) => {
            let err_msg = "无法向房主发起连接，Steam NetworkingSockets 初始化失败".to_string();
            let _ = ready_tx.send(Err(err_msg.clone()));
            return Err(err_msg.into());
        }
    };

    // 等待连接建立
    let connect_deadline = Instant::now() + Duration::from_secs(15);
    let mut last_state_log = Instant::now();
    loop {
        client.run_callbacks();
        if let Ok(info) = sockets.get_connection_info(&connection) {
            if let Ok(state) = info.state() {
                // 每秒打印一次连接状态
                if last_state_log.elapsed() > Duration::from_secs(1) {
                    info!("📊 连接状态: {:?}", state);
                    last_state_log = Instant::now();
                }
                
                match state {
                    NetworkingConnectionState::Connected => {
                        info!("✅ NetworkingSockets 连接已建立");
                        break;
                    }
                    NetworkingConnectionState::ClosedByPeer => {
                        let err_msg = "房主拒绝了连接 (ClosedByPeer) - 请确保房主程序正在运行且房间号正确".to_string();
                        error!("{}", err_msg);
                        let _ = ready_tx.send(Err(err_msg.clone()));
                        return Err(err_msg.into());
                    }
                    NetworkingConnectionState::ProblemDetectedLocally => {
                        let err_msg = "本地检测到连接问题 (ProblemDetectedLocally) - 可能是网络问题或Steam服务不可用".to_string();
                        error!("{}", err_msg);
                        let _ = ready_tx.send(Err(err_msg.clone()));
                        return Err(err_msg.into());
                    }
                    NetworkingConnectionState::None => {
                        info!("⏳ 连接状态: None (初始化中...)");
                    }
                    NetworkingConnectionState::Connecting => {
                        info!("⏳ 连接状态: Connecting (正在连接房主...)");
                    }
                    NetworkingConnectionState::FindingRoute => {
                        info!("⏳ 连接状态: FindingRoute (正在寻找路由...)");
                    }
                }
            }
        }

        if Instant::now() > connect_deadline {
            let err_msg = "连接房主超时 (15秒) - 房主可能不在线或网络问题".to_string();
            error!("{}", err_msg);
            let _ = ready_tx.send(Err(err_msg.clone()));
            return Err(err_msg.into());
        }
        thread::sleep(Duration::from_millis(50));
    }

    // 启动本地监听
    let listener = match TcpListener::bind(format!("0.0.0.0:{}", CLIENT_LISTEN_PORT)) {
        Ok(l) => l,
        Err(e) => {
            let err_msg = format!("无法绑定端口 {}: {}", CLIENT_LISTEN_PORT, e);
            let _ = ready_tx.send(Err(err_msg.clone()));
            return Err(err_msg.into());
        }
    };
    listener.set_nonblocking(true)?;
    info!(
        ">>> 请在 Minecraft 中连接: 127.0.0.1:{}",
        CLIENT_LISTEN_PORT
    );

    // 启动LAN发现广播
    let broadcaster = LanBroadcaster::new(Some("LAN world".to_string()), CLIENT_LISTEN_PORT)?;
    let _broadcast_handle = broadcaster.start();
    info!("✓ Minecraft LAN发现广播已启动 (服务器名称: LAN world)");

    info!("");
    info!("┌─────────────────────────────────────────────────────────┐");
    info!("│  ✅ 已连接到房主!                                       │");
    info!("├─────────────────────────────────────────────────────────┤");
    info!("│  🎮 Minecraft 连接方式:                                 │");
    info!(
        "│     多人游戏 -> 添加服务器 -> 输入: 127.0.0.1:{}    │",
        CLIENT_LISTEN_PORT
    );
    info!("└─────────────────────────────────────────────────────────┘");
    info!("");

    // 通知前端连接已就绪
    let _ = ready_tx.send(Ok(()));

    // Channel: MC读取线程 -> 主循环 (发送到Steam)
    let (from_mc_tx, from_mc_rx): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = mpsc::channel();

    let mut mc_stream: Option<TcpStream> = None;
    let mut mc_read_thread_started = false;

    // 性能统计会话
    let session_metrics = metrics::SessionMetrics::new();
    let mut last_report_time = Instant::now();

    loop {
        client.run_callbacks();

        // 定期打印性能报告
        if last_report_time.elapsed() > Duration::from_secs(5) {
            session_metrics.print_report();
            last_report_time = Instant::now();
        }

        // 检查是否有新的 MC 客户端连接
        if mc_stream.is_none() {
            match listener.accept() {
                Ok((stream, addr)) => {
                    info!("┌─────────────────────────────────────");
                    info!("│ [连接] MC 客户端已连接: {}", addr);
                    info!("└─────────────────────────────────────");

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
                                        info!("[读取线程] MC 客户端断开连接");
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
                                        error!("✗ 读取 MC 失败: {:?}", e);
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
                    error!("等待 MC 连接时发生错误: {:?}", e);
                }
            }
        }

        // 更新延迟信息
        if let Ok((status, _)) = sockets.get_realtime_connection_status(&connection, 0) {
            let ping_ms = status.ping() as u32;
            let host_id = client.matchmaking().lobby_owner(lobby_id);
            metrics::update_latency(host_id.raw(), ping_ms);
        }

        // 从 MC 读取数据 -> 发送到 Steam
        while let Ok(data) = from_mc_rx.try_recv() {
            match connection.send_message(&data, SendFlags::RELIABLE_NO_NAGLE) {
                Ok(_) => {
                    metrics::record_packet_sent(data.len() as u64);
                }
                Err(err) => {
                    error!("✗ 发送到房主失败: {:?}", err);
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
                            error!("✗ 写入 MC 失败: {:?}", e);
                            mc_stream = None;
                        }
                    }
                }
            }
            Err(err) => {
                warn!("⚠️ 从房主接收数据失败: {:?}", err);
            }
        }

        thread::sleep(Duration::from_micros(100));
    }
}
