use bollard::config::ContainerSummaryStateEnum;
use serde::{Deserialize, Serialize};

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

// ============================================================================
// Container status & health
// ============================================================================

/// Docker container lifecycle state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContainerStatus {
    Created,
    Running,
    Paused,
    Restarting,
    Removing,
    Exited,
    Dead,
    Stopping,
    Unknown(String),
}

impl ContainerStatus {
    pub fn from_state_enum(state: Option<&ContainerSummaryStateEnum>) -> Self {
        match state {
            Some(ContainerSummaryStateEnum::CREATED) => Self::Created,
            Some(ContainerSummaryStateEnum::RUNNING) => Self::Running,
            Some(ContainerSummaryStateEnum::PAUSED) => Self::Paused,
            Some(ContainerSummaryStateEnum::RESTARTING) => Self::Restarting,
            Some(ContainerSummaryStateEnum::REMOVING) => Self::Removing,
            Some(ContainerSummaryStateEnum::EXITED) => Self::Exited,
            Some(ContainerSummaryStateEnum::DEAD) => Self::Dead,
            Some(ContainerSummaryStateEnum::STOPPING) => Self::Stopping,
            Some(ContainerSummaryStateEnum::EMPTY) | None => Self::Unknown("empty".to_owned()),
        }
    }
}

impl std::fmt::Display for ContainerStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Created => write!(f, "created"),
            Self::Running => write!(f, "running"),
            Self::Paused => write!(f, "paused"),
            Self::Restarting => write!(f, "restarting"),
            Self::Removing => write!(f, "removing"),
            Self::Exited => write!(f, "exited"),
            Self::Dead => write!(f, "dead"),
            Self::Stopping => write!(f, "stopping"),
            Self::Unknown(s) => write!(f, "unknown({s})"),
        }
    }
}

/// Docker health-check result for a container.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContainerHealth {
    /// No HEALTHCHECK defined in the image.
    None,
    /// Health check is still running for the first time.
    Starting,
    Healthy,
    Unhealthy,
    Unknown,
}

impl ContainerHealth {
    /// Parse from the `Status` string in `ContainerSummary.status`, which looks
    /// like `"Up 3 hours (healthy)"` or `"Up 5 minutes (unhealthy)"`.
    pub fn from_status_str(status: &str) -> Self {
        let lower = status.to_lowercase();
        if lower.contains("(healthy)") {
            Self::Healthy
        } else if lower.contains("(unhealthy)") {
            Self::Unhealthy
        } else if lower.contains("(health: starting)") {
            Self::Starting
        } else {
            Self::None
        }
    }
}

impl std::fmt::Display for ContainerHealth {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "none"),
            Self::Starting => write!(f, "starting"),
            Self::Healthy => write!(f, "healthy"),
            Self::Unhealthy => write!(f, "unhealthy"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

/// The action carried out by a command — included in `ContainerActionResult`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContainerAction {
    Stop,
    Kill,
    Start,
    Restart,
    Pause,
    Unpause,
}

impl std::fmt::Display for ContainerAction {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Stop => write!(f, "stop"),
            Self::Kill => write!(f, "kill"),
            Self::Start => write!(f, "start"),
            Self::Restart => write!(f, "restart"),
            Self::Pause => write!(f, "pause"),
            Self::Unpause => write!(f, "unpause"),
        }
    }
}

/// Sort key for `GetTopContainers` — subset of `SortKey` relevant to containers.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DockerSortKey {
    Cpu,
    Memory,
}

impl std::fmt::Display for DockerSortKey {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Cpu => write!(f, "cpu"),
            Self::Memory => write!(f, "memory"),
        }
    }
}
