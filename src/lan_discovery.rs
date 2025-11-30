use std::net::UdpSocket;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::config::{LAN_BROADCAST_INTERVAL_MS, LAN_DISCOVERY_PORT, LAN_SERVER_NAME};

/// LAN广播器，用于向本地Minecraft客户端发送局域网服务器发现消息
pub struct LanBroadcaster {
    socket: UdpSocket,
    server_name: String,
    server_port: u16,
    running: Arc<AtomicBool>,
}

impl LanBroadcaster {
    /// 创建新的LAN广播器
    ///
    /// # Arguments
    /// * `server_name` - 服务器名称（显示在MC客户端中）
    /// * `server_port` - 服务器端口（MC客户端连接的端口）
    pub fn new(server_name: Option<String>, server_port: u16) -> Result<Self, Box<dyn std::error::Error>> {
        // 创建UDP socket用于发送广播
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        
        Ok(LanBroadcaster {
            socket,
            server_name: server_name.unwrap_or_else(|| LAN_SERVER_NAME.to_string()),
            server_port,
            running: Arc::new(AtomicBool::new(false)),
        })
    }

    /// 发送单次LAN发现广播
    fn broadcast_once(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Minecraft LAN发现消息格式: [MOTD]服务器名称[/MOTD][AD]端口[/AD]
        let message = format!(
            "[MOTD]{}[/MOTD][AD]{}[/AD]",
            self.server_name, self.server_port
        );

        // 发送到本地回环地址，MC客户端会监听此端口
        let target = format!("127.0.0.1:{}", LAN_DISCOVERY_PORT);
        self.socket.send_to(message.as_bytes(), &target)?;
        
        Ok(())
    }

    /// 启动LAN广播线程
    ///
    /// 返回一个停止句柄，调用stop()可以停止广播
    pub fn start(self) -> BroadcastHandle {
        self.running.store(true, Ordering::Relaxed);
        let running = Arc::clone(&self.running);

        let handle = thread::spawn(move || {
            println!("📡 LAN发现广播已启动");
            println!("   服务器名称: {}", self.server_name);
            println!("   服务器端口: {}", self.server_port);

            let mut broadcast_count = 0u32;
            
            while self.running.load(Ordering::Relaxed) {
                if let Err(e) = self.broadcast_once() {
                    println!("⚠ LAN广播发送失败: {:?}", e);
                } else {
                    broadcast_count += 1;
                    if broadcast_count == 1 {
                        println!("✓ 首次LAN广播已发送");
                    } else if broadcast_count % 10 == 0 {
                        println!("📊 已发送 {} 次LAN广播", broadcast_count);
                    }
                }

                // 每1.5秒发送一次广播
                thread::sleep(Duration::from_millis(LAN_BROADCAST_INTERVAL_MS));
            }

            println!("🛑 LAN发现广播已停止 (共发送 {} 次)", broadcast_count);
        });

        BroadcastHandle {
            running,
            handle: Some(handle),
        }
    }
}

/// LAN广播停止句柄
pub struct BroadcastHandle {
    running: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}

impl BroadcastHandle {
    /// 停止LAN广播
    pub fn stop(mut self) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for BroadcastHandle {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}
