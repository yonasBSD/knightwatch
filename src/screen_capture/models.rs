#![allow(dead_code)]

use tokio::sync::{mpsc, oneshot};

use crate::prelude::Result;

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

#[derive(Debug)]
pub enum ScreenCaptureQuery {
    GetScreenshots {
        response: tokio::sync::oneshot::Sender<Vec<Screenshot>>,
    },
}

/// Mutating commands that alter capture state or act on live processes.
/// These require `&mut self` and travel on a separate channel from read-only queries.
#[derive(Debug)]
pub enum ScreenCaptureCommand {
    /// Replace the polling interval and restart the tick timer immediately.
    SetPollInterval {
        interval: std::time::Duration,
        response: oneshot::Sender<Result<()>>,
    },
    /// Stop emitting ticks; the capture keeps running and still handles queries/commands.
    PausePoll {
        response: oneshot::Sender<Result<()>>,
    },
    /// Resume ticking at the current poll interval.
    ResumePoll {
        response: oneshot::Sender<Result<()>>,
    },
}
