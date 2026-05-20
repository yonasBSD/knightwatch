#![allow(dead_code)]

use serde::{Deserialize, Serialize};

use super::structs::{SystemdSnapshot, UnitSnapshot};
use crate::prelude::warn;

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
        response: tokio::sync::oneshot::Sender<Option<SystemdSnapshot>>,
    },
    Unit {
        unit_name: String,
        response: tokio::sync::oneshot::Sender<Option<UnitSnapshot>>,
    },
    ByActiveState {
        state: UnitActiveState,
        response: tokio::sync::oneshot::Sender<Vec<UnitSnapshot>>,
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
