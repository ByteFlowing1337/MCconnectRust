use steamworks::{Client, SendType, SteamId};
use std::thread;
use std::time::Duration;

const RETRY_ATTEMPTS: usize = 5;
const RETRY_DELAY_MS: u64 = 50;

/// Send data reliably with a few retries to smooth over transient failures.
pub fn send_reliable_with_retry(
    client: &Client,
    target: SteamId,
    data: &[u8],
) -> bool {
    for attempt in 1..=RETRY_ATTEMPTS {
        if client
            .networking()
            .send_p2p_packet(target, SendType::Reliable, data)
        {
            return true;
        }

        println!(
            "send_p2p_packet 失败，第 {}/{} 次重试...",
            attempt, RETRY_ATTEMPTS
        );
        thread::sleep(Duration::from_millis(RETRY_DELAY_MS));
    }

    println!("发送失败，放弃此次数据包");
    false
}
