use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use axum_extra::{TypedHeader, headers};
use base64::{Engine as _, engine::general_purpose};

use super::{models::*, utils::*};
use crate::{
    process_tracker::{self, ProcessSignal, ProcessSnapshot, ProcessStatus, ProcessTree},
    screen_capture, system_resources, systemd,
    utils::now_rfc3339,
};

pub async fn shutdown(
    State(cancel_token): State<tokio_util::sync::CancellationToken>,
) -> &'static str {
    cancel_token.cancel();
    "Shutting down…"
}

pub async fn health() -> Json<HealthResponse> {
    let uptime = super::handlers::START_TIME
        .get()
        .map(|t| t.elapsed().as_secs())
        .unwrap_or(0);
    Json(HealthResponse {
        status: "healthy".to_string(),
        timestamp: now_rfc3339(),
        version: crate::utils::get_version().to_string(),
        uptime: crate::utils::format_uptime(uptime),
    })
}

pub async fn info() -> Json<InfoResponse> {
    let args = &crate::prelude::get_config().args;
    Json(InfoResponse {
        auth_enabled: args.enable_auth,
        #[cfg(feature = "screenshot")]
        blind: args.blind,
        #[cfg(not(feature = "screenshot"))]
        blind: true,
        pid: process_tracker::get_root_pids().await,
        top_processes: args.top_processes,
        limit_processes: args.limit_processes,
        telegram_bot: args.telegram,
        system_resources: args.system_resources,
        systemd: args.systemd,
        allow_process_commands: args.allow_process_commands,
        allow_screen_commands: args.allow_screen_commands,
    })
}

pub async fn login(Json(body): Json<LoginRequest>) -> Result<Json<LoginResponse>, StatusCode> {
    let users = crate::config::get_users();
    if users.users.is_empty() {
        return Err(StatusCode::NOT_FOUND);
    }
    match users.verify_password(&body.username, &body.password) {
        Ok(true) => {}
        _ => return Err(StatusCode::UNAUTHORIZED),
    }
    let token = uuid::Uuid::new_v4().to_string();
    let session = super::session::Session {
        username: body.username.clone(),
        token: token.clone(),
    };
    super::session::get_sessions()
        .write()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .insert(session);
    Ok(Json(LoginResponse { token }))
}

pub async fn logout(
    TypedHeader(auth): TypedHeader<headers::Authorization<headers::authorization::Bearer>>,
) -> StatusCode {
    match super::session::get_sessions().write() {
        Ok(mut sessions) => {
            sessions.remove_by_token(auth.token());
            StatusCode::OK
        }
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

// ---------------------------------------------------------------------------
// Screenshot endpoints
// ---------------------------------------------------------------------------

pub async fn screenshot() -> Result<Json<ScreenshotResponse>, (StatusCode, String)> {
    let images = screen_capture::get_screenshots().await;
    if images.is_empty() {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "No screens found".to_string(),
        ));
    }
    let screens: Vec<ScreenshotImage> = images
        .into_iter()
        .map(|s| ScreenshotImage {
            data: general_purpose::STANDARD.encode(&s.image),
            mime: "image/png".to_string(),
            monitor_name: s.monitor_name,
            monitor_id: s.monitor_id,
            width: s.width,
            height: s.height,
            timestamp: s.timestamp,
        })
        .collect();
    let count = screens.len();
    Ok(Json(ScreenshotResponse { screens, count }))
}

// ---------------------------------------------------------------------------
// Screen capture command endpoints (requires --allow-screen-commands)
// ---------------------------------------------------------------------------

/// `POST /screen/poll/pause`
pub async fn screen_capture_pause_poll() -> Result<StatusCode, (StatusCode, String)> {
    screen_capture::pause_poll()
        .await
        .map_err(internal_server_error)?;
    Ok(StatusCode::OK)
}

/// `POST /screen/poll/resume`
pub async fn screen_capture_resume_poll() -> Result<StatusCode, (StatusCode, String)> {
    screen_capture::resume_poll()
        .await
        .map_err(internal_server_error)?;
    Ok(StatusCode::OK)
}

/// `POST /screen/poll/interval`
pub async fn screen_capture_set_poll_interval(
    Json(body): Json<SetPollIntervalRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let interval = tokio::time::Duration::from_millis(body.interval_ms);
    screen_capture::set_poll_interval(interval)
        .await
        .map_err(internal_server_error)?;
    Ok(StatusCode::OK)
}

// ---------------------------------------------------------------------------
// Process tracking endpoints
// ---------------------------------------------------------------------------

/// `GET /root_pids`
///
/// Returns a list of currently tracked root PIDs.
pub async fn root_pids() -> Json<Vec<u32>> {
    Json(process_tracker::get_root_pids().await)
}

/// `GET /process/{pid}`
///
/// Returns the full process tree of a given root pid: root + all live descendants, plus a
/// `work_done` flag. Useful for dashboards or external orchestration.
pub async fn process_tree(Path(root_pid): Path<u32>) -> Json<ProcessTree> {
    let (root, children, work_done) = tokio::join!(
        process_tracker::get_root(root_pid),
        process_tracker::get_children(root_pid),
        process_tracker::is_work_done(root_pid),
    );

    let child_count = children.len();
    Json(ProcessTree {
        root,
        children,
        child_count,
        work_done,
        timestamp: now_rfc3339(),
    })
}

/// `GET /process/root/{pid}`
///
/// Returns only the root process snapshot of a given root pid, or 404 if it has exited.
pub async fn process_root(
    Path(root_pid): Path<u32>,
) -> Result<Json<ProcessSnapshot>, (StatusCode, String)> {
    match process_tracker::get_root(root_pid).await {
        Some(snap) => Ok(Json(snap)),
        None => Err(not_found("Root process is not running".to_string())),
    }
}

/// `GET /process/children/{pid}`
///
/// Returns snapshots of all currently live child processes of a given root pid.
pub async fn process_children(Path(root_pid): Path<u32>) -> Json<Vec<ProcessSnapshot>> {
    let children = process_tracker::get_children(root_pid).await;
    Json(children)
}

/// `GET /process/status/{pid}`
///
/// Lightweight summary — cheap to poll frequently.
/// Returns root alive/dead, child count, and the `work_done` flag of a given root pid.
pub async fn process_status(Path(root_pid): Path<u32>) -> Json<ProcessStatus> {
    let (root_snap, child_count, work_done) = tokio::join!(
        process_tracker::get_root(root_pid),
        async { process_tracker::get_children(root_pid).await.len() },
        process_tracker::is_work_done(root_pid),
    );

    Json(ProcessStatus {
        root_alive: root_snap.is_some(),
        root_pid: root_snap.as_ref().map(|s| s.pid),
        root_name: root_snap.map(|s| s.name),
        child_count,
        work_done,
        timestamp: now_rfc3339(),
    })
}

/// `GET /top-processes?limit=10&sort=cpu`
///
/// Returns the top N processes sorted by the given key.
///
/// # Query Parameters
/// - `limit`: Number of processes to return (default: 0 = all)
/// - `sort`: Sort key, either `cpu` or `mem` (default: `cpu`)
///
/// # Errors
/// - `400 Bad Request` if `sort` is not a valid sort key
pub async fn top_processes(
    Query(params): Query<TopProcessesParams>,
) -> Result<Json<Vec<ProcessSnapshot>>, (StatusCode, String)> {
    let limit = params.limit.unwrap_or(0);
    let sort_key = process_tracker::SortKey::try_from(params.sort).map_err(bad_request)?;
    let top_processes = process_tracker::get_top_processes(sort_key, limit).await;
    Ok(Json(top_processes))
}

/// `GET /supported-signals`
///
/// Returns a list of supported signal based on current platform.
pub async fn supported_signals() -> Json<Vec<ProcessSignal>> {
    Json(ProcessSignal::get_supported_signals())
}

// ---------------------------------------------------------------------------
// Process command endpoints (requires --allow-process-commands)
// ---------------------------------------------------------------------------

/// `POST /process/kill/{pid}`
pub async fn kill_process(
    Path(pid): Path<u32>,
    body: Json<KillProcessRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let signal =
        process_tracker::ProcessSignal::try_from(body.signal.as_str()).map_err(bad_request)?;
    if !signal.is_supported() {
        return Err(bad_request(
            crate::errors::Error::unsupported_signal(signal).to_string(),
        ));
    }
    process_tracker::kill_process(pid, signal)
        .await
        .map_err(internal_server_error)?;
    Ok(StatusCode::OK)
}

/// `POST /process/kill-tree/{root_pid}`
pub async fn kill_tree(Path(root_pid): Path<u32>) -> Result<Json<Vec<u32>>, (StatusCode, String)> {
    process_tracker::kill_tree(root_pid)
        .await
        .map(Json)
        .map_err(internal_server_error)
}

/// `POST /process/track/{pid}`
pub async fn track_pid(Path(pid): Path<u32>) -> Result<StatusCode, (StatusCode, String)> {
    process_tracker::track_pid(pid)
        .await
        .map_err(internal_server_error)?;
    Ok(StatusCode::OK)
}

/// `POST /process/untrack/{pid}`
pub async fn untrack_pid(Path(pid): Path<u32>) -> Result<StatusCode, (StatusCode, String)> {
    process_tracker::untrack_pid(pid)
        .await
        .map_err(internal_server_error)?;
    Ok(StatusCode::OK)
}

/// `POST /process/poll/pause`
pub async fn process_tracker_pause_poll() -> Result<StatusCode, (StatusCode, String)> {
    process_tracker::pause_poll()
        .await
        .map_err(internal_server_error)?;
    Ok(StatusCode::OK)
}

/// `POST /process/poll/resume`
pub async fn process_tracker_resume_poll() -> Result<StatusCode, (StatusCode, String)> {
    process_tracker::resume_poll()
        .await
        .map_err(internal_server_error)?;
    Ok(StatusCode::OK)
}

/// `POST /process/poll/interval`
pub async fn process_tracker_set_poll_interval(
    Json(body): Json<SetPollIntervalRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let interval = tokio::time::Duration::from_millis(body.interval_ms);
    process_tracker::set_poll_interval(interval)
        .await
        .map_err(internal_server_error)?;
    Ok(StatusCode::OK)
}

// ---------------------------------------------------------------------------
// System Resources endpoints
// ---------------------------------------------------------------------------

/// `GET /system`
///
/// Returns the current System Snapshot.
pub async fn system_snapshot()
-> Result<Json<system_resources::SystemSnapshot>, (StatusCode, String)> {
    match system_resources::get_snapshot().await {
        Some(snap) => Ok(Json(snap)),
        None => Err(not_found("No System Snapshot was found".to_string())),
    }
}

/// `GET /cpu`
///
/// Returns the current Cpu Snapshot.
pub async fn cpu_snapshot() -> Result<Json<system_resources::CpuSnapshot>, (StatusCode, String)> {
    match system_resources::get_cpu().await {
        Some(snap) => Ok(Json(snap)),
        None => Err(not_found("No Cpu Snapshot was found".to_string())),
    }
}

/// `GET /memory`
///
/// Returns the current Memory Snapshot.
pub async fn memory_snapshot()
-> Result<Json<system_resources::MemorySnapshot>, (StatusCode, String)> {
    match system_resources::get_memory().await {
        Some(snap) => Ok(Json(snap)),
        None => Err(not_found("No Memory Snapshot was found".to_string())),
    }
}

/// `GET /disks`
///
/// Returns the Disks Snapshots.
pub async fn disks_snapshots() -> Json<Vec<system_resources::DiskSnapshot>> {
    Json(system_resources::get_disks().await)
}

/// `GET /networks`
///
/// Returns the Networks Snapshots.
pub async fn networks_snapshot() -> Json<Vec<system_resources::NetworkSnapshot>> {
    Json(system_resources::get_networks().await)
}

/// `GET /gpus`
///
/// Returns the Gpus Snapshots.
pub async fn gpus_snapshots() -> Json<Vec<system_resources::GpuSnapshot>> {
    Json(system_resources::get_gpus().await)
}

/// `GET /battery`
///
/// Returns the current Battery Snapshot.
pub async fn battery_snapshot()
-> Result<Json<system_resources::BatterySnapshot>, (StatusCode, String)> {
    match system_resources::get_battery().await {
        Some(snap) => Ok(Json(snap)),
        None => Err(not_found("No battery Snapshot was found".to_string())),
    }
}

/// `GET /host-info`
///
/// Returns the current Host Info Snapshot.
pub async fn host_info_snapshot() -> Result<Json<system_resources::HostInfo>, (StatusCode, String)>
{
    match system_resources::get_host_info().await {
        Some(snap) => Ok(Json(snap)),
        None => Err(not_found("No host info was found".to_string())),
    }
}

/// `GET /temperatures`
///
/// Returns the Temperatures Snapshots.
pub async fn temperatures_snapshots() -> Json<Vec<system_resources::ThermalSnapshot>> {
    Json(system_resources::get_temperatures().await)
}

// ---------------------------------------------------------------------------
// Systemd endpoints
// ---------------------------------------------------------------------------

/// `GET /systemd`
///
/// Returns the current Systemd Snapshot.
pub async fn systemd_snapshot() -> Result<Json<systemd::SystemdSnapshot>, (StatusCode, String)> {
    match systemd::get_snapshot().await {
        Some(snap) => Ok(Json(snap)),
        None => Err(not_found("No Systemd Snapshot was found".to_string())),
    }
}

/// `GET /unit/{unit_name}`
///
/// Returns Unit Snapshot by name.
pub async fn unit_snapshot(
    Path(unit_name): Path<String>,
) -> Result<Json<systemd::UnitSnapshot>, (StatusCode, String)> {
    match systemd::get_unit(unit_name).await {
        Some(snap) => Ok(Json(snap)),
        None => Err(not_found("No Unit Snapshot was found".to_string())),
    }
}

/// `GET /units/{unit_state}`
///
/// Returns units by active state.
pub async fn units_by_active_state(
    Path(unit_state): Path<String>,
) -> Json<Vec<systemd::UnitSnapshot>> {
    Json(systemd::get_units_by_active_state(unit_state.as_str().into()).await)
}

/// `GET /failed_units`
///
/// Returns failedunits.
pub async fn failed_units() -> Json<Vec<systemd::UnitSnapshot>> {
    Json(systemd::get_failed_units().await)
}
