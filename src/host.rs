use crate::config::BUFFER_SIZE;
use crate::metrics;
use crate::send_queue::SendQueue;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};
use steamworks::{Client, LobbyType, SteamId};

static RUNNING: AtomicBool = AtomicBool::new(true);

pub fn run_host(client: Client, port: u16) -> Result<(), Box<dyn std::error::Error>> {
    println!(" 正在创建 Steam 大厅...");

    client.matchmaking().create_lobby(LobbyType::Public, 10, |result| {
        match result {
            Ok(lobby_id) => {
                println!("┌─────────────────────────────────────");
                println!("│ ✓ 房间创建成功!");
                println!("│ 房间 ID: {}", lobby_id.raw());
                println!("│ 好友可通过此 ID 加入游戏");
                println!("└─────────────────────────────────────");
            }
            Err(e) => println!("✗ 房间创建失败: {:?}", e),
        }
    });

    let mut client_streams: HashMap<SteamId, TcpStream> = HashMap::new();
    println!(" 等待来自 Steam 的 P2P 数据，MC 端口: {}", port);

    // 性能统计会话
    let session_metrics = metrics::SessionMetrics::new();
    let mut last_report_time = Instant::now();

    while RUNNING.load(Ordering::Relaxed) {
        client.run_callbacks();

        // 定期打印性能报告 (每 5 秒)
        if last_report_time.elapsed() > Duration::from_secs(5) {
            session_metrics.print_report();
            last_report_time = Instant::now();
        }

        while let Some(size) = client.networking().is_p2p_packet_available() {
            let mut buf = vec![0; size];
            if let Some((steam_id, len)) = client.networking().read_p2p_packet(&mut buf) {
                if len == 0 {
                    continue;
                }

                // 记录接收指标
                metrics::record_packet_received(len as u64);

                let data = &buf[..len];


                if !client_streams.contains_key(&steam_id) {
                    println!("┌─────────────────────────────────────");
                    println!("│ [新玩家] Steam ID: {:?}", steam_id);
                    println!("└─────────────────────────────────────");
                    
                    let connect_time = Instant::now();
                    let stream = match connect_mc_with_retry(port) {
                        Some(s) => {
                            println!("✓ 已连接到本地 MC 服务器 (端口 {})！", port);
                            s
                        }
                        None => {
                            println!("✗ 无法连接本地 MC 服务器，拒绝此玩家 {:?}", steam_id);
                            continue;
                        }
                    };

                    let _ = stream.set_nodelay(true);
                    let mut read_stream = stream.try_clone()?;
                    let client_clone = client.clone();
                    let steam_id_clone = steam_id;

                    // 创建异步发送队列
                    let send_queue = SendQueue::new(client_clone.clone(), steam_id_clone);

                    thread::spawn(move || {
                        // 使用配置的大缓冲区
                        let mut buffer = [0u8; BUFFER_SIZE];
                        let mut total_sent = 0u64;
                        let mut packet_count = 0u32;
                        
                        loop {
                            match read_stream.read(&mut buffer) {
                                Ok(0) => {
                                    println!("✓ MC 返回 EOF，玩家 {:?} 正常结束", steam_id_clone);
                                    break;
                                }
                                Ok(n) => {
                                    total_sent += n as u64;
                                    packet_count += 1;
                                    
                                    // 记录发送指标
                                    metrics::record_packet_sent(n as u64);

                                    // 使用异步队列发送
                                    if !send_queue.send(buffer[..n].to_vec()) {
                                        println!("⚠ 警告: 发送队列满或断开，丢弃数据 {:?}", steam_id_clone);
                                        // 注意：这里我们不立即断开连接，而是允许丢包，
                                        // 因为队列满可能是暂时的。但如果持续满，可能会有问题。
                                        // 也可以选择 break 断开。
                                    }
                                }
                                Err(e) => {
                                    println!("✗ MC 读取错误 {:?}: {:?}", steam_id_clone, e);
                                    break;
                                }
                            }
                        }
                        
                        let duration = connect_time.elapsed();
                        let duration_secs = duration.as_secs_f32();
                        
                        println!("┌─────────────────────────────────────");
                        println!("│ [断开] 玩家 {:?}", steam_id_clone);
                        println!("│ 持续时间: {:.2}秒", duration_secs);
                        println!("│ 转发数据: {} 字节 ({} 包)", total_sent, packet_count);
                        println!("│ 平均吞吐: {:.2} MB/s", total_sent as f32 / duration_secs / 1024.0 / 1024.0);
                        println!("└─────────────────────────────────────");
                        
                        client_clone.networking().close_p2p_session(steam_id_clone);
                    });

                    client_streams.insert(steam_id, stream);
                    println!(" 当前活跃玩家: {}", client_streams.len());
                }

                if !is_handshake {
                    if let Some(stream) = client_streams.get_mut(&steam_id) {
                        if let Err(e) = stream.write_all(data) {
                            println!("┌─────────────────────────────────────");
                            println!("│ [错误] 写入 MC 失败");
                            println!("│ 玩家: {:?}", steam_id);
                            println!("│ 原因: {:?}", e);
                            println!("└─────────────────────────────────────");
                            
                            client.networking().close_p2p_session(steam_id);
                            client_streams.remove(&steam_id);
                            println!("当前活跃玩家: {}", client_streams.len());
                        }
                    }
                }
            }
        }

        thread::sleep(Duration::from_millis(1));
    }

    Ok(())
}

fn connect_mc_with_retry(port: u16) -> Option<TcpStream> {
    for attempt in 1..=20 {
        match TcpStream::connect(("127.0.0.1", port)) {
            Ok(s) => return Some(s),
            Err(e) => {
                println!("MC 未就绪（第 {} 次尝试）: {:?}", attempt, e);
                thread::sleep(Duration::from_millis(200));
            }
        }
    }
    None
}
