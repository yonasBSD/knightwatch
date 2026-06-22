#![allow(dead_code)]

use serde::{Deserialize, Serialize};

use crate::prelude::warn;

/// One unit row — equivalent to a line in `systemctl list-units`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitSnapshot {
    pub unit_name: String, // e.g. "nginx.service"
    pub unit_type: UnitType,
    pub load_state: UnitLoadState,
    pub active_state: UnitActiveState,
    pub sub_state: String, // e.g. "running", "dead", "waiting" — freeform from systemd
    pub description: String,

    // Only populated for .service units that are active
    pub main_pid: Option<u32>,
    pub memory_bytes: Option<u64>,
    pub cpu_usage_ns: Option<u64>, // CPUUsageNSec from D-Bus
    pub restart_count: Option<u32>,
    pub since: Option<String>, // rfc3339 of last state change (ActiveEnterTimestamp)

    // Fragment path — useful for linking to unit file location
    pub fragment_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemdSnapshot {
    pub timestamp: String,
    pub units: Vec<UnitSnapshot>,
    pub failed_count: u32,
    pub active_count: u32,
    pub inactive_count: u32,
}

pub struct UnitFilter {
    pub types: Option<Vec<UnitType>>,
    pub active_states: Option<Vec<UnitActiveState>>,
    pub name_prefix: Option<String>,
    pub include_failed: bool,
}

impl Default for UnitFilter {
    fn default() -> Self {
        Self {
            types: Some(vec![UnitType::Service]),
            active_states: None,
            name_prefix: None,
            include_failed: true,
        }
    }
}

impl UnitFilter {
    /// Returns true if this unit should be included in the snapshot.
    pub fn matches(&self, unit_type: &UnitType, active_state: &str, unit_name: &str) -> bool {
        // Always include failed units if the flag is set
        let is_failed = active_state == "failed";
        if is_failed && self.include_failed {
            return true;
        }

        // Type filter
        if let Some(ref allowed_types) = self.types
            && !allowed_types.iter().any(|t| t == unit_type)
        {
            return false;
        }

        // Active state filter
        if let Some(ref allowed_states) = self.active_states
            && !allowed_states.iter().any(|s| s.as_str() == active_state)
        {
            return false;
        }

        // Name prefix filter
        if let Some(ref prefix) = self.name_prefix
            && !unit_name.starts_with(prefix.as_str())
        {
            return false;
        }

        true
    }
}

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
