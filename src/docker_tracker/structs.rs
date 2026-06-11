use serde::Serialize;
use tokio::sync::mpsc;

use super::enums::*;

/// Per-container resource stats captured each poll tick.
#[derive(Debug, Clone, Serialize)]
pub struct ContainerStats {
    /// CPU usage as a percentage of all available cores (0.0–100.0 * num_cpus).
    pub cpu_percent: f64,
    /// Current RSS-equivalent memory usage in bytes.
    pub memory_bytes: u64,
    /// Memory limit enforced by Docker (0 = unlimited / host RAM).
    pub memory_limit_bytes: u64,
    /// Memory usage as a fraction of the limit (0.0–1.0). None when limit is 0.
    pub memory_percent: Option<f64>,
    /// Cumulative bytes received across all network interfaces.
    pub net_rx_bytes: u64,
    /// Cumulative bytes transmitted across all network interfaces.
    pub net_tx_bytes: u64,
    /// Cumulative bytes read from block devices.
    pub block_read_bytes: u64,
    /// Cumulative bytes written to block devices.
    pub block_write_bytes: u64,
    /// Number of processes/threads inside the container (from pids_stats).
    pub pid_count: u64,
}

impl ContainerStats {
    /// Calculate CPU % from two consecutive raw stat frames.
    /// Docker's own formula: (cpu_delta / system_delta) * num_cpus * 100.
    pub fn from_bollard(stats: &bollard::config::ContainerStatsResponse) -> Self {
        let cpu_percent = super::utils::compute_cpu_percent(stats);

        let memory_bytes = stats
            .memory_stats
            .as_ref()
            .and_then(|m| m.usage)
            .unwrap_or(0);
        let memory_limit_bytes = stats
            .memory_stats
            .as_ref()
            .and_then(|m| m.limit)
            .unwrap_or(0);
        let memory_percent = if memory_limit_bytes > 0 {
            Some(memory_bytes as f64 / memory_limit_bytes as f64)
        } else {
            None
        };

        let (net_rx_bytes, net_tx_bytes) = stats
            .networks
            .as_ref()
            .map(|nets| {
                nets.values().fold((0, 0), |(rx, tx), iface| {
                    (
                        rx + iface.rx_bytes.unwrap_or(0),
                        tx + iface.tx_bytes.unwrap_or(0),
                    )
                })
            })
            .unwrap_or((0, 0));

        let (block_read_bytes, block_write_bytes) = stats
            .blkio_stats
            .as_ref()
            .and_then(|blkio| blkio.io_service_bytes_recursive.as_ref())
            .map(|entries| {
                entries.iter().fold((0, 0), |(r, w), entry| {
                    match entry
                        .op
                        .as_deref()
                        .unwrap_or_default()
                        .to_lowercase()
                        .as_str()
                    {
                        "read" => (r + entry.value.unwrap_or(0), w),
                        "write" => (r, w + entry.value.unwrap_or(0)),
                        _ => (r, w),
                    }
                })
            })
            .unwrap_or((0, 0));

        let pid_count = stats
            .pids_stats
            .as_ref()
            .and_then(|pids| pids.current)
            .unwrap_or(0);

        Self {
            cpu_percent,
            memory_bytes,
            memory_limit_bytes,
            memory_percent,
            net_rx_bytes,
            net_tx_bytes,
            block_read_bytes,
            block_write_bytes,
            pid_count,
        }
    }
}

/// Lightweight snapshot of a container captured each tick.
#[derive(Debug, Clone, Serialize)]
pub struct ContainerSnapshot {
    pub id: String,
    /// Short 12-char ID for display.
    pub short_id: String,
    /// Primary name (Docker strips the leading `/`).
    pub name: String,
    pub image: String,
    pub status: ContainerStatus,
    pub health: ContainerHealth,
    pub stats: Option<ContainerStats>,
}

impl ContainerSnapshot {
    pub fn from_summary(c: &bollard::models::ContainerSummary) -> Option<Self> {
        let id = c.id.clone()?;
        let short_id: String = id.chars().take(12).collect();
        let name = c
            .names
            .as_deref()
            .and_then(|n| n.first())
            .map(|n| n.trim_start_matches('/').to_owned())
            .unwrap_or_else(|| short_id.clone());
        let image = c.image.clone().unwrap_or_default();
        let status = ContainerStatus::from_state_enum(c.state.as_ref());
        let health = ContainerHealth::from_status_str(c.status.as_deref().unwrap_or(""));
        Some(Self {
            id,
            short_id,
            name,
            image,
            status,
            health,
            stats: None,
        })
    }
}

/// Channel bundle — mirrors `ProcessTrackerChannels`.
pub struct DockerTrackerChannels {
    pub query_tx: mpsc::Sender<DockerTrackerQuery>,
    pub query_rx: Option<mpsc::Receiver<DockerTrackerQuery>>,
    pub command_tx: mpsc::Sender<DockerTrackerCommand>,
    pub command_rx: Option<mpsc::Receiver<DockerTrackerCommand>>,
    pub event_tx: tokio::sync::broadcast::Sender<DockerTrackerEvent>,
}
