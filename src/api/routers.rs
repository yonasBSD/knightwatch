use axum::{
    Router, middleware,
    routing::{get, post},
};
use tokio_util::sync::CancellationToken;

use super::{end_points::*, middleware::auth_middleware};

fn create_auth_router() -> Router {
    Router::new()
        .route("/login", post(login))
        .route("/logout", post(logout))
}

fn create_common_router() -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/info", get(info))
}

fn create_api_router(cancel_token: CancellationToken, auth_layer: bool) -> Router {
    let api = Router::new()
        .route("/shutdown", post(shutdown))
        // ── Screenshot ────────────────────────────────────────────────────
        .route("/screenshot", get(screenshot))
        // ── Process tracking ──────────────────────────────────────────────
        .route("/root_pids", get(root_pids)) // root pids
        .route("/process/{root_pid}", get(process_tree)) // full tree
        .route("/process/root/{root_pid}", get(process_root)) // root only
        .route("/process/children/{root_pid}", get(process_children)) // children only
        .route("/process/status/{root_pid}", get(process_status)) // lightweight summary
        .route("/top-processes", get(top_processes)) // top processes
        .route("/supported-signals", get(supported_signals)) // supported signals
        // ── System Resources ──────────────────────────────────────────────
        .route("/system", get(system_snapshot)) // full system snapshot
        .route("/cpu", get(cpu_snapshot)) // cpu snapshot
        .route("/memory", get(memory_snapshot)) // memory snapshot
        .route("/disks", get(disks_snapshots)) // disks snapshot
        .route("/networks", get(networks_snapshot)) // networks snapshot
        .route("/gpus", get(gpus_snapshots)) // gpus snapshot
        .route("/battery", get(battery_snapshot)) // battery snapshot
        .route("/host-info", get(host_info_snapshot)) // host info snapshot
        .route("/temperatures", get(temperatures_snapshots)) // temperatures snapshot
        // ── Systemd ───────────────────────────────────────────────────────
        .route("/systemd", get(systemd_snapshot)) // systemd snapshot
        .route("/unit/{unit_name}", get(unit_snapshot)) // unit snapshot
        .route("/units/{unit_state}", get(units_by_active_state)) // units by active state
        .route("/failed_units", get(failed_units)) // failed_units
        .with_state(cancel_token);
    if auth_layer {
        api.layer(middleware::from_fn(auth_middleware))
    } else {
        api
    }
}

fn create_process_commands_router() -> Router {
    Router::new()
        .route("/process/kill/{pid}", post(kill_process))
        .route("/process/kill-tree/{root_pid}", post(kill_tree))
        .route("/process/track/{pid}", post(track_pid))
        .route("/process/untrack/{pid}", post(untrack_pid))
        .route("/process/poll/pause", post(pause_poll))
        .route("/process/poll/resume", post(resume_poll))
        .route("/process/poll/interval", post(set_poll_interval))
        .layer(middleware::from_fn(auth_middleware))
}

pub fn create_routers(
    config: &crate::config::AppConfig,
    cancel_token: CancellationToken,
) -> Router {
    let api_router = create_api_router(cancel_token, config.args.enable_auth);
    let mut app = Router::new()
        .nest("/api", api_router)
        .nest("/api", create_common_router());
    if config.args.enable_auth || config.args.allow_process_commands {
        super::session::init_sessions();
        app = app.nest("/api/auth", create_auth_router());
    }
    if config.args.allow_process_commands {
        app = app.nest("/api", create_process_commands_router());
    }
    app
}
