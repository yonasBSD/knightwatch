use axum::{
    Router,
    routing::{get, post},
};
use tokio_util::sync::CancellationToken;

use super::end_points::*;
use crate::prelude::*;

fn init_start_time() {
    super::constants::START_TIME.get_or_init(std::time::Instant::now);
}

fn create_api_router(cancel_token: CancellationToken) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/shutdown", post(shutdown))
        .route("/config", get(config))
        // ── Screenshot ────────────────────────────────────────────────────
        .route("/screenshot", get(screenshot))
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

fn create_web_dashboard() -> Router {
    Router::new()
        // ── Web dashboard ────────────────────────────────────────────────────
        .route("/dashboard", get(dashboard))
        .route("/view.css", get(view_css))
        .route("/view.js", get(view_js))
}

pub fn init_api_server(cancel_token: CancellationToken) -> Result<()> {
    let config = get_config();
    if config.args.no_api {
        return Ok(());
    }
    init_start_time();
    let api_listener = crate::utils::get_listener(&config.server_address())?;
    let mut app = Router::new();
    app = app.nest("/api", create_api_router(cancel_token.clone()));
    if !config.args.no_dashboard {
        app = app.merge(create_web_dashboard());
    }
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
    if !config.args.no_dashboard {
        info!("Dashboard available at /");
    }
    crate::utils::print_local_ips(config.args.port);
    Ok(())
}
