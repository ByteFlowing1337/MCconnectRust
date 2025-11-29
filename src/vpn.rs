use std::sync::mpsc::{self, Receiver};
use std::thread;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tun::Configuration;

// Performance tuning for large-scale packet handling
const TUN_READ_BUFFER_SIZE: usize = 65536;      // 64KB for mod burst traffic
const ASYNC_CHANNEL_CAPACITY: usize = 2000;     // 2K packets async queue
const SYNC_CHANNEL_CAPACITY: usize = 2000;      // 2K packets sync queue

pub struct VpnDevice {
    // We expose sync channels for the main application to use.
    pub tx: std::sync::mpsc::SyncSender<Vec<u8>>,      // Send packet TO TUN (from Steam)
    pub rx: Receiver<Vec<u8>>,    // Receive packet FROM TUN (to Steam)
}

impl VpnDevice {
    pub fn new(ip: &str, netmask: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let (tun_out_tx, tun_out_rx) = mpsc::sync_channel(SYNC_CHANNEL_CAPACITY); // TUN -> App
        let (app_in_tx, app_in_rx) = mpsc::sync_channel(SYNC_CHANNEL_CAPACITY);   // App -> TUN
        let (init_tx, init_rx) = mpsc::channel();       // Init signal

        let ip = ip.to_string();
        let netmask = netmask.to_string();
        
        let ip_clone = ip.clone();
        let netmask_clone = netmask.clone();

        thread::spawn(move || {
            let ip = ip_clone;
            let netmask = netmask_clone;
            let rt = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build() {
                    Ok(rt) => rt,
                    Err(e) => {
                        let _ = init_tx.send(Err(format!("Failed to build Tokio runtime: {}", e)));
                        return;
                    }
                };

            rt.block_on(async move {
                let mut config = Configuration::default();
                config
                    .address(&ip)
                    .netmask(&netmask)
                    .destination(&ip)
                    .up();

                #[cfg(target_os = "windows")]
                config.platform(|config| {
                    let _ = config;
                });

                let dev = match tun::create_as_async(&config) {
                    Ok(d) => d,
                    Err(e) => {
                        let _ = init_tx.send(Err(format!("Failed to create TUN device: {}. (Hint: Run as Administrator?)", e)));
                        return;
                    }
                };
                
                // Signal success
                let _ = init_tx.send(Ok(()));

                // Split the async device
                let (mut reader, mut writer) = tokio::io::split(dev);

                // Task: Read from TUN -> Send to App
                tokio::spawn(async move {
                    let mut buf = vec![0u8; TUN_READ_BUFFER_SIZE]; // 64KB buffer for burst traffic
                    loop {
                        match reader.read(&mut buf).await {
                            Ok(n) if n > 0 => {
                                let packet = buf[..n].to_vec();
                                if tun_out_tx.send(packet).is_err() {
                                    break;
                                }
                            }
                            Ok(_) => continue,
                            Err(e) => {
                                println!("Error reading from TUN: {:?}", e);
                                break;
                            }
                        }
                    }
                });

                // Bridge: App (Sync) -> TUN (Async)
                let (async_tx, mut async_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(ASYNC_CHANNEL_CAPACITY);

                // Spawn a blocking task to read from the Sync Receiver
                tokio::task::spawn_blocking(move || {
                    while let Ok(packet) = app_in_rx.recv() {
                        if async_tx.blocking_send(packet).is_err() {
                            break;
                        }
                    }
                });

                // Spawn an async task to write to TUN
                tokio::spawn(async move {
                    while let Some(packet) = async_rx.recv().await {
                        if let Err(e) = writer.write_all(&packet).await {
                            println!("Error writing to TUN: {:?}", e);
                            break;
                        }
                    }
                });
                
                std::future::pending::<()>().await;
            });
        });

        // Wait for initialization result
        match init_rx.recv() {
            Ok(Ok(_)) => {
                println!("VPN Interface created: IP={}, Mask={}", ip, netmask);
                Ok(VpnDevice {
                    tx: app_in_tx,
                    rx: tun_out_rx,
                })
            }
            Ok(Err(e)) => Err(e.into()),
            Err(_) => Err("VPN thread panicked or disconnected during initialization".into()),
        }
    }
}
