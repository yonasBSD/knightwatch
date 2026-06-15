use serde::{Deserialize, Serialize};

use crate::utils::now_rfc3339;

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: String,
    pub version: String,
    pub uptime: String,
}

impl HealthResponse {
    pub fn new() -> Self {
        let uptime = super::handlers::START_TIME
            .get()
            .map(|t| t.elapsed().as_secs())
            .unwrap_or(0);
        Self {
            status: "healthy".to_string(),
            timestamp: now_rfc3339(),
            version: crate::utils::get_version().to_string(),
            uptime: crate::utils::format_uptime(uptime),
        }
    }
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
    pub docker: bool,
    pub allow_process_commands: bool,
    pub allow_screen_commands: bool,
    pub allow_system_resources_commands: bool,
    pub allow_systemd_commands: bool,
    pub allow_docker_commands: bool,
}

impl InfoResponse {
    pub fn from_pids(pid: Vec<u32>) -> Self {
        let args = &crate::prelude::get_config().args;
        Self {
            auth_enabled: args.enable_auth,
            blind: args.is_blind(),
            pid,
            top_processes: args.top_processes,
            limit_processes: args.limit_processes,
            telegram_bot: args.telegram,
            system_resources: args.system_resources,
            systemd: args.systemd,
            docker: args.docker,
            allow_process_commands: args.allow_process_commands,
            allow_screen_commands: args.allow_screen_commands,
            allow_system_resources_commands: args.allow_system_resources_commands,
            allow_systemd_commands: args.allow_systemd_commands,
            allow_docker_commands: args.allow_docker_commands,
        }
    }
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

impl From<crate::screen_capture::Screenshot> for ScreenshotImage {
    fn from(screenshot: crate::screen_capture::Screenshot) -> Self {
        Self {
            data: base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                &screenshot.image,
            ),
            mime: "image/png".to_string(),
            monitor_name: screenshot.monitor_name,
            monitor_id: screenshot.monitor_id,
            width: screenshot.width,
            height: screenshot.height,
            timestamp: now_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ScreenshotResponse {
    pub screens: Vec<ScreenshotImage>,
    pub count: usize,
}

#[derive(Deserialize)]
pub struct TopProcessesParams {
    pub limit: Option<usize>,
    pub sort: crate::process_tracker::SortKey,
}

#[derive(Deserialize)]
pub struct KillProcessRequest {
    pub signal: crate::process_tracker::ProcessSignal,
}

#[derive(Deserialize)]
pub struct SetPollIntervalRequest {
    pub interval_ms: u64,
}

#[derive(Debug, Deserialize)]
pub struct SetThresholdsRequest {
    pub cpu_warn: f32,
    pub memory_warn: f32,
    pub disk_warn: f32,
    pub battery_low: f32,
}

#[derive(Debug, Deserialize)]
pub struct SetRefreshMaskRequest {
    pub cpu: bool,
    pub memory: bool,
    pub disks: bool,
    pub networks: bool,
    pub temperatures: bool,
    pub gpus: bool,
}

#[derive(Deserialize)]
pub struct TopContainersParams {
    pub sort: crate::docker_tracker::DockerSortKey,
    pub limit: Option<usize>,
}

#[derive(Deserialize)]
pub struct ContainerRequest {
    pub id_or_name: String,
}

#[derive(Deserialize)]
pub struct KillContainerRequest {
    pub id_or_name: String,
    pub signal: Option<String>,
}

#[derive(Deserialize)]
pub struct ContainerTimeoutRequest {
    pub id_or_name: String,
    pub timeout_secs: Option<i32>,
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
