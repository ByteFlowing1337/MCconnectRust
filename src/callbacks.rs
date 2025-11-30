use std::sync::{Arc, Mutex};
use steamworks::networking_types::NetConnectionStatusChanged;
use steamworks::{CallbackHandle, Client, GameLobbyJoinRequested, LobbyId, P2PSessionConnectFail, P2PSessionRequest};

pub struct CallbackRegistry {
    pub join_lobby_id: Arc<Mutex<Option<LobbyId>>>,
    _join_handle: CallbackHandle,
    _p2p_handle: CallbackHandle,
    _p2p_fail_handle: CallbackHandle,
    _net_status_handle: CallbackHandle,
}

impl CallbackRegistry {
    pub fn register(client: &Client) -> Self {
        let join_lobby_id = Arc::new(Mutex::new(None));
        let join_lobby_clone = Arc::clone(&join_lobby_id);

        let join_handle = client.register_callback(move |val: GameLobbyJoinRequested| {
            println!("\n┌─────────────────────────────────────");
            println!("│  收到好友邀请！");
            println!("│ 房间 ID: {:?}", val.lobby_steam_id);
            println!("│ 准备加入大厅...");
            println!("└─────────────────────────────────────");
            *join_lobby_clone.lock().unwrap() = Some(val.lobby_steam_id);
        });

        let client_p2p = client.clone();
        let p2p_handle = client.register_callback(move |req: P2PSessionRequest| {
            println!("┌─────────────────────────────────────");
            println!("│ 收到 P2P 连接请求");
            println!("│ 来自: {:?}", req.remote);
            println!("│ 状态: 已自动接受");
            println!("└─────────────────────────────────────");
            client_p2p.networking().accept_p2p_session(req.remote);
        });

        let p2p_fail_handle = client.register_callback(|fail: P2PSessionConnectFail| {
            println!("┌─────────────────────────────────────");
            println!("│ ✗ P2P 连接失败");
            println!("│ 对方: {:?}", fail.remote);
            println!("│ 错误码: {} ({})", fail.error, describe_p2p_error(fail.error));
            println!("│ 提示: 检查对方是否在线且运行相同应用");
            println!("└─────────────────────────────────────");
        });

        let net_status_handle = client.register_callback(move |event: NetConnectionStatusChanged| {
            let current_state = event.connection_info.state();
            println!("┌─────────────────────────────────────");
            println!("│  连接状态变更");
            println!("│ 旧状态: {:?}", event.old_state);
            println!("│ 新状态: {:?}", current_state);

            if let Some(remote) = event.connection_info.identity_remote() {
                println!("│ 远程: {:?}", remote);
            }

            if let Some(reason) = event.connection_info.end_reason() {
                println!("│ 结束原因: {:?}", reason);
            }
            println!("└─────────────────────────────────────");

            /*
            if event.old_state != NetworkingConnectionState::Connected {
                println!("连接详情: {:?}", event.connection_info);
            }
            */
        });

        Self {
            join_lobby_id,
            _join_handle: join_handle,
            _p2p_handle: p2p_handle,
            _p2p_fail_handle: p2p_fail_handle,
            _net_status_handle: net_status_handle,
        }
    }
}

fn describe_p2p_error(code: u8) -> &'static str {
    match code {
        0 => "None",
        1 => "Remote 未运行该应用",
        2 => "无权访问该应用",
        3 => "Remote 未登录 Steam",
        4 => "连接超时",
        _ => "未知错误",
    }
}