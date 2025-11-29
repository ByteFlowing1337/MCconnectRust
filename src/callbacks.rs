use std::sync::{Arc, Mutex};
use steamworks::networking_types::NetConnectionStatusChanged;
use steamworks::{CallbackHandle, Client, GameLobbyJoinRequested, LobbyId};

pub struct CallbackRegistry {
    pub join_lobby_id: Arc<Mutex<Option<LobbyId>>>,
    _join_handle: CallbackHandle,
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

        // NOTE: Legacy P2P callbacks (P2PSessionRequest, P2PSessionConnectFail) removed.
        // We now use NetworkingSockets API exclusively.

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
            _net_status_handle: net_status_handle,
        }
    }
}
