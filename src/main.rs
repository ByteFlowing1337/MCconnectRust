mod app;
mod callbacks;
mod client_mode;
mod config;
mod host;
mod metrics;
mod vpn;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    app::run()
}