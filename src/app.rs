use crate::callbacks::CallbackRegistry;
use crate::client_mode::run_client;
use crate::config::MC_SERVER_PORT;
use crate::host::run_host;
use steamworks::{Client, LobbyId};
use std::io::{self, Write};
use std::thread;
use std::time::Duration;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::init()?;

    // Warm up SDR relay access so fallback is ready before gameplay starts.
    let relay_utils = client.networking_utils();
    relay_utils.init_relay_network_access();
    let relay_status = relay_utils.relay_network_status();
    
    println!("\n╔════════════════════════════════════════════╗");
    println!("║   🎮 Steam MC Connect Tool v0.1.0         ║");
    println!("╠════════════════════════════════════════════╣");
    println!("║ Steam 用户: {:<31}║", client.friends().name());
    println!("║ 中继状态: {:<32}║", format!("{:?}", relay_status));
    println!("╚════════════════════════════════════════════╝\n");

    let callbacks = CallbackRegistry::register(&client);

    println!("请选择模式:");
    println!("  1.  [主机] 创建房间 (我是服主)");
    println!("  2.  [客机] 加入房间 (输入房间号)");
    println!("  3.  [自动] 等待好友邀请/加入");
    print!("\n请输入 > ");
    std::io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let mode = input.trim();

    if mode == "1" {
        run_host_mode(client)?;
    } else {
        run_client_mode(client, &callbacks, mode == "2")?;
    }

    Ok(())
}

fn run_host_mode(client: Client) -> Result<(), Box<dyn std::error::Error>> {
    print!("\n  请输入本地 MC 服务器端口 (默认 25565) > ");
    std::io::stdout().flush()?;
    let mut port_str = String::new();
    io::stdin().read_line(&mut port_str)?;
    let port = port_str.trim().parse::<u16>().unwrap_or(MC_SERVER_PORT);
    run_host(client, port)
}

fn run_client_mode(
    client: Client,
    callbacks: &CallbackRegistry,
    manual_id: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let target_lobby = if manual_id {
        ask_lobby_id()?
    } else {
        wait_for_invite(&client, callbacks)
    };

    if let Some(lobby_id) = target_lobby {
        run_client(client, lobby_id)?;
    } else {
        println!(" 无效的大厅 ID");
    }

    Ok(())
}

fn ask_lobby_id() -> Result<Option<LobbyId>, Box<dyn std::error::Error>> {
    print!("\n 请输入对方的房间号 (Lobby ID) > ");
    std::io::stdout().flush()?;
    let mut id_str = String::new();
    io::stdin().read_line(&mut id_str)?;
    Ok(Some(LobbyId::from_raw(
        id_str.trim().parse::<u64>().unwrap_or(0),
    )))
}

fn wait_for_invite(client: &Client, callbacks: &CallbackRegistry) -> Option<LobbyId> {
    println!("\n 正在等待好友邀请... (保持此界面不动)");
    loop {
        client.run_callbacks();
        if let Some(id) = *callbacks.join_lobby_id.lock().unwrap() {
            return Some(id);
        }
        thread::sleep(Duration::from_millis(50));
    }
}
