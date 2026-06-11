use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;

use super::structs::ContainerSnapshot;
use crate::types::Result;

// ============================================================================
// Events
// ============================================================================

/// Events emitted on the broadcast bus — mirrors `ProcessTrackerEvent`.
#[derive(Debug, Clone)]
pub enum DockerTrackerEvent {
    /// Emitted once on the very first tick with every container currently running.
    InitialSnapshot {
        containers: Vec<ContainerSnapshot>,
    },

    /// One or more containers that weren't present last tick are now running.
    ContainersAppeared {
        containers: Vec<ContainerSnapshot>,
    },

    /// One or more containers that were present last tick are no longer listed.
    /// Carries the last known snapshot so callers have name/image context.
    ContainersDisappeared {
        containers: Vec<ContainerSnapshot>,
    },

    /// A container's lifecycle status changed (e.g. running → exited).
    ContainerStatusChanged {
        container: ContainerSnapshot,
        previous: ContainerStatus,
    },

    /// A container's health check result changed (healthy ↔ unhealthy ↔ starting).
    ContainerHealthChanged {
        container: ContainerSnapshot,
        previous: ContainerHealth,
    },

    /// A container was OOM-killed by the kernel. Derived from the Docker events
    /// stream (`oom` action) rather than poll diffing.
    ContainerOomKilled {
        id: String,
        name: String,
    },

    /// A container action was performed via a `DockerTrackerCommand`.
    ContainerActionResult {
        id: String,
        name: String,
        action: ContainerAction,
        success: bool,
    },
}

// ============================================================================
// Queries
// ============================================================================

/// Read-only queries — answered synchronously from cached state without
/// hitting the Docker daemon. Mirrors `ProcessTrackerQuery`.
#[derive(Debug)]
pub enum DockerTrackerQuery {
    /// Returns snapshots of all currently tracked containers.
    ListContainers {
        response: oneshot::Sender<Vec<ContainerSnapshot>>,
    },

    /// Returns the snapshot for a single container by ID or name.
    /// `None` if not currently tracked.
    GetContainer {
        id_or_name: String,
        response: oneshot::Sender<Option<ContainerSnapshot>>,
    },

    /// Returns the top N containers sorted by the given key.
    GetTopContainers {
        by: DockerSortKey,
        limit: usize,
        response: oneshot::Sender<Vec<ContainerSnapshot>>,
    },
}

// ============================================================================
// Commands
// ============================================================================

/// Mutating commands — require `&mut self` and travel on the command channel.
#[derive(Debug)]
pub enum DockerTrackerCommand {
    /// Stop a running container (graceful SIGTERM + timeout, then SIGKILL).
    StopContainer {
        id_or_name: String,
        /// Seconds to wait before killing. `None` uses Docker's default (10 s).
        timeout_secs: Option<i32>,
        response: oneshot::Sender<Result<()>>,
    },

    /// Immediately kill a container with SIGKILL (or a custom signal).
    KillContainer {
        id_or_name: String,
        /// e.g. `"SIGKILL"`, `"SIGTERM"`. `None` defaults to `"SIGKILL"`.
        signal: Option<String>,
        response: oneshot::Sender<Result<()>>,
    },

    /// Start a stopped container.
    StartContainer {
        id_or_name: String,
        response: oneshot::Sender<Result<()>>,
    },

    /// Restart a container (stop + start).
    RestartContainer {
        id_or_name: String,
        timeout_secs: Option<i32>,
        response: oneshot::Sender<Result<()>>,
    },

    /// Pause all processes in a container (SIGSTOP).
    PauseContainer {
        id_or_name: String,
        response: oneshot::Sender<Result<()>>,
    },

    /// Unpause a paused container.
    UnpauseContainer {
        id_or_name: String,
        response: oneshot::Sender<Result<()>>,
    },

    /// Replace the polling interval and restart the tick timer immediately.
    SetPollInterval {
        interval: std::time::Duration,
        response: oneshot::Sender<Result<()>>,
    },

    /// Suspend polling (event stream keeps running).
    PausePoll {
        response: oneshot::Sender<Result<()>>,
    },

    /// Resume polling at the current interval.
    ResumePoll {
        response: oneshot::Sender<Result<()>>,
    },
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
    // pub fn from_str(s: &str) -> Self {
    //     match s {
    //         "created" => Self::Created,
    //         "running" => Self::Running,
    //         "paused" => Self::Paused,
    //         "restarting" => Self::Restarting,
    //         "removing" => Self::Removing,
    //         "exited" => Self::Exited,
    //         "dead" => Self::Dead,
    //         "stopping" => Self::Stopping,
    //         other => Self::Unknown(other.to_owned()),
    //     }
    // }
    pub fn from_state_enum(
        state: Option<&bollard::config::ContainerSummaryStateEnum>,
    ) -> Self {
        use bollard::config::ContainerSummaryStateEnum;
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
