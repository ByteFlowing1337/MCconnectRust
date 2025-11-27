use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use steamworks::{Client, GameLobbyJoinRequested, LobbyId, LobbyType, SendType, SteamId};

// 配置常量
const MC_SERVER_PORT: u16 = 25565; // 主机本地 MC 端口
const CLIENT_LISTEN_PORT: u16 = 55555; // 客机本地监听端口

// 全局运行状态
static RUNNING: AtomicBool = AtomicBool::new(true);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 初始化 Steamworks 客户端
    let client = Client::init()?;
    
    println!("------------------------------------------------");
    println!(">>> Steam MC Connect Tool  <<<");
    println!("当前 Steam 用户: {}", client.friends().name());
    println!("------------------------------------------------");

    // 2. 注册邀请回调 (关键功能)
    // 当你点击好友发来的“加入游戏”时，这个变量会被设置
    let join_lobby_id = Arc::new(Mutex::new(None));
    let join_lobby_id_cb = join_lobby_id.clone();
    
    // 注册回调：处理好友邀请/加入请求
    let _cb_join = client.register_callback(move |val: GameLobbyJoinRequested| {
        println!("\n>>> 收到好友邀请！准备加入大厅: {:?}", val.lobby_steam_id);
        *join_lobby_id_cb.lock().unwrap() = Some(val.lobby_steam_id);
    });

    // 3. 询问模式
    println!("请选择模式:");
    println!("1. [主机] 创建房间 (我是服主)");
    println!("2. [客机] 加入房间 (输入房间号)");
    println!("3. [自动] 等待好友邀请/加入 (保持此界面不动)");
    print!("请输入 > ");
    std::io::stdout().flush()?;

    // 简单的非阻塞输入检查（为了能同时响应回调，这里简化为直接读取）
    // 实际使用中，如果用户想等邀请，可以直接挂着。如果输入数字则手动操作。
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    let mode = input.trim();

    // 4. 根据模式启动
    if mode == "1" {
        print!("请输入本地 MC 服务器端口 (默认 25565) > ");
        std::io::stdout().flush()?;
        let mut port_str = String::new();
        std::io::stdin().read_line(&mut port_str)?;
        let port = port_str.trim().parse::<u16>().unwrap_or(MC_SERVER_PORT);
        run_host(client, port)?;
    } else {
        // 如果是模式 2 或 3，我们需要判断是手动输入 ID 还是等待回调
        let target_lobby = if mode == "2" {
            print!("请输入对方的房间号 (Lobby ID) > ");
            std::io::stdout().flush()?;
            let mut id_str = String::new();
            std::io::stdin().read_line(&mut id_str)?;
            Some(LobbyId::from_raw(id_str.trim().parse::<u64>().unwrap_or(0)))
        } else {
            // 模式 3: 循环运行回调，直到收到邀请
            println!("正在等待好友邀请... (请在 Steam 聊天中点击 '加入游戏')");
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

/// ================= 主机逻辑 (Server) =================
fn run_host(client: Client, port: u16) -> Result<(), Box<dyn std::error::Error>> {
    println!("正在创建 Steam 大厅...");
    
    // 创建公开大厅 (允许非好友通过 ID 加入)
    client.matchmaking().create_lobby(LobbyType::Public, 10, |result| {
        match result {
            Ok(lobby_id) => {
                println!("\n>>> 房间创建成功! <<<");
                println!("房间 ID: {}", lobby_id.raw());
                println!("操作方法:");
                println!("1. 复制上面的 ID 给朋友");
                println!("2. 或者直接在 Steam 好友列表右键朋友 -> '邀请加入游戏'");
            }
            Err(e) => println!("房间创建失败: {:?}", e),
        }
    });

    // 存储所有连入的客机 Socket: Map<SteamID, TcpStream>
    let mut client_streams: HashMap<SteamId, TcpStream> = HashMap::new();

    println!("正在监听 Steam P2P 通道，转发至 localhost:{}...", port);

    while RUNNING.load(Ordering::Relaxed) {
        client.run_callbacks();

        // 处理 P2P 数据包
        while let Some(size) = client.networking().is_p2p_packet_available() {
            let mut buf = vec![0; size];
            // 读取数据
            if let Some((steam_id, len)) = client.networking().read_p2p_packet(&mut buf) {
                let data = &buf[0..len];

                // 检查是否已经有连接
                if !client_streams.contains_key(&steam_id) {
                    println!("新玩家连接: {:?}", steam_id);
                    
                    // 尝试连接本地 MC 服务器
                    match TcpStream::connect(format!("127.0.0.1:{}", port)) {
                        Ok(stream) => {
                            // !!! 关键优化：禁用 Nagle 算法，降低延迟 !!!
                            let _ = stream.set_nodelay(true);
                            
                            // 克隆 socket 用于读取线程
                            let mut read_stream = stream.try_clone()?;
                            let client_clone = client.clone();
                            
                            // 启动线程：从本地 MC 读取数据 -> 发送给 Steam 好友
                            thread::spawn(move || {
                                let mut buffer = [0u8; 4096];
                                loop {
                                    match read_stream.read(&mut buffer) {
                                        Ok(n) if n > 0 => {
                                            // SendReliable: 对应 TCP，保证顺序和必达
                                            client_clone.networking().send_p2p_packet(
                                                steam_id, 
                                                SendType::Reliable, 
                                                &buffer[0..n]
                                                
                                            );
                                        }
                                        _ => break, // 连接断开
                                    }
                                }
                                println!("玩家 {:?} 的本地连接断开", steam_id);
                            });

                            client_streams.insert(steam_id, stream);
                        }
                        Err(e) => {
                            println!("无法连接本地 MC 服务器: {}", e);
                            continue;
                        }
                    }
                }

                // 将 Steam 收到的数据写入本地 TCP
                if let Some(stream) = client_streams.get_mut(&steam_id) {
                    if let Err(_) = stream.write_all(data) {
                        // 写入失败，说明连接断了，移除
                        client_streams.remove(&steam_id);
                    }
                }
            }
        }
        
        // 接受 P2P 连接请求 (自动同意)
        // 实际上 steamworks-rs 默认策略通常是自动接受，但手动处理更稳妥
        // 这里略过 SessionRequest 回调，因为默认配置通常够用
        
        thread::sleep(Duration::from_millis(1));
    }
    Ok(())
}

/// ================= 客机逻辑 (Client) =================
fn run_client(client: Client, lobby_id: LobbyId) -> Result<(), Box<dyn std::error::Error>> {
    println!("正在加入房间: {}", lobby_id.raw());
    
    let (tx, rx) = std::sync::mpsc::channel();
    client.matchmaking().join_lobby(lobby_id, move |result| {
        let _ = tx.send(result);
    });

    // 循环等待加入结果，同时必须运行回调
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
                    println!("可能原因: 房间不存在(房主已关闭)、房间非公开(需好友关系)或网络问题。");
                    return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Failed to join lobby")));
                }
            }
        }
        thread::sleep(Duration::from_millis(50));
    }

    let host_id = client.matchmaking().lobby_owner(lobby_id);
    println!("房主 Steam ID: {:?}", host_id);

    if host_id == client.user().steam_id() {
        println!("!!! 警告: 你正在尝试连接自己 (Steam ID 相同) !!!");
        println!("!!! Steam P2P 不支持本地回环连接，请使用两个不同的 Steam 账号进行测试 !!!");
    }

    // 启动本地监听
    let listener = TcpListener::bind(format!("0.0.0.0:{}", CLIENT_LISTEN_PORT))?;
    println!(">>> 请在 Minecraft 中连接: 127.0.0.1:{}", CLIENT_LISTEN_PORT);

    // 本地 TCP 写入流 (主线程持有，用于把 Steam 数据写给 MC)
    let mut local_stream: Option<TcpStream> = None;

    // 设置非阻塞监听，或者我们用一个线程去 accept，主线程跑 callbacks
    // 为了简单，我们这里只支持一个 MC 客户端连接（通常也只需要一个）
    listener.set_nonblocking(true)?;

    loop {
        client.run_callbacks();

        // 1. 尝试接受本地 MC 的连接
        if local_stream.is_none() {
            if let Ok((stream, addr)) = listener.accept() {
                println!("本地 MC 已连接: {}", addr);
                let _ = stream.set_nodelay(true); // 关键优化
                
                let mut read_stream = stream.try_clone()?;
                let client_clone = client.clone();
                let target_host = host_id;

                // 启动线程：读取 MC 数据 -> 发送给房主
                thread::spawn(move || {
                    let mut buffer = [0u8; 4096];
                    loop {
                        match read_stream.read(&mut buffer) {
                            Ok(n) if n > 0 => {
                                client_clone.networking().send_p2p_packet(
                                    target_host,
                                    SendType::Reliable,
                                    &buffer[0..n]
                                    );
                            }
                            _ => break,
                        }
                    }
                    println!("本地 MC 断开连接");
                });

                local_stream = Some(stream);
                
                // 给房主发个空包打个招呼，建立 P2P 握手
                client.networking().send_p2p_packet(host_id, SendType::Reliable, &[0u8; 0]);
            }
        }

        // 2. 读取 Steam 发回的数据 -> 写入本地 MC
        while let Some(size) = client.networking().is_p2p_packet_available() {
            let mut buf = vec![0; size];
            if let Some((_sender, len)) = client.networking().read_p2p_packet(&mut buf) {
                // 如果收到空包（握手包），忽略
                if len == 0 { continue; }
                
                if let Some(ref mut stream) = local_stream {
                    let _ = stream.write_all(&buf[0..len]);
                }
            }
        }

        thread::sleep(Duration::from_millis(1));
    }
}