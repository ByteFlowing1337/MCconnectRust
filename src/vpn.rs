use std::sync::mpsc::{self, Receiver, Sender};

pub struct VpnDevice {
    // tx: send data to the virtual NIC (outgoing to the kernel/device)
    pub tx: Sender<Vec<u8>>,
    // rx: receive data from the virtual NIC (incoming from the kernel/device)
    pub rx: Receiver<Vec<u8>>,
}

impl VpnDevice {
    pub fn new(_ip: &str, _netmask: &str) -> Result<VpnDevice, Box<dyn std::error::Error>> {
        // Minimal in-memory tunnel implementation used to get the project compiling.
        // This creates two channels and returns one side to the caller. A proper
        // implementation should open a real TUN device and bridge bytes between
        // the OS and these channels.
        // Channels: app -> device, device -> app
        let (app_to_dev_tx, _app_to_dev_rx) = mpsc::channel::<Vec<u8>>();
        let (_dev_to_app_tx, dev_to_app_rx) = mpsc::channel::<Vec<u8>>();

        // NOTE: A real implementation would spawn a TUN device driver here and
        // bridge packets between the OS and these channels. For now, this stub
        // simply lets the project compile.

        Ok(VpnDevice {
            tx: app_to_dev_tx,
            rx: dev_to_app_rx,
        })
    }
}
