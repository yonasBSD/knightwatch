use crate::{
    process_tracker::enums::ProcessTrackerEvent, system_monitor::enums::SystemMonitorEvent,
    utils::now_rfc3339,
};

#[derive(Debug, serde::Serialize)]
pub struct WebhookPayload {
    pub version: &'static str,
    pub event: &'static str,
    pub timestamp: String,
    pub data: serde_json::Value,
}

impl From<&ProcessTrackerEvent> for WebhookPayload {
    fn from(event: &ProcessTrackerEvent) -> Self {
        let (event_name, data) = match event {
            ProcessTrackerEvent::RootExited { pid } => {
                ("process.root_exited", serde_json::json!({ "pid": pid }))
            }
            ProcessTrackerEvent::ChildrenExited { pid, children } => (
                "process.children_exited",
                serde_json::json!({ "pid": pid, "children": children }),
            ),
            ProcessTrackerEvent::ChildrenAppeared { pid, children } => (
                "process.children_appeared",
                serde_json::json!({ "pid": pid, "children": children }),
            ),
            ProcessTrackerEvent::AllChildrenGone { pid } => (
                "process.all_children_gone",
                serde_json::json!({ "pid": pid }),
            ),
            ProcessTrackerEvent::InitialSnapshot { root, children } => (
                "process.initial_snapshot",
                serde_json::json!({
                    "root_pid": root.pid,
                    "child_count": children.len()
                }),
            ),
            ProcessTrackerEvent::WorkComplete { pid } => {
                ("process.work_complete", serde_json::json!({ "pid": pid }))
            }
        };
        Self {
            version: env!("CARGO_PKG_VERSION"),
            event: event_name,
            timestamp: now_rfc3339(),
            data,
        }
    }
}

impl From<&SystemMonitorEvent> for WebhookPayload {
    fn from(event: &SystemMonitorEvent) -> Self {
        let (event_name, data) = match event {
            SystemMonitorEvent::InitialSnapshot { snapshot } => (
                "system.initial_snapshot",
                serde_json::json!({ "snapshot": snapshot }),
            ),
            SystemMonitorEvent::Tick { snapshot } => {
                ("system.tick", serde_json::json!({ "snapshot": snapshot }))
            }
            SystemMonitorEvent::CpuThresholdExceeded {
                usage_percent,
                threshold,
            } => (
                "system.cpu_threshold_exceeded",
                serde_json::json!({ "usage_percent": usage_percent, "threshold": threshold }),
            ),
            SystemMonitorEvent::MemoryThresholdExceeded {
                used_percent,
                threshold,
            } => (
                "system.memory_threshold_exceeded",
                serde_json::json!({ "usage_percent": used_percent, "threshold": threshold }),
            ),
            SystemMonitorEvent::DiskThresholdExceeded {
                mount_point,
                used_percent,
                threshold,
            } => (
                "system.disk_threshold_exceeded",
                serde_json::json!({ "mount_point": mount_point, "usage_percent": used_percent, "threshold": threshold }),
            ),
            SystemMonitorEvent::BatteryLow {
                charge_percent,
                threshold,
            } => (
                "system.battery_low",
                serde_json::json!({ "charge_percent": charge_percent, "threshold": threshold }),
            ),
            SystemMonitorEvent::BatteryStateChanged { state } => (
                "system.battery_state_changed",
                serde_json::json!({ "state": state }),
            ),
        };
        Self {
            version: env!("CARGO_PKG_VERSION"),
            event: event_name,
            timestamp: now_rfc3339(),
            data,
        }
    }
}
