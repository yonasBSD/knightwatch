use crate::process_tracker::enums::ProcessTrackerEvent;

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
            timestamp: crate::utils::now_rfc3339(),
            data,
        }
    }
}
