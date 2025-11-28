use crate::config::{RETRY_ATTEMPTS, RETRY_DELAY_MS, SEND_QUEUE_SIZE};
use crate::metrics;
use steamworks::{Client, SendType, SteamId};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender, TrySendError};
use std::thread;
use std::time::Duration;

/// 异步发送队列
pub struct SendQueue {
    tx: SyncSender<Vec<u8>>,
}

impl SendQueue {
    /// 创建新的发送队列并启动后台发送线程
    pub fn new(client: Client, target: SteamId) -> Self {
        let (tx, rx): (SyncSender<Vec<u8>>, Receiver<Vec<u8>>) = sync_channel(SEND_QUEUE_SIZE);

        thread::spawn(move || {
            Self::worker_loop(rx, client, target);
        });

        Self { tx }
    }

    /// 非阻塞发送数据
    /// 如果队列已满，返回 false
    pub fn send(&self, data: Vec<u8>) -> bool {
        match self.tx.try_send(data) {
            Ok(_) => true,
            Err(TrySendError::Full(_)) => {
                metrics::record_packet_dropped();
                false
            }
            Err(TrySendError::Disconnected(_)) => false,
        }
    }

    /// 后台工作线程循环
    fn worker_loop(rx: Receiver<Vec<u8>>, client: Client, target: SteamId) {
        for data in rx {
            Self::send_reliable_with_retry(&client, target, &data);
        }
    }

    /// 内部重试发送逻辑
    fn send_reliable_with_retry(client: &Client, target: SteamId, data: &[u8]) -> bool {
        for _ in 1..=RETRY_ATTEMPTS {
            if client
                .networking()
                .send_p2p_packet(target, SendType::Reliable, data)
            {
                return true;
            }

            // 失败重试，这里是在后台线程，阻塞是可以接受的
            // 但为了不阻塞后续包太久，可以考虑更短的等待或指数退避
            // 这里保持简单，使用配置的延迟
            thread::sleep(Duration::from_millis(RETRY_DELAY_MS));
        }

        metrics::record_packet_dropped();
        false
    }
}
