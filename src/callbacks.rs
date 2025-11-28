use std::sync::{Arc, Mutex};
use steamworks::networking_types::NetConnectionStatusChanged;
use steamworks::{CallbackHandle, Client, GameLobbyJoinRequested, LobbyId, P2PSessionRequest};

pub struct CallbackRegistry {
    pub join_lobby_id: Arc<Mutex<Option<LobbyId>>>,
    _join_handle: CallbackHandle,
    _p2p_handle: CallbackHandle,
    _net_status_handle: CallbackHandle,
}

impl CallbackRegistry {
    pub fn register(client: &Client) -> Self {
        let join_lobby_id = Arc::new(Mutex::new(None));
        let join_lobby_clone = Arc::clone(&join_lobby_id);

        let join_handle = client.register_callback(move |val: GameLobbyJoinRequested| {
            println!("\n>>> 收到好友邀请！准备加入大厅: {:?}", val.lobby_steam_id);
            *join_lobby_clone.lock().unwrap() = Some(val.lobby_steam_id);
        });

        let client_p2p = client.clone();
        let p2p_handle = client.register_callback(move |req: P2PSessionRequest| {
            println!(">>> 收到 P2P 连接请求，来自: {:?}，已自动接受。", req.remote);
            client_p2p.networking().accept_p2p_session(req.remote);
        });

        let net_status_handle = client.register_callback(move |event: NetConnectionStatusChanged| {
            let current_state = event.connection_info.state();
            println!(
                ">>> 连接状态变更: {:?} -> {:?}",
                event.old_state,
                current_state
            );

            if let Some(remote) = event.connection_info.identity_remote() {
                println!("连接来自: {:?}", remote);
            }

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
            _net_status_handle: net_status_handle,
        }
    }
}
