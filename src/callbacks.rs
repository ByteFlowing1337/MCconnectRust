use log::{info, warn};
use std::sync::{Arc, Mutex};
use steamworks::networking_types::NetConnectionStatusChanged;
use steamworks::{
    CallbackHandle, Client, GameLobbyJoinRequested, LobbyId, P2PSessionConnectFail,
    P2PSessionRequest,
};

#[allow(dead_code)]
pub struct CallbackRegistry {
    pub join_lobby_id: Arc<Mutex<Option<LobbyId>>>,
    _join_handle: CallbackHandle,
    _p2p_handle: CallbackHandle,
    _p2p_fail_handle: CallbackHandle,
    _net_status_handle: CallbackHandle,
}

impl CallbackRegistry {
    #[allow(dead_code)]
    pub fn register(client: &Client) -> Self {
        let join_lobby_id = Arc::new(Mutex::new(None));
        let join_lobby_clone = Arc::clone(&join_lobby_id);

        let join_handle = client.register_callback(move |val: GameLobbyJoinRequested| {
            info!("\n┌─────────────────────────────────────");
            info!("│  收到好友邀请！");
            info!("│ 房间 ID: {:?}", val.lobby_steam_id);
            info!("│ 准备加入大厅...");
            info!("└─────────────────────────────────────");
            *join_lobby_clone.lock().unwrap() = Some(val.lobby_steam_id);
        });

        let client_p2p = client.clone();
        let p2p_handle = client.register_callback(move |req: P2PSessionRequest| {
            info!("┌─────────────────────────────────────");
            info!("│ 收到 P2P 连接请求");
            info!("│ 来自: {:?}", req.remote);
            info!("│ 状态: 已自动接受");
            info!("└─────────────────────────────────────");
            client_p2p.networking().accept_p2p_session(req.remote);
        });

        let p2p_fail_handle = client.register_callback(|fail: P2PSessionConnectFail| {
            warn!("┌─────────────────────────────────────");
            warn!("│ ✗ P2P 连接失败");
            warn!("│ 对方: {:?}", fail.remote);
            warn!(
                "│ 错误码: {} ({})",
                fail.error,
                describe_p2p_error(fail.error)
            );
            warn!("│ 提示: 检查对方是否在线且运行相同应用");
            warn!("└─────────────────────────────────────");
        });

        let net_status_handle =
            client.register_callback(move |event: NetConnectionStatusChanged| {
                let current_state = event.connection_info.state();
                info!("┌─────────────────────────────────────");
                info!("│  连接状态变更");
                info!("│ 旧状态: {:?}", event.old_state);
                info!("│ 新状态: {:?}", current_state);

                if let Some(remote) = event.connection_info.identity_remote() {
                    info!("│ 远程: {:?}", remote);
                }

                if let Some(reason) = event.connection_info.end_reason() {
                    info!("│ 结束原因: {:?}", reason);
                }
                info!("└─────────────────────────────────────");

                /*
                if event.old_state != NetworkingConnectionState::Connected {
                    info!("连接详情: {:?}", event.connection_info);
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

#[allow(dead_code)]
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
