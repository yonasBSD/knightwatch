#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::oneshot;

use super::structs::{SystemdSnapshot, UnitSnapshot};
use crate::prelude::{Result, warn};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnitLoadState {
    Loaded,
    NotFound,
    BadSetting,
    Error,
    Masked,
}

impl From<&str> for UnitLoadState {
    fn from(s: &str) -> Self {
        match s {
            "loaded" => Self::Loaded,
            "not-found" => Self::NotFound,
            "bad-setting" => Self::BadSetting,
            "error" => Self::Error,
            "masked" => Self::Masked,
            other => {
                warn!(state = other, "unknown load state from systemd");
                Self::NotFound
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnitActiveState {
    Active,
    Reloading,
    Inactive,
    Failed,
    Activating,
    Deactivating,
}

impl UnitActiveState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Reloading => "reloading",
            Self::Inactive => "inactive",
            Self::Failed => "failed",
            Self::Activating => "activating",
            Self::Deactivating => "deactivating",
        }
    }
}

impl From<&str> for UnitActiveState {
    fn from(s: &str) -> Self {
        match s {
            "active" => Self::Active,
            "reloading" => Self::Reloading,
            "inactive" => Self::Inactive,
            "failed" => Self::Failed,
            "activating" => Self::Activating,
            "deactivating" => Self::Deactivating,
            other => {
                warn!(state = other, "unknown active state from systemd");
                Self::Inactive
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnitType {
    Service,
    Socket,
    Target,
    Timer,
    Mount,
    Device,
    Other(String),
}

impl UnitType {
    pub fn from_name(name: &str) -> Self {
        match name.rsplit_once('.').map(|(_, ext)| ext) {
            Some("service") => UnitType::Service,
            Some("socket") => UnitType::Socket,
            Some("target") => UnitType::Target,
            Some("timer") => UnitType::Timer,
            Some("mount") | Some("automount") => UnitType::Mount,
            Some("device") => UnitType::Device,
            Some(other) => UnitType::Other(other.to_string()),
            None => UnitType::Other(String::new()),
        }
    }
}

pub enum SystemdQuery {
    Snapshot {
        response: oneshot::Sender<Option<SystemdSnapshot>>,
    },
    Unit {
        unit_name: String,
        response: oneshot::Sender<Option<UnitSnapshot>>,
    },
    ByActiveState {
        state: UnitActiveState,
        response: oneshot::Sender<Vec<UnitSnapshot>>,
    },
}

#[derive(Debug, Clone)]
pub enum SystemdEvent {
    /// First successful D-Bus enumeration
    InitialSnapshot { snapshot: SystemdSnapshot },
    /// Normal poll tick
    Tick { snapshot: SystemdSnapshot },
    /// A unit transitioned into Failed — wire into Telegram/webhook
    UnitFailed {
        unit_name: String,
        previous_state: UnitActiveState,
    },
    /// A unit recovered out of Failed
    UnitRecovered { unit_name: String },
    /// A unit that wasn't in the list before appeared (e.g. transient unit)
    UnitAppeared { unit: UnitSnapshot },
    /// A unit disappeared entirely (unloaded/transient gone)
    UnitDisappeared { unit_name: String },
}

impl From<&SystemdEvent> for crate::events::EventPayload {
    fn from(event: &SystemdEvent) -> Self {
        let (event_name, data) = match event {
            SystemdEvent::InitialSnapshot { snapshot } => (
                "systemd.initial_snapshot",
                json!({
                    "timestamp": snapshot.timestamp,
                    "unit_count": snapshot.units.len(),
                    "failed_count": snapshot.failed_count,
                    "active_count": snapshot.active_count,
                    "inactive_count": snapshot.inactive_count,
                }),
            ),
            SystemdEvent::Tick { snapshot } => (
                "systemd.tick",
                json!({
                    "timestamp": snapshot.timestamp,
                    "unit_count": snapshot.units.len(),
                    "failed_count": snapshot.failed_count,
                    "active_count": snapshot.active_count,
                    "inactive_count": snapshot.inactive_count,
                }),
            ),
            SystemdEvent::UnitFailed {
                unit_name,
                previous_state,
            } => (
                "systemd.unit_failed",
                json!({
                    "unit_name": unit_name,
                    "previous_state": previous_state.as_str(),
                }),
            ),
            SystemdEvent::UnitRecovered { unit_name } => {
                ("systemd.unit_recovered", json!({ "unit_name": unit_name }))
            }
            SystemdEvent::UnitAppeared { unit } => (
                "systemd.unit_appeared",
                json!({
                    "unit_name": unit.unit_name,
                    "active_state": unit.active_state.as_str(),
                    "sub_state": unit.sub_state,
                    "description": unit.description,
                }),
            ),
            SystemdEvent::UnitDisappeared { unit_name } => (
                "systemd.unit_disappeared",
                json!({ "unit_name": unit_name }),
            ),
        };
        Self::new(event_name, data)
    }
}

/// Mutating commands that alter systemd state.
/// These require `&mut self` and travel on a separate channel from read-only queries.
#[derive(Debug)]
pub enum SystemdCommand {
    /// Replace the polling interval and restart the tick timer immediately.
    SetPollInterval {
        interval: std::time::Duration,
        response: oneshot::Sender<Result<()>>,
    },
    /// Stop emitting ticks; the systemd keeps running and still handles queries/commands.
    PausePoll {
        response: oneshot::Sender<Result<()>>,
    },
    /// Resume ticking at the current poll interval.
    ResumePoll {
        response: oneshot::Sender<Result<()>>,
    },
}
