use serde::Serialize;
use std::collections::HashSet;
use tokio::sync::{broadcast, mpsc};

use super::enums::*;

// Linux-only structures
#[cfg(target_os = "linux")]
#[derive(Debug, Serialize, Clone)]
pub struct FileDescriptorInfo {
    pub fd: i32,
    pub target: String,
    pub fd_type: FDType,
}

#[cfg(target_os = "linux")]
impl From<procfs::process::FDInfo> for FileDescriptorInfo {
    fn from(fd_info: procfs::process::FDInfo) -> Self {
        Self {
            fd: fd_info.fd,
            target: format!("{:?}", fd_info.target),
            fd_type: fd_info.target.into(),
        }
    }
}

#[cfg(target_os = "linux")]
#[derive(Debug, Serialize, Clone, Copy)]
pub struct IOStats {
    pub read_bytes: u64,
    pub write_bytes: u64,
    pub read_chars: u64,
    pub write_chars: u64,
}

#[cfg(target_os = "linux")]
impl From<procfs::process::Io> for IOStats {
    fn from(io: procfs::process::Io) -> Self {
        Self {
            read_bytes: io.read_bytes,
            write_bytes: io.write_bytes,
            read_chars: io.rchar,
            write_chars: io.wchar,
        }
    }
}

pub struct RootProcess {
    #[allow(unused)]
    pub root_pid: u32,
    pub first_tick: bool,
    pub root_appeared: bool,
    pub prev_child_pids: HashSet<u32>,
    pub work_done: bool,
    pub root_exited: bool,
    pub children_ever_seen: bool,
    pub last_root: Option<ProcessSnapshot>,
    pub last_children: Vec<ProcessSnapshot>,
}

impl RootProcess {
    pub fn new(root_pid: u32) -> Self {
        Self {
            root_pid,
            first_tick: true,
            root_appeared: false,
            prev_child_pids: HashSet::new(),
            work_done: false,
            root_exited: false,
            children_ever_seen: false,
            last_root: None,
            last_children: Vec::new(),
        }
    }
}

/// Lightweight per-process data captured each tick.
#[derive(Debug, Clone, Serialize)]
pub struct ProcessSnapshot {
    pub pid: u32,
    pub name: String,
    pub state: ProcessState,
    pub cpu_usage: f32,
    pub memory_bytes: u64,
    pub disk_usage: u64,

    // Optional fields only available on Linux
    #[cfg(target_os = "linux")]
    pub cwd: Option<String>,
    #[cfg(target_os = "linux")]
    pub cmdline: Vec<String>,
    #[cfg(target_os = "linux")]
    pub open_files: Vec<FileDescriptorInfo>,
    #[cfg(target_os = "linux")]
    pub io_stats: Option<IOStats>,
}

impl From<&sysinfo::Process> for ProcessSnapshot {
    fn from(process: &sysinfo::Process) -> Self {
        let pid = process.pid().as_u32();
        #[cfg(target_os = "linux")]
        let (cwd, cmdline) = super::utils::collect_extended_info(pid);
        Self {
            pid,
            name: process.name().to_string_lossy().into_owned(),
            state: ProcessState::from(process.status()),
            cpu_usage: process.cpu_usage(),
            memory_bytes: process.memory(),
            disk_usage: super::utils::disk_usage_total(process.disk_usage()),
            #[cfg(target_os = "linux")]
            cwd,
            #[cfg(target_os = "linux")]
            cmdline,
            #[cfg(target_os = "linux")]
            open_files: super::utils::collect_file_descriptors(pid),
            #[cfg(target_os = "linux")]
            io_stats: super::utils::collect_io_stats(pid),
        }
    }
}

pub struct ProcessTrackerChannels {
    pub query_tx: mpsc::Sender<ProcessTrackerQuery>,
    pub query_rx: Option<mpsc::Receiver<ProcessTrackerQuery>>,
    pub command_tx: mpsc::Sender<ProcessTrackerCommand>,
    pub command_rx: Option<mpsc::Receiver<ProcessTrackerCommand>>,
    pub event_tx: broadcast::Sender<ProcessTrackerEvent>,
}

#[derive(Debug, Serialize)]
pub struct ProcessTree {
    pub root: Option<ProcessSnapshot>,
    pub children: Vec<ProcessSnapshot>,
    pub child_count: usize,
    pub work_done: bool,
    pub timestamp: String,
}

#[derive(Debug, Serialize)]
pub struct ProcessStatus {
    pub root_alive: bool,
    pub root_pid: Option<u32>,
    pub root_name: Option<String>,
    pub child_count: usize,
    pub work_done: bool,
    pub timestamp: String,
}
