use tokio::sync::mpsc;

use super::enums::*;

#[derive(Debug, Clone, serde::Serialize)]
pub struct Screenshot {
    pub image: Vec<u8>,
    pub monitor_name: String,
    pub monitor_id: u32,
    pub width: u32,
    pub height: u32,
    pub timestamp: String,
}

pub struct ScreenCaptureChannels {
    pub query_tx: mpsc::Sender<ScreenCaptureQuery>,
    pub query_rx: Option<mpsc::Receiver<ScreenCaptureQuery>>,
    pub command_tx: mpsc::Sender<ScreenCaptureCommand>,
    pub command_rx: Option<mpsc::Receiver<ScreenCaptureCommand>>,
}
