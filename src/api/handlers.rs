use axum::{
    Router,
    body::Body,
    http::StatusCode,
    response::Response,
    routing::{get, post},
};
use std::{sync::OnceLock, time::Instant};
use tokio_util::sync::CancellationToken;

use super::{end_points::*, models::Vite};
use crate::prelude::*;

pub static START_TIME: OnceLock<Instant> = OnceLock::new();

fn init_start_time() {
    START_TIME.get_or_init(Instant::now);
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

#[cfg(debug_assertions)]
async fn serve_dashboard(uri: axum::http::Uri) -> Response {
    let vite_url = match uri.query() {
        Some(q) => format!("http://localhost:5173{}?{}", uri.path(), q),
        None => format!("http://localhost:5173{}", uri.path()),
    };
    match reqwest::Client::new().get(&vite_url).send().await {
        Ok(res) => {
            let status = res.status();
            let headers = res.headers().clone();
            let bytes = res.bytes().await.unwrap_or_default();
            let mut builder = Response::builder().status(status);
            if let Some(ct) = headers.get(reqwest::header::CONTENT_TYPE) {
                builder = builder.header(reqwest::header::CONTENT_TYPE, ct);
            }
            builder.body(Body::from(bytes)).unwrap()
        }
        Err(_) => Response::builder()
            .status(StatusCode::BAD_GATEWAY)
            .body(Body::from("Vite dev server not running on :5173"))
            .unwrap(),
    }
}

#[cfg(not(debug_assertions))]
async fn serve_dashboard(uri: axum::http::Uri) -> Response {
    use super::models::DashboardAssets;
    let path = uri.path().trim_start_matches('/');
    let is_spa_route = path == "dashboard" || path == "index.html" || path.is_empty();
    let asset_path = if is_spa_route { "index.html" } else { path };
    if !is_spa_route && DashboardAssets::get(asset_path).is_none() {
        return Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("404 Not Found"))
            .unwrap();
    }
    match DashboardAssets::get(asset_path) {
        Some(content) => {
            let mime = mime_guess::from_path(asset_path)
                .first_or_octet_stream()
                .to_string();
            Response::builder()
                .status(StatusCode::OK)
                .header(reqwest::header::CONTENT_TYPE, mime)
                .body(Body::from(content.data))
                .unwrap()
        }
        None => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("404 Not Found"))
            .unwrap(),
    }
}

pub fn init_api_server(cancel_token: CancellationToken) -> Result<Option<Vite>> {
    let config = get_config();
    if config.args.no_api {
        return Ok(None);
    }
    init_start_time();
    let api_listener = crate::utils::get_listener(&config.server_address())?;
    let mut app = Router::new();
    app = app.nest("/api", create_api_router(cancel_token.clone()));
    #[allow(unused_mut)]
    let mut vite = None;
    if !config.args.no_dashboard {
        #[cfg(debug_assertions)]
        {
            vite = crate::utils::start_dev_server().map(|child_process| Vite { child_process });
        }
        app = app.fallback(serve_dashboard);
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
    crate::utils::print_local_ips(config.args.port);
    info!("API server started");
    if !config.args.no_dashboard {
        info!("Dashboard available at /");
    }
    Ok(vite)
}
