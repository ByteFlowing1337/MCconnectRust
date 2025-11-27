use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::Duration;
use steamworks::{
    Client, GameLobbyJoinRequested, LobbyId, LobbyType, P2PSessionRequest, SendType, SteamId,
};

// 配置常量
const MC_SERVER_PORT: u16 = 25565;
const CLIENT_LISTEN_PORT: u16 = 55555;

// 全局运行状态
static RUNNING: AtomicBool = AtomicBool::new(true);

fn send_reliable_with_retry(
    client: &Client,
    target: SteamId,
    data: &[u8],
) -> bool {
    const MAX_RETRIES: usize = 20;
    for _ in 0..MAX_RETRIES {
        if client
            .networking()
            .send_p2p_packet(target, SendType::Reliable, data)
        {
            return true;
        }
        thread::sleep(Duration::from_millis(10));
    }
    println!("警告: 向 {:?} 发送 {} 字节的 P2P 数据失败，连接尚未就绪", target, data.len());
    false
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化 Steamworks 客户端
    let client = Client::init()?;

    println!("------------------------------------------------");
    println!(">>> Steam MC Connect Tool (Fixed) <<<");
    println!("当前 Steam 用户: {}", client.friends().name());
    println!("------------------------------------------------");

    // 注册 P2P 握手回调，允许好友/非好友连接
    let client_p2p = client.clone();
    let _cb_p2p = client.register_callback(move |req: P2PSessionRequest| {
        println!(">>> 收到 P2P 连接请求，来自: {:?}，已自动接受。", req.remote);
        client_p2p.networking().accept_p2p_session(req.remote);
    });

    // 注册好友邀请回调
    let join_lobby_id = Arc::new(Mutex::new(None));
    let join_lobby_id_cb = join_lobby_id.clone();
    
    let _cb_join = client.register_callback(move |val: GameLobbyJoinRequested| {
        println!("\n>>> 收到好友邀请！准备加入大厅: {:?}", val.lobby_steam_id);
        *join_lobby_id_cb.lock().unwrap() = Some(val.lobby_steam_id);
    });

    println!("请选择模式:");
    println!("1. [主机] 创建房间 (我是服主)");
    println!("2. [客机] 加入房间 (输入房间号)");
    println!("3. [自动] 等待好友邀请/加入");
    print!("请输入 > ");
    std::io::stdout().flush()?;

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    let mode = input.trim();

    if mode == "1" {
        print!("请输入本地 MC 服务器端口 (默认 25565) > ");
        std::io::stdout().flush()?;
        let mut port_str = String::new();
        std::io::stdin().read_line(&mut port_str)?;
        let port = port_str.trim().parse::<u16>().unwrap_or(MC_SERVER_PORT);
        
        run_host(client, port)?;
    } else {
        let target_lobby = if mode == "2" {
            print!("请输入对方的房间号 (Lobby ID) > ");
            std::io::stdout().flush()?;
            let mut id_str = String::new();
            std::io::stdin().read_line(&mut id_str)?;
            Some(LobbyId::from_raw(id_str.trim().parse::<u64>().unwrap_or(0)))
        } else {
            println!("正在等待好友邀请... (保持此界面不动)");
            loop {
                client.run_callbacks();
                {
                    let lock = join_lobby_id.lock().unwrap();
                    if lock.is_some() {
                        break;
                    }
                }
                thread::sleep(Duration::from_millis(50));
            }
            let id = *join_lobby_id.lock().unwrap();
            id
        };

        if let Some(lobby_id) = target_lobby {
            run_client(client, lobby_id)?;
        } else {
            println!("无效的大厅 ID");
        }
    }

    Ok(())
}
fn connect_mc_with_retry(port: u16) -> Option<TcpStream> {
    for attempt in 1..=20 {
        match TcpStream::connect(("127.0.0.1", port)) {
            Ok(s) => return Some(s),
            Err(e) => {
                println!("MC 未就绪（第 {} 次尝试）: {:?}", attempt, e);
                std::thread::sleep(Duration::from_millis(200));
            }
        }
    }
    None
}

/// ================= 主机逻辑 (Server) =================
fn run_host(client: Client, port: u16) -> Result<(), Box<dyn std::error::Error>> {
    println!("正在创建 Steam 大厅...");
    
    client.matchmaking().create_lobby(LobbyType::Public, 10, |result| {
        match result {
            Ok(lobby_id) => {
                println!("\n>>> 房间创建成功! ID: {} <<<", lobby_id.raw());
            }
            Err(e) => println!("房间创建失败: {:?}", e),
        }
    });

    // 存储每个玩家的 MC TCP stream
    let mut client_streams: HashMap<SteamId, TcpStream> = HashMap::new();

    println!("等待来自 Steam 的 P2P 数据，MC 端口: {}", port);

    while RUNNING.load(Ordering::Relaxed) {
        client.run_callbacks();

        // 检查 Steam 是否有包到达
        while let Some(size) = client.networking().is_p2p_packet_available() {
            let mut buf = vec![0; size];
            if let Some((steam_id, len)) = client.networking().read_p2p_packet(&mut buf) {

                // ① 忽略 Steam 的 P2P keep-alive 空包 (len == 0)
                if len == 0 {
                    // println!("[调试] 忽略空包（steam keep-alive）");
                    continue;
                }

                let data = &buf[..len];

                // ② 如果该玩家第一包到达，建立到 MC 的 TCP 连接
                if !client_streams.contains_key(&steam_id) {
                    println!(">>> 新玩家连接: {:?}", steam_id);

                    // 尝试重复连接本地 MC
                    let stream = match connect_mc_with_retry(port) {
                        Some(s) => {
                            println!("已连接到本地 MC 服务器！");
                            s
                        }
                        None => {
                            println!("无法连接本地 MC 服务器，拒绝此玩家 {:?}", steam_id);
                            continue;
                        }
                    };

                    let _ = stream.set_nodelay(true);
                    let mut read_stream = stream.try_clone()?;

                    // 为该玩家启动 MC -> Steam 转发线程
                    let client_clone = client.clone();
                    let steam_id_clone = steam_id;

                    thread::spawn(move || {
                        let mut buffer = [0u8; 4096];
                        loop {
                            match read_stream.read(&mut buffer) {
                                Ok(0) => {
                                    println!("MC 返回 EOF，玩家 {:?} 连接结束", steam_id_clone);
                                    break;
                                }
                                Ok(n) => {
                                    if !send_reliable_with_retry(
                                        &client_clone,
                                        steam_id_clone,
                                        &buffer[..n],
                                    ) {
                                        println!("警告: MC->Steam 转发失败 {:?}", steam_id_clone);
                                        break;
                                    }
                                }
                                Err(e) => {
                                    println!("MC 读取错误 {:?}: {:?}", steam_id_clone, e);
                                    break;
                                }
                            }
                        }
                        println!("玩家 {:?} 的本地连接断开", steam_id_clone);
                    });

                    client_streams.insert(steam_id, stream);
                }

                // ③ Steam -> MC 写入转发
                if let Some(stream) = client_streams.get_mut(&steam_id) {
                    if let Err(e) = stream.write_all(data) {
                        println!(
                            "写入 MC 失败，断开玩家 {:?}，原因: {:?}",
                            steam_id, e
                        );
                        client_streams.remove(&steam_id);
                    }
                }
            }
        }

        thread::sleep(Duration::from_millis(1));
    }

    Ok(())
}


/// ================= 客机逻辑 (Client) =================
fn run_client(client: Client, lobby_id: LobbyId) -> Result<(), Box<dyn std::error::Error>> {
    println!("正在加入房间: {}", lobby_id.raw());
    
    // 使用 channel 来同步等待加入结果
    let (tx, rx) = std::sync::mpsc::channel();
    client.matchmaking().join_lobby(lobby_id, move |result| {
        let _ = tx.send(result);
    });

    // 等待加入完成
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

    let mut local_stream: Option<TcpStream> = None;
    let (disconnect_tx, disconnect_rx) = mpsc::channel();
    listener.set_nonblocking(true)?;

    loop {
        client.run_callbacks();

        while disconnect_rx.try_recv().is_ok() {
            println!("检测到本地 MC 连接断开，等待重新连接...");
            local_stream = None;
        }

        if local_stream.is_none() {
            if let Ok((stream, addr)) = listener.accept() {
                println!("MC 客户端已连接: {}", addr);
                let _ = stream.set_nodelay(true);
                
                let mut read_stream = stream.try_clone()?;
                let client_clone = client.clone();
                let target_host = host_id;

                let tx = disconnect_tx.clone();
                thread::spawn(move || {
                    let mut buffer = [0u8; 4096];
                    loop {
                        match read_stream.read(&mut buffer) {
                            Ok(n) if n > 0 => {
                                if !send_reliable_with_retry(
                                    &client_clone,
                                    target_host,
                                    &buffer[0..n],
                                ) {
                                    println!("警告: 客机向房主发送数据失败，可能正在重试");
                                }
                            }
                            _ => break,
                        }
                    }
                    println!("本地 MC 断开连接");
                    let _ = tx.send(());
                });

                local_stream = Some(stream);
                
                // 发送空包触发握手
                if !send_reliable_with_retry(&client, host_id, &[0u8; 0]) {
                    println!("警告: 无法向房主发送握手包，请稍后重试");
                }
            }
        }

        while let Some(size) = client.networking().is_p2p_packet_available() {
            let mut buf = vec![0; size];
            if let Some((_sender, len)) = client.networking().read_p2p_packet(&mut buf) {
                if len == 0 { continue; } // 忽略握手包
                
                if let Some(ref mut stream) = local_stream {
                    if let Err(e) = stream.write_all(&buf[0..len]) {
                        println!("写入本地 MC 失败: {}", e);
                        local_stream = None;
                    }
                }
            }
        }

        thread::sleep(Duration::from_millis(1));
    }
}