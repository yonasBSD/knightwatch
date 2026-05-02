use tokio::{net::TcpListener, sync::broadcast};

use crate::prelude::*;

pub fn get_listener(address: &str) -> Result<TcpListener> {
    let std_listener =
        std::net::TcpListener::bind(address).map_err(|err| Error::bind_address(address, err))?;
    std_listener
        .set_nonblocking(true)
        .map_err(|err| Error::bind_address(address, err))?;
    TcpListener::from_std(std_listener).map_err(|err| Error::bind_address(address, err))
}

pub fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn get_local_ip() -> Option<String> {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    Some(socket.local_addr().ok()?.ip().to_string())
}

pub fn print_local_ips() {
    let port = get_config().args.port;
    println!("API Server running at:");
    println!("  → http://localhost:{}", port);
    println!("  → http://127.0.0.1:{}", port);
    if let Some(ip) = get_local_ip() {
        println!("  → http://{}:{}", ip, port);
    }
}

pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1_024;
    const MB: u64 = KB * 1_024;
    const GB: u64 = MB * 1_024;
    const TB: u64 = GB * 1_024;
    match bytes {
        b if b >= TB => format!("{:.1} TB", b as f64 / TB as f64),
        b if b >= GB => format!("{:.1} GB", b as f64 / GB as f64),
        b if b >= MB => format!("{:.1} MB", b as f64 / MB as f64),
        b if b >= KB => format!("{:.1} KB", b as f64 / KB as f64),
        b => format!("{b} B"),
    }
}

pub fn format_uptime(secs: u64) -> String {
    let days = secs / 86_400;
    let hours = (secs % 86_400) / 3_600;
    let mins = (secs % 3_600) / 60;
    match days {
        0 => format!("{hours}h {mins}m"),
        d => format!("{d}d {hours}h {mins}m"),
    }
}

pub async fn recv_or_pending<T: Clone>(rx: &mut Option<broadcast::Receiver<T>>, name: &str) -> T {
    match rx {
        Some(rx) => loop {
            match rx.recv().await {
                Ok(val) => return val,
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => {
                    error!("{name} channel closed");
                    std::future::pending().await
                }
            }
        },
        None => std::future::pending().await,
    }
}
