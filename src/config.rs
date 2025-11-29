

// 性能优化配置
pub const BUFFER_SIZE: usize = 65536;           // 64KB 读取缓冲区
pub const SEND_QUEUE_SIZE: usize = 1000;        // 发送队列容量
pub const RETRY_ATTEMPTS: usize = 5;            // 重试次数
pub const RETRY_DELAY_MS: u64 = 50;             // 重试延迟（毫秒）

// 异步队列配置 (为后续阶段准备)
#[allow(dead_code)]
pub const WORKER_THREADS: usize = 2;            // 发送工作线程数
