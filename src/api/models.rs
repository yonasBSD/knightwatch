use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: String,
    pub version: String,
    pub uptime: String,
}

#[derive(Debug, Serialize)]
pub struct ConfigResponse {
    pub blind: bool,
    pub pid: Vec<u32>,
    pub top_processes: bool,
    pub limit_processes: usize,
    pub telegram_bot: bool,
    pub system_monitor: bool,
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

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Deserialize)]
pub struct TopProcessesParams {
    pub limit: Option<usize>,
    pub sort: String,
}

#[cfg(not(debug_assertions))]
#[derive(rust_embed::Embed)]
#[folder = "Dashboard/dist/"]
pub struct DashboardAssets;
