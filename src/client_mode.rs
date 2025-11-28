use crate::config::{BUFFER_SIZE, CLIENT_LISTEN_PORT};
use crate::metrics;
use crate::send_queue::SendQueue;
use crate::util::send_reliable_with_retry;
use std::io::{ErrorKind, Read, Write};
use std::net::TcpListener;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use steamworks::{Client, LobbyId};

pub fn run_client(client: Client, lobby_id: LobbyId) -> Result<(), Box<dyn std::error::Error>> {
    println!("正在加入房间: {}", lobby_id.raw());

    let (tx, rx) = std::sync::mpsc::channel();
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

    let listener = TcpListener::bind(format!("0.0.0.0:{}", CLIENT_LISTEN_PORT))?;
    println!(">>> 请在 Minecraft 中连接: 127.0.0.1:{}", CLIENT_LISTEN_PORT);

    let mut local_stream: Option<std::net::TcpStream> = None;
    let (disconnect_tx, disconnect_rx) = mpsc::channel();
    listener.set_nonblocking(true)?;

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

        while disconnect_rx.try_recv().is_ok() {
            println!("检测到本地 MC 连接断开，等待重新连接...");
            local_stream = None;
        }

        if local_stream.is_none() {
            match listener.accept() {
                Ok((stream, addr)) => {
                    let connect_time = Instant::now();
                    println!("┌─────────────────────────────────────");
                    println!("│ [连接] MC 客户端请求连接: {}", addr);
                    println!("└─────────────────────────────────────");
                    
                    // 设置为阻塞模式，避免 WouldBlock 错误
                    let _ = stream.set_nonblocking(false);
                    let _ = stream.set_nodelay(true);

                    let mut read_stream = stream.try_clone()?;
                    let client_clone = client.clone();
                    let target_host = host_id;
                    let tx = disconnect_tx.clone();

                    // 创建异步发送队列
                    let send_queue = SendQueue::new(client_clone.clone(), target_host);

                    thread::spawn(move || {
                        // 使用配置的大缓冲区
                        let mut buffer = [0u8; BUFFER_SIZE];
                        let mut total_sent = 0u64;
                        let mut packet_count = 0u32;
                        
                        loop {
                            match read_stream.read(&mut buffer) {
                                Ok(n) if n > 0 => {
                                    total_sent += n as u64;
                                    packet_count += 1;
                                    
                                    // 记录发送指标
                                    metrics::record_packet_sent(n as u64);

                                    // 使用异步队列发送
                                    if !send_queue.send(buffer[..n].to_vec()) {
                                        println!("⚠ 警告: 发送队列满或断开，丢弃数据");
                                        metrics::record_packet_dropped();
                                    }
                                }
                                Ok(_) => break,
                                Err(e) => {
                                    println!("✗ 读取本地 MC 失败: {:?}", e);
                                    break;
                                }
                            }
                        }
                        
                        let duration = connect_time.elapsed();
                        let duration_secs = duration.as_secs_f32();
                        
                        println!("┌─────────────────────────────────────");
                        println!("│ [断开] 本地 MC 连接已断开");
                        println!("│ 持续时间: {:.2}秒", duration_secs);
                        println!("│ 发送数据: {} 字节 ({} 包)", total_sent, packet_count);
                        println!("│ 平均吞吐: {:.2} MB/s", total_sent as f32 / duration_secs / 1024.0 / 1024.0);
                        println!("└─────────────────────────────────────");
                        
                        let _ = tx.send(());
                    });

                    // 发送空包触发握手 (仍使用同步发送以确保握手到达)
                    if !send_reliable_with_retry(&client, host_id, &[0]) {
                        println!("警告: 无法向房主发送握手包，请稍后重试");
                    }

                    local_stream = Some(stream);
                }
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => {}
                Err(e) => {
                    println!("等待 MC 连接时发生错误: {:?}", e);
                }
            }
        }

        while let Some(size) = client.networking().is_p2p_packet_available() {
            let mut buf = vec![0; size];
            if let Some((steam_id, len)) = client.networking().read_p2p_packet(&mut buf) {
                if len == 0 {
                    // println!("💓 收到来自 {:?} 的 keep-alive 包", steam_id);
                    continue;
                }

                if steam_id != host_id {
                    println!("⚠ 忽略来自 {:?} 的数据 (期望 {:?})", steam_id, host_id);
                    continue;
                }

                // 记录接收指标
                metrics::record_packet_received(len as u64);

                // 确保不越界
                let actual_len = len.min(buf.len());
                
                if let Some(ref mut stream) = local_stream {
                    if let Err(e) = stream.write_all(&buf[..actual_len]) {
                        println!("✗ 写入本地 MC 失败: {:?}", e);
                        local_stream = None;
                        println!("⚠ Steam 数据 {} bytes 被丢弃，等待 MC 重新连接", actual_len);
                    }
                } else {
                    println!("⚠ 收到 Steam 数据 {} bytes 但 MC 未连接 (缓冲中...)", actual_len);
                }
            }
        }

        thread::sleep(Duration::from_millis(1));
    }
}
