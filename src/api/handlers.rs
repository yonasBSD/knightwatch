use axum::{
    Router,
    routing::{get, post},
};
use tokio_util::sync::CancellationToken;

use super::end_points::*;
use crate::prelude::*;

fn create_router(cancel_token: CancellationToken) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/shutdown", post(shutdown))
        .route("/config", get(config))
        // ── Screenshot ────────────────────────────────────────────────────
        .route("/screenshot", get(screenshot))
        // ── Web dashboard ────────────────────────────────────────────────────
        .route("/", get(view))
        .route("/view", get(view))
        .route("/view.css", get(view_css))
        .route("/view.js", get(view_js))
        // ── Process tracking ──────────────────────────────────────────────
        .route("/root_pids", get(root_pids)) // root pids
        .route("/process/{root_pid}", get(process_tree)) // full tree
        .route("/process/root/{root_pid}", get(process_root)) // root only
        .route("/process/children/{root_pid}", get(process_children)) // children only
        .route("/process/status/{root_pid}", get(process_status)) // lightweight summary
        .route("/top-processes", get(top_processes)) // top processes
        // ── System Monitoring ──────────────────────────────────────────────
        .route("/system", get(system_snapshot)) // full system snapshot
        .route("/cpu", get(cpu_snapshot)) // cpu snapshot
        .route("/memory", get(memory_snapshot)) // memory snapshot
        .route("/disks", get(disks_snapshots)) // disks snapshot
        .route("/networks", get(networks_snapshot)) // networks snapshot
        .route("/gpus", get(gpus_snapshots)) // gpus snapshot
        .route("/battery", get(battery_snapshot)) // battery snapshot
        .route("/host-info", get(host_info_snapshot)) // host info snapshot
        .route("/temperatures", get(temperatures_snapshots)) // temperatures snapshot
        .with_state(cancel_token)
}

pub fn init_api_server(cancel_token: CancellationToken) -> Result<()> {
    let config = get_config();
    if config.args.no_server {
        return Ok(());
    }
    let api_listener = crate::utils::get_listener(&get_config().server_address())?;
    let app = create_router(cancel_token.clone());
    tokio::spawn(async move {
        if let Err(err) = axum::serve(api_listener, app)
            .with_graceful_shutdown(async move {
                cancel_token.cancelled().await;
            })
            .await
        {
            error!(?err, "API server error");
        } else {
            info!("API server stopped gracefully");
        }
    });
    info!("API server started");
    crate::utils::print_local_ips();
    Ok(())
}
