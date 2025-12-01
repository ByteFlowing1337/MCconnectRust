use log::{info, warn};
use serde::{Deserialize, Serialize};
use std::net::{Ipv4Addr, SocketAddr, TcpStream, UdpSocket};
use std::time::{Duration, Instant};

/// Minecraft 服务器信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinecraftServer {
    pub ip: String,
    pub port: u16,
    pub motd: String,
    pub latency_ms: f32,
}

/// 监听 Minecraft LAN 发现广播，查找本地服务器
///
/// # Returns
/// 返回找到的第一个服务器信息，如果超时未找到则返回 None
pub fn discover_minecraft_server() -> Option<MinecraftServer> {
    info!("🔍 开始搜索本地 Minecraft 服务器...");

    // 创建 UDP socket 并绑定到组播端口
    let socket = match UdpSocket::bind("0.0.0.0:4445") {
        Ok(s) => s,
        Err(e) => {
            warn!("✗ 无法绑定 UDP 端口 4445: {}", e);
            return None;
        }
    };

    // 加入组播组 224.0.2.60
    let multicast_addr = Ipv4Addr::new(224, 0, 2, 60);
    let interface_addr = Ipv4Addr::new(0, 0, 0, 0);

    if let Err(e) = socket.join_multicast_v4(&multicast_addr, &interface_addr) {
        warn!("✗ 无法加入组播组: {}", e);
        return None;
    }

    // 设置 3 秒超时
    if let Err(e) = socket.set_read_timeout(Some(Duration::from_secs(3))) {
        warn!("✗ 无法设置超时: {}", e);
        return None;
    }

    info!("📡 监听组播地址 224.0.2.60:4445...");

    // 监听广播消息
    let mut buffer = [0u8; 1024];
    loop {
        match socket.recv_from(&mut buffer) {
            Ok((size, addr)) => {
                let message = String::from_utf8_lossy(&buffer[..size]);
                info!("📥 收到来自 {} 的 LAN 广播: {}", addr, message);

                // 解析消息: [MOTD]服务器名称[/MOTD][AD]端口[/AD]
                if let Some(parsed) = parse_lan_message(&message) {
                    let server_addr = SocketAddr::new(addr.ip(), parsed.port);

                    let now = Instant::now();
                    let latency =
                        if let Ok(_stream) = TcpStream::connect_timeout(&server_addr, Duration::from_secs(1)) {
                            now.elapsed().as_secs_f32() * 1000.0
                        } else {
                            -1.0
                        };

                    let server = MinecraftServer {
                        ip: server_addr.ip().to_string(),
                        port: parsed.port,
                        motd: parsed.motd,
                        latency_ms: latency,
                    };

                    info!(
                        "✓ 发现 Minecraft 服务器: {} ({}:{}) - 延迟: {:.2} ms",
                        server.motd, server.ip, server.port, server.latency_ms
                    );
                    return Some(server);
                }
            }
            Err(e) => {
                // 超时或其他错误
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut
                {
                    info!("⏱ 搜索超时，未找到 Minecraft 服务器");
                } else {
                    warn!("✗ 接收数据失败: {}", e);
                }
                break;
            }
        }
    }

    None
}

/// 从广播消息中解析出的信息
struct ParsedInfo {
    port: u16,
    motd: String,
}

/// 解析 Minecraft LAN 广播消息
///
/// 消息格式: [MOTD]服务器名称[/MOTD][AD]端口[/AD]
fn parse_lan_message(message: &str) -> Option<ParsedInfo> {
    // 提取 MOTD
    let motd = extract_tag_value(message, "MOTD")?;

    // 提取端口
    let port_str = extract_tag_value(message, "AD")?;
    let port = port_str.parse::<u16>().ok()?;

    Some(ParsedInfo {
        port,
        motd: motd.to_string(),
    })
}

/// 从消息中提取标签值
///
/// 例如: extract_tag_value("[MOTD]My Server[/MOTD]", "MOTD") -> Some("My Server")
fn extract_tag_value<'a>(message: &'a str, tag: &str) -> Option<&'a str> {
    let start_tag = format!("[{}]", tag);
    let end_tag = format!("[/{}]", tag);
    
    let start = message.find(&start_tag)? + start_tag.len();
    let end = message.find(&end_tag)?;
    
    if start < end {
        Some(&message[start..end])
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_lan_message() {
        let message = "[MOTD]My Test Server[/MOTD][AD]25565[/AD]";
        let parsed = parse_lan_message(message).unwrap();
        assert_eq!(parsed.port, 25565);
        assert_eq!(parsed.motd, "My Test Server");
    }

    #[test]
    fn test_extract_tag_value() {
        assert_eq!(extract_tag_value("[MOTD]Test[/MOTD]", "MOTD"), Some("Test"));
        assert_eq!(extract_tag_value("[AD]12345[/AD]", "AD"), Some("12345"));
        assert_eq!(extract_tag_value("Invalid", "MOTD"), None);
    }
}
