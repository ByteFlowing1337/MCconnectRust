use crate::callbacks::CallbackRegistry;
use crate::client_mode::run_client;
use crate::config::MC_SERVER_PORT;
use crate::host::run_host;
use log::{info, warn};
use std::io::{self, Write};
use std::thread;
use std::time::Duration;
use steamworks::{Client, LobbyId};

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::init()?;

    // Warm up SDR relay access so fallback is ready before gameplay starts.
    let relay_utils = client.networking_utils();
    relay_utils.init_relay_network_access();
    let relay_status = relay_utils.relay_network_status();

    info!("\n╔════════════════════════════════════════════╗");
    info!("║   🎮 Steam MC Connect Tool v0.1.0         ║");
    info!("╠════════════════════════════════════════════╣");
    info!("║ Steam 用户: {:<31}║", client.friends().name());
    info!("║ 中继状态: {:<32}║", format!("{:?}", relay_status));
    info!("╚════════════════════════════════════════════╝\n");

    let callbacks = CallbackRegistry::register(&client);

    info!("请选择模式:");
    info!("  1.  [主机] 创建房间 (我是服主)");
    info!("  2.  [客机] 加入房间 (输入房间号)");
    info!("  3.  [自动] 等待好友邀请/加入");
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
    let port = loop {
        print!("\n  请输入本地 MC 服务器端口 (默认 25565) > ");
        std::io::stdout().flush()?;
        let mut port_str = String::new();
        io::stdin().read_line(&mut port_str)?;
        let trimmed = port_str.trim();
        if trimmed.is_empty() {
            break MC_SERVER_PORT;
        }
        match trimmed.parse::<u16>() {
            Ok(port) => break port,
            Err(_) => {
                warn!("✗ 无效的端口号，请输入一个 1-65535 之间的数字。");
            }
        }
    };
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
        if lobby_id.raw() == 0 {
            warn!("✗ 无效或空的大厅 ID。");
        } else {
            run_client(client, lobby_id)?;
        }
    } else {
        warn!("未找到大厅。");
    }

    Ok(())
}

fn ask_lobby_id() -> Result<Option<LobbyId>, Box<dyn std::error::Error>> {
    let lobby_id = loop {
        print!("\n 请输入对方的房间号 (Lobby ID) > ");
        std::io::stdout().flush()?;
        let mut id_str = String::new();
        io::stdin().read_line(&mut id_str)?;
        let trimmed = id_str.trim();
        if trimmed.is_empty() {
            warn!("✗ 房间号不能为空。");
            continue;
        }
        match trimmed.parse::<u64>() {
            Ok(id) => break LobbyId::from_raw(id),
            Err(_) => {
                warn!("✗ 无效的房间号，请输入一个纯数字 ID。");
            }
        }
    };
    Ok(Some(lobby_id))
}

fn wait_for_invite(client: &Client, callbacks: &CallbackRegistry) -> Option<LobbyId> {
    info!("\n 正在等待好友邀请... (保持此界面不动)");
    loop {
        client.run_callbacks();
        if let Some(id) = *callbacks.join_lobby_id.lock().unwrap() {
            return Some(id);
        }
        thread::sleep(Duration::from_millis(50));
    }
}
