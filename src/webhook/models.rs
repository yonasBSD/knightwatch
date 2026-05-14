use serde_json::{Value, json};

use crate::{process_tracker::ProcessTrackerEvent, system_resources::SystemResourcesEvent};

#[derive(Debug, serde::Serialize)]
pub struct WebhookPayload {
    pub version: &'static str,
    pub event: &'static str,
    pub timestamp: String,
    pub data: Value,
}

impl WebhookPayload {
    pub fn new(event: &'static str, data: Value) -> Self {
        Self {
            version: crate::utils::get_version(),
            event,
            timestamp: crate::utils::now_rfc3339(),
            data,
        }
    }
}

impl From<&ProcessTrackerEvent> for WebhookPayload {
    fn from(event: &ProcessTrackerEvent) -> Self {
        let (event_name, data) = match event {
            ProcessTrackerEvent::RootExited { pid } => {
                ("process.root_exited", json!({ "pid": pid }))
            }
            ProcessTrackerEvent::ChildrenExited { pid, children } => (
                "process.children_exited",
                json!({ "pid": pid, "children": children }),
            ),
            ProcessTrackerEvent::ChildrenAppeared { pid, children } => (
                "process.children_appeared",
                json!({ "pid": pid, "children": children }),
            ),
            ProcessTrackerEvent::AllChildrenGone { pid } => {
                ("process.all_children_gone", json!({ "pid": pid }))
            }
            ProcessTrackerEvent::InitialSnapshot { root, children } => (
                "process.initial_snapshot",
                json!({
                    "root_pid": root.pid,
                    "child_count": children.len()
                }),
            ),
            ProcessTrackerEvent::WorkComplete { pid } => {
                ("process.work_complete", json!({ "pid": pid }))
            }
        };
        Self::new(event_name, data)
    }
}

impl From<&SystemResourcesEvent> for WebhookPayload {
    fn from(event: &SystemResourcesEvent) -> Self {
        let (event_name, data) = match event {
            SystemResourcesEvent::InitialSnapshot { snapshot } => (
                "resources.initial_snapshot",
                json!({ "snapshot": snapshot }),
            ),
            SystemResourcesEvent::Tick { snapshot } => {
                ("resources.tick", json!({ "snapshot": snapshot }))
            }
            SystemResourcesEvent::CpuThresholdExceeded {
                usage_percent,
                threshold,
            } => (
                "resources.cpu_threshold_exceeded",
                json!({ "usage_percent": usage_percent, "threshold": threshold }),
            ),
            SystemResourcesEvent::MemoryThresholdExceeded {
                used_percent,
                threshold,
            } => (
                "resources.memory_threshold_exceeded",
                json!({ "usage_percent": used_percent, "threshold": threshold }),
            ),
            SystemResourcesEvent::DiskThresholdExceeded {
                mount_point,
                used_percent,
                threshold,
            } => (
                "resources.disk_threshold_exceeded",
                json!({ "mount_point": mount_point, "usage_percent": used_percent, "threshold": threshold }),
            ),
            SystemResourcesEvent::BatteryLow {
                charge_percent,
                threshold,
            } => (
                "resources.battery_low",
                json!({ "charge_percent": charge_percent, "threshold": threshold }),
            ),
            SystemResourcesEvent::BatteryStateChanged { state } => {
                ("resources.battery_state_changed", json!({ "state": state }))
            }
        };
        Self::new(event_name, data)
    }
}
