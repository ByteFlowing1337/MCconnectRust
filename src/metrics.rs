use log::info;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// 全局性能指标
pub struct NetworkMetrics {
    packets_sent: AtomicU64,
    packets_received: AtomicU64,
    bytes_sent: AtomicU64,
    bytes_received: AtomicU64,
    packets_dropped: AtomicU64,
}

static METRICS: NetworkMetrics = NetworkMetrics {
    packets_sent: AtomicU64::new(0),
    packets_received: AtomicU64::new(0),
    bytes_sent: AtomicU64::new(0),
    bytes_received: AtomicU64::new(0),
    packets_dropped: AtomicU64::new(0),
};

/// 记录发送的包
pub fn record_packet_sent(bytes: u64) {
    METRICS.packets_sent.fetch_add(1, Ordering::Relaxed);
    METRICS.bytes_sent.fetch_add(bytes, Ordering::Relaxed);
}

/// 记录接收的包
pub fn record_packet_received(bytes: u64) {
    METRICS.packets_received.fetch_add(1, Ordering::Relaxed);
    METRICS.bytes_received.fetch_add(bytes, Ordering::Relaxed);
}

/// 记录丢弃的包
pub fn record_packet_dropped() {
    METRICS.packets_dropped.fetch_add(1, Ordering::Relaxed);
}

/// 获取当前指标快照
pub fn get_snapshot() -> MetricsSnapshot {
    MetricsSnapshot {
        packets_sent: METRICS.packets_sent.load(Ordering::Relaxed),
        packets_received: METRICS.packets_received.load(Ordering::Relaxed),
        bytes_sent: METRICS.bytes_sent.load(Ordering::Relaxed),
        bytes_received: METRICS.bytes_received.load(Ordering::Relaxed),
        packets_dropped: METRICS.packets_dropped.load(Ordering::Relaxed),
    }
}

/// 性能指标快照
#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    pub packets_sent: u64,
    pub packets_received: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub packets_dropped: u64,
}

impl MetricsSnapshot {
    /// 计算与另一个快照的差值
    pub fn delta(&self, earlier: &MetricsSnapshot) -> MetricsSnapshot {
        MetricsSnapshot {
            packets_sent: self.packets_sent.saturating_sub(earlier.packets_sent),
            packets_received: self
                .packets_received
                .saturating_sub(earlier.packets_received),
            bytes_sent: self.bytes_sent.saturating_sub(earlier.bytes_sent),
            bytes_received: self.bytes_received.saturating_sub(earlier.bytes_received),
            packets_dropped: self.packets_dropped.saturating_sub(earlier.packets_dropped),
        }
    }

    /// 格式化输出性能报告
    pub fn format_report(&self, duration: Duration) -> String {
        let secs = duration.as_secs_f32();
        if secs == 0.0 {
            return String::from("时间太短，无法计算速率");
        }

        let mbps_sent = (self.bytes_sent as f32 / secs) / 1024.0 / 1024.0;
        let mbps_recv = (self.bytes_received as f32 / secs) / 1024.0 / 1024.0;
        let pps_sent = self.packets_sent as f32 / secs;
        let pps_recv = self.packets_received as f32 / secs;

        format!(
            "发送: {:.2} MB/s ({:.0} pkt/s) | 接收: {:.2} MB/s ({:.0} pkt/s) | 丢包: {}",
            mbps_sent, pps_sent, mbps_recv, pps_recv, self.packets_dropped
        )
    }
}

/// 会话性能追踪器
pub struct SessionMetrics {
    start_time: Instant,
    initial_snapshot: MetricsSnapshot,
}

impl SessionMetrics {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            initial_snapshot: get_snapshot(),
        }
    }

    /// 获取会话期间的指标
    pub fn get_session_stats(&self) -> (MetricsSnapshot, Duration) {
        let current = get_snapshot();
        let delta = current.delta(&self.initial_snapshot);
        let duration = self.start_time.elapsed();
        (delta, duration)
    }

    /// 打印会话报告
    pub fn print_report(&self) {
        let (stats, duration) = self.get_session_stats();
        info!("│ 性能报告: {}", stats.format_report(duration));
    }
}
