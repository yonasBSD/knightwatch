use serde_json::Value;

#[derive(Debug, Clone, serde::Serialize)]
pub struct EventPayload {
    pub version: &'static str,
    pub event: &'static str,
    pub timestamp: String,
    pub data: Value,
}

impl EventPayload {
    pub fn new(event: &'static str, data: Value) -> Self {
        Self {
            version: crate::utils::get_version(),
            event,
            timestamp: crate::utils::now_rfc3339(),
            data,
        }
    }
}