
mod app;
mod callbacks;
mod client_mode;
mod config;
mod host;
mod lan_discovery;
mod metrics;

use std::io;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if let Err(e) = app::run() {
        println!("\n❌ 发生错误: {}", e);
        println!("\n按回车键退出...");
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        return Err(e);
    }
    Ok(())
}