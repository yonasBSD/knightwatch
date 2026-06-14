use serde::Serialize;
use tokio::sync::oneshot;

use super::structs::*;
use crate::prelude::Result;

#[derive(Debug, Clone, Serialize)]
pub enum SystemResourcesEvent {
    /// Emitted on the very first tick; establishes a baseline for all metrics.
    InitialSnapshot { snapshot: SystemSnapshot },

    /// Emitted every tick with fresh readings for all subsystems.
    Tick { snapshot: SystemSnapshot },

    /// CPU usage crossed a threshold (aggregate across all cores).
    CpuThresholdExceeded { usage_percent: f32, threshold: f32 },

    /// Memory usage crossed a threshold.
    MemoryThresholdExceeded { used_percent: f32, threshold: f32 },

    /// A disk's used percentage crossed a threshold.
    DiskThresholdExceeded {
        mount_point: String,
        used_percent: f32,
        threshold: f32,
    },

    /// Battery is discharging and has fallen below a threshold.
    BatteryLow { charge_percent: f32, threshold: f32 },

    /// Battery state changed (e.g. plugged in / unplugged).
    BatteryStateChanged { state: BatteryState },
}

#[derive(Debug)]
pub enum SystemResourcesQuery {
    /// Returns the most recent full snapshot.
    Snapshot {
        response: oneshot::Sender<Option<SystemSnapshot>>,
    },

    /// Returns the most recent CPU reading only (cheaper to clone).
    Cpu {
        response: oneshot::Sender<Option<CpuSnapshot>>,
    },

    /// Returns the most recent memory reading.
    Memory {
        response: oneshot::Sender<Option<MemorySnapshot>>,
    },

    /// Returns the most recent per-disk readings.
    Disks {
        response: oneshot::Sender<Vec<DiskSnapshot>>,
    },

    /// Returns the most recent per-network-interface readings.
    Networks {
        response: oneshot::Sender<Vec<NetworkSnapshot>>,
    },

    /// Returns the most recent GPU readings (may be empty if unsupported).
    Gpus {
        response: oneshot::Sender<Vec<GpuSnapshot>>,
    },

    /// Returns the most recent battery snapshot (None if no battery present).
    Battery {
        response: oneshot::Sender<Option<BatterySnapshot>>,
    },

    /// Returns the host info (static — only changes on hostname/OS update).
    HostInfo {
        response: oneshot::Sender<Option<HostInfo>>,
    },

    /// Returns thermal readings (may be empty if unsupported).
    Temperatures {
        response: oneshot::Sender<Vec<ThermalSnapshot>>,
    },
}

#[derive(Debug)]
pub enum SystemResourcesCommand {
    /// Replace all alert thresholds at once.
    SetThresholds {
        thresholds: Thresholds,
        response: oneshot::Sender<Result<()>>,
    },

    /// Control which subsystems are refreshed each tick.
    SetRefreshMask {
        mask: RefreshMask,
        response: oneshot::Sender<Result<()>>,
    },
    /// Replace the polling interval and restart the tick timer immediately.
    SetPollInterval {
        interval: std::time::Duration,
        response: oneshot::Sender<Result<()>>,
    },
    /// Stop emitting ticks; the tracker keeps running and still handles queries/commands.
    PausePoll {
        response: oneshot::Sender<Result<()>>,
    },
    /// Resume ticking at the current poll interval.
    ResumePoll {
        response: oneshot::Sender<Result<()>>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum SystemHealth {
    /// Everything within normal thresholds.
    Healthy,
    /// One or more subsystems are elevated but not critical.
    Warning,
    /// One or more subsystems are at critical levels.
    Critical,
}

impl std::fmt::Display for SystemHealth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Healthy => write!(f, "healthy"),
            Self::Warning => write!(f, "warning"),
            Self::Critical => write!(f, "critical"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum BatteryState {
    Charging,
    Discharging,
    Full,
    /// Battery present but state cannot be determined.
    Unknown,
}

impl std::fmt::Display for BatteryState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Charging => write!(f, "charging"),
            Self::Discharging => write!(f, "discharging"),
            Self::Full => write!(f, "full"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

impl From<starship_battery::State> for BatteryState {
    fn from(state: starship_battery::State) -> Self {
        match state {
            starship_battery::State::Charging => Self::Charging,
            starship_battery::State::Discharging => Self::Discharging,
            starship_battery::State::Full => Self::Full,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum DiskKind {
    Ssd,
    Hdd,
    /// e.g. RAM disk, network mount, fuse, etc.
    Unknown,
}

impl From<sysinfo::DiskKind> for DiskKind {
    fn from(kind: sysinfo::DiskKind) -> Self {
        match kind {
            sysinfo::DiskKind::HDD => Self::Hdd,
            sysinfo::DiskKind::SSD => Self::Ssd,
            _ => Self::Unknown,
        }
    }
}

impl std::fmt::Display for DiskKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ssd => write!(f, "SSD"),
            Self::Hdd => write!(f, "HDD"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}
