use serde_json::{Value, json};

use crate::{
    docker_tracker::DockerTrackerEvent, process_tracker::ProcessTrackerEvent,
    system_resources::SystemResourcesEvent, systemd::SystemdEvent,
};

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
                    "root_pid": if let Some(root) = root { root.pid } else { 0 },
                    "child_count": children.len()
                }),
            ),
            ProcessTrackerEvent::WorkComplete { pid } => {
                ("process.work_complete", json!({ "pid": pid }))
            }
            ProcessTrackerEvent::ProcessKilled { pid, success } => (
                "process.process_killed",
                json!({ "pid": pid, "success": success }),
            ),
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

impl From<&SystemdEvent> for WebhookPayload {
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

impl From<&DockerTrackerEvent> for WebhookPayload {
    fn from(event: &DockerTrackerEvent) -> Self {
        let (event_name, data) = match event {
            DockerTrackerEvent::InitialSnapshot { containers } => (
                "docker.initial_snapshot",
                json!({
                    "container_count": containers.len(),
                    "containers": containers,
                }),
            ),
            DockerTrackerEvent::ContainersAppeared { containers } => (
                "docker.containers_appeared",
                json!({
                    "container_count": containers.len(),
                    "containers": containers,
                }),
            ),
            DockerTrackerEvent::ContainersDisappeared { containers } => (
                "docker.containers_disappeared",
                json!({
                    "container_count": containers.len(),
                    "containers": containers,
                }),
            ),
            DockerTrackerEvent::ContainerStatusChanged {
                container,
                previous,
            } => (
                "docker.container_status_changed",
                json!({
                    "id": container.id,
                    "name": container.name,
                    "image": container.image,
                    "status": container.status,
                    "previous_status": previous,
                }),
            ),
            DockerTrackerEvent::ContainerHealthChanged {
                container,
                previous,
            } => (
                "docker.container_health_changed",
                json!({
                    "id": container.id,
                    "name": container.name,
                    "image": container.image,
                    "health": container.health,
                    "previous_health": previous,
                }),
            ),
            DockerTrackerEvent::ContainerOomKilled { id, name } => (
                "docker.container_oom_killed",
                json!({
                    "id": id,
                    "name": name,
                }),
            ),
            DockerTrackerEvent::ContainerActionResult {
                id,
                name,
                action,
                success,
            } => (
                "docker.container_action_result",
                json!({
                    "id": id,
                    "name": name,
                    "action": action,
                    "success": success,
                }),
            ),
        };
        Self::new(event_name, data)
    }
}
