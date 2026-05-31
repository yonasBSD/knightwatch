use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: String,
    pub version: String,
    pub uptime: String,
}

#[derive(Debug, Serialize)]
pub struct InfoResponse {
    pub auth_enabled: bool,
    pub blind: bool,
    pub pid: Vec<u32>,
    pub top_processes: bool,
    pub limit_processes: usize,
    pub telegram_bot: bool,
    pub system_resources: bool,
    pub systemd: bool,
    pub allow_process_commands: bool,
    pub allow_screen_commands: bool,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
}

// ---------------------------------------------------------------------------
// Screenshot
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct ScreenshotImage {
    pub data: String,
    pub mime: String,
    pub monitor_name: String,
    pub monitor_id: u32,
    pub width: u32,
    pub height: u32,
    pub timestamp: String,
}

#[derive(Debug, Serialize)]
pub struct ScreenshotResponse {
    pub screens: Vec<ScreenshotImage>,
    pub count: usize,
}

#[derive(Deserialize)]
pub struct TopProcessesParams {
    pub limit: Option<usize>,
    pub sort: String,
}

#[derive(Deserialize)]
pub struct KillProcessRequest {
    pub signal: String,
}

#[derive(Deserialize)]
pub struct SetPollIntervalRequest {
    pub interval_ms: u64,
}

#[cfg(not(debug_assertions))]
#[derive(rust_embed::Embed)]
#[folder = "dashboard/dist/"]
pub struct DashboardAssets;

pub struct Vite {
    pub child_process: std::process::Child,
}

impl Vite {
    pub fn stop(mut self) {
        let _ = self.child_process.kill();
        let _ = self.child_process.wait();
        tracing::info!("Shutdown vite");
    }
}
