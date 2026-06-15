use tokio::sync::oneshot;

use crate::prelude::Result;

#[derive(Debug)]
pub enum ScreenCaptureQuery {
    GetScreenshots {
        response: tokio::sync::oneshot::Sender<Vec<super::structs::Screenshot>>,
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
