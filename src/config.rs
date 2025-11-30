// 网络端口配置
pub const MC_SERVER_PORT: u16 = 25565;
pub const CLIENT_LISTEN_PORT: u16 = 55555;

// 性能优化配置
pub const BUFFER_SIZE: usize = 65536;           // 64KB 读取缓冲区
pub const SEND_QUEUE_SIZE: usize = 1000;        // 发送队列容量
pub const RETRY_ATTEMPTS: usize = 5;            // 重试次数
pub const RETRY_DELAY_MS: u64 = 50;             // 重试延迟（毫秒）

// 异步队列配置 (为后续阶段准备)
#[allow(dead_code)]
pub const WORKER_THREADS: usize = 2;            // 发送工作线程数

// LAN发现配置
pub const LAN_DISCOVERY_PORT: u16 = 4445;
pub const LAN_BROADCAST_INTERVAL_MS: u64 = 1500;
pub const LAN_SERVER_NAME: &str = "MCconnect P2P Server";
