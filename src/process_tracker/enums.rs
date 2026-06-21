use serde::{Deserialize, Serialize};
use serde_json::json;
use sysinfo::ProcessStatus;
use tokio::sync::oneshot;

use super::structs::{ProcessSnapshot, ProcessTree};
use crate::types::Result;

/// Events emitted by the tracker on its broadcast bus.
/// Subscribers receive these without polling.
#[derive(Debug, Clone)]
pub enum ProcessTrackerEvent {
    /// Emitted on the very first tick; contains everything we found.
    InitialSnapshot {
        root: Option<ProcessSnapshot>,
        children: Vec<ProcessSnapshot>,
    },
    /// One or more new child processes appeared.
    ChildrenAppeared {
        pid: u32,
        children: Vec<ProcessSnapshot>,
    },
    /// One or more child PIDs exited.
    ChildrenExited {
        pid: u32,
        children: Vec<u32>,
    },
    /// All descendants have exited (root may still be alive).
    AllChildrenGone {
        pid: u32,
    },
    /// The root process itself has exited.
    RootExited {
        pid: u32,
    },
    WorkComplete {
        pid: u32,
    },
    /// A process was killed via a KillProcess or KillTree command.
    ProcessKilled {
        pid: u32,
        /// `false` if the signal was sent but the OS reported failure,
        /// or if the process was not found.
        success: bool,
    },
}

impl From<&ProcessTrackerEvent> for crate::events::EventPayload {
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

/// One-shot queries callers can send to read tracker state synchronously.
#[derive(Debug)]
pub enum ProcessTrackerQuery {
    /// Returns a snapshot of the root process (None if already gone).
    GetRoot {
        root_pid: u32,
        response: oneshot::Sender<Option<ProcessSnapshot>>,
    },
    /// Returns snapshots of all currently live descendants.
    GetChildren {
        root_pid: u32,
        response: oneshot::Sender<Vec<ProcessSnapshot>>,
    },
    /// Returns true when no live descendants remain.
    IsWorkDone {
        root_pid: u32,
        response: oneshot::Sender<Option<bool>>,
    },
    GetTopProcesses {
        by: SortKey,
        limit: usize,
        response: oneshot::Sender<Vec<ProcessSnapshot>>,
    },
    GetTrackedPids {
        response: oneshot::Sender<Vec<u32>>,
    },
    GetProcessTree {
        root_pid: u32,
        response: oneshot::Sender<Option<ProcessTree>>,
    },
    GetAllProcessTrees {
        response: oneshot::Sender<Vec<ProcessTree>>,
    },
    GetProcessStatus {
        root_pid: u32,
        response: oneshot::Sender<Option<super::structs::ProcessStatus>>,
    },
}

#[derive(Debug, Serialize, Clone, Copy, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessSignal {
    Kill,
    #[serde(rename = "int")]
    Interrupt,
    Stop,
    #[serde(rename = "cont")]
    Continue,
    Term,
}

impl std::fmt::Display for ProcessSignal {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Kill => write!(f, "kill"),
            Self::Interrupt => write!(f, "int"),
            Self::Stop => write!(f, "stop"),
            Self::Continue => write!(f, "cont"),
            Self::Term => write!(f, "term"),
        }
    }
}

impl TryFrom<&str> for ProcessSignal {
    type Error = String;

    fn try_from(signal: &str) -> Result<Self, Self::Error> {
        match signal {
            "kill" => Ok(Self::Kill),
            "int" => Ok(Self::Interrupt),
            "stop" => Ok(Self::Stop),
            "cont" => Ok(Self::Continue),
            "term" => Ok(Self::Term),
            _ => Err(format!("Invalid signal: '{signal}'.")),
        }
    }
}

impl ProcessSignal {
    /// Returns `None` on Windows for any signal other than Kill,
    /// since only forceful termination is supported there.
    pub fn sysinfo_signal(&self) -> Option<sysinfo::Signal> {
        #[cfg(windows)]
        {
            // Windows only supports Kill; everything else is a no-op.
            match self {
                ProcessSignal::Kill => Some(sysinfo::Signal::Kill),
                _ => None,
            }
        }

        #[cfg(not(windows))]
        {
            Some(match self {
                ProcessSignal::Kill => sysinfo::Signal::Kill,
                ProcessSignal::Interrupt => sysinfo::Signal::Interrupt,
                ProcessSignal::Stop => sysinfo::Signal::Stop,
                ProcessSignal::Continue => sysinfo::Signal::Continue,
                ProcessSignal::Term => sysinfo::Signal::Term,
            })
        }
    }

    /// True if this signal is deliverable on the current platform.
    pub fn is_supported(&self) -> bool {
        #[cfg(windows)]
        {
            // Windows only supports Kill; everything else is a no-op.
            matches!(self, ProcessSignal::Kill)
        }
        #[cfg(not(windows))]
        {
            true
        }
    }

    /// Returns a list of supported signal based on current platform.
    pub fn get_supported_signals() -> Vec<ProcessSignal> {
        #[cfg(windows)]
        {
            // Windows only supports Kill; everything else is a no-op.
            vec![Self::Kill]
        }
        #[cfg(not(windows))]
        {
            vec![
                Self::Kill,
                Self::Interrupt,
                Self::Stop,
                Self::Continue,
                Self::Term,
            ]
        }
    }
}

/// Mutating commands that alter tracker state or act on live processes.
/// These require `&mut self` and travel on a separate channel from read-only queries.
#[derive(Debug)]
pub enum ProcessTrackerCommand {
    /// Send an arbitrary signal to a single process.
    /// Responds with `Ok(true)` on success, `Ok(false)` if the signal was
    /// delivered but the OS reported failure, or `Err` if the PID was not found.
    KillProcess {
        pid: u32,
        signal: ProcessSignal,
        response: oneshot::Sender<Result<bool>>,
    },
    /// Kill a root process and every descendant in its subtree.
    /// Responds with the list of PIDs that were successfully signalled.
    KillTree {
        root_pid: u32,
        response: oneshot::Sender<Result<Vec<u32>>>,
    },
    /// Begin tracking a new root PID. A no-op if the PID is already tracked.
    TrackPid {
        pid: u32,
        response: oneshot::Sender<Result<()>>,
    },
    /// Stop tracking a root PID and discard its state.
    UntrackPid {
        pid: u32,
        response: oneshot::Sender<Result<()>>,
    },
    /// Replace the polling interval and restart the tick timer immediately.
    SetPollInterval {
        interval: std::time::Duration,
        response: oneshot::Sender<Result<()>>,
    },
    /// Stop emitting ticks; the tracker keeps running and still handles queries/commands.
    PausePoll {
        response: oneshot::Sender<Result<()>>,
    },
    /// Resume ticking at the current poll interval.
    ResumePoll {
        response: oneshot::Sender<Result<()>>,
    },
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum ProcessState {
    Running,
    Sleeping,
    Other(String),
    Gone,
}

impl From<ProcessStatus> for ProcessState {
    fn from(status: ProcessStatus) -> Self {
        match status {
            ProcessStatus::Run => ProcessState::Running,
            ProcessStatus::Sleep | ProcessStatus::Idle => ProcessState::Sleeping,
            other => ProcessState::Other(format!("{other:?}")),
        }
    }
}

impl std::fmt::Display for ProcessState {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ProcessState::Running => write!(f, "running"),
            ProcessState::Sleeping => write!(f, "sleeping"),
            ProcessState::Other(s) => write!(f, "other({s})"),
            ProcessState::Gone => write!(f, "gone"),
        }
    }
}

#[cfg(target_os = "linux")]
#[derive(Debug, Serialize, Clone)]
pub enum FDType {
    File,
    Socket,
    Pipe,
    Other,
}

#[cfg(target_os = "linux")]
impl std::fmt::Display for FDType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            FDType::File => "file",
            FDType::Socket => "socket",
            FDType::Pipe => "pipe",
            FDType::Other => "other",
        };
        write!(f, "{s}")
    }
}

#[cfg(target_os = "linux")]
impl From<procfs::process::FDTarget> for FDType {
    fn from(fd_target: procfs::process::FDTarget) -> Self {
        use procfs::process::FDTarget;
        match fd_target {
            FDTarget::Path(_) => Self::File,
            FDTarget::Socket(_) => Self::Socket,
            FDTarget::Pipe(_) => Self::Pipe,
            _ => Self::Other,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SortKey {
    Memory,
    Cpu,
    Disk,
}

impl std::fmt::Display for SortKey {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Memory => write!(f, "Memory"),
            Self::Cpu => write!(f, "Cpu"),
            Self::Disk => write!(f, "Disk"),
        }
    }
}
