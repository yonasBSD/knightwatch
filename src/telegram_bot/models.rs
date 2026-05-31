use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use teloxide::types::ChatId;

use super::utils::escape_mdv2;
use crate::{
    prelude::*,
    process_tracker::ProcessSignal,
    systemd::UnitActiveState,
    utils::{format_bytes, format_uptime},
};

#[derive(teloxide::utils::command::BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Available commands:")]
pub enum Command {
    #[command(description = "Start the bot and show the main menu")]
    Start,
    #[command(description = "Show the main menu")]
    Menu,
    #[command(description = "Show this help message")]
    Help,
    #[command(description = "Authenticate using token")]
    Auth,
    #[command(description = "Get Screenshot of all monitors")]
    Screenshot,
    #[command(description = "Get Process Info")]
    Process,
    #[command(description = "Get Top Processes Info")]
    TopProcesses,
    #[command(description = "Get System Snapshot")]
    SystemSnapshot,
    #[command(description = "Stop Knight Watch")]
    StopKnightWatch,
}

#[derive(Debug, Clone)]
pub struct State {
    chats: Arc<Mutex<HashMap<ChatId, Chat>>>,
    auth_enabled: bool,
}

impl State {
    pub fn new() -> Self {
        let chats = Arc::new(Mutex::new(HashMap::new()));
        let pre_auth_ids = get_users().get_telegram_chat_ids().into_iter().map(ChatId);
        if let Ok(mut g) = chats.lock() {
            for chat_id in pre_auth_ids {
                g.insert(chat_id, Chat::new_authed());
            }
        }
        Self {
            chats,
            auth_enabled: get_config().args.enable_auth,
        }
    }

    pub fn get_chat_ids(&self) -> Vec<ChatId> {
        self.chats
            .lock()
            .ok()
            .map(|g| g.keys().cloned().collect())
            .unwrap_or_default()
    }

    pub fn get_authed_chat_ids(&self) -> Vec<ChatId> {
        self.chats
            .lock()
            .ok()
            .map(|g| {
                g.iter()
                    .filter(|(_, chat)| chat.auth_state == AuthState::Authenticated)
                    .map(|(&id, _)| id)
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn get_relevant_chat_ids(&self) -> Vec<ChatId> {
        if self.auth_enabled {
            self.get_authed_chat_ids()
        } else {
            self.get_chat_ids()
        }
    }

    pub fn remove_chat(&mut self, chat_id: ChatId) {
        if let Ok(mut g) = self.chats.lock() {
            g.remove(&chat_id);
        }
    }

    pub fn get_chat_state(&self, chat_id: ChatId) -> ChatState {
        self.chats
            .lock()
            .ok()
            .and_then(|g| g.get(&chat_id).map(|c| c.chat_state.clone()))
            .unwrap_or(ChatState::Idle)
    }

    pub fn get_chat_auth(&self, chat_id: ChatId) -> AuthState {
        self.chats
            .lock()
            .ok()
            .and_then(|g| g.get(&chat_id).map(|c| c.auth_state.clone()))
            .unwrap_or(AuthState::Unauthenticated)
    }

    pub fn set_chat_state(&self, chat_id: ChatId, state: ChatState) {
        if let Ok(mut g) = self.chats.lock() {
            g.entry(chat_id).or_insert_with(Chat::new).chat_state = state;
        }
    }

    pub fn set_chat_auth(&self, chat_id: ChatId, auth: AuthState) {
        if let Ok(mut g) = self.chats.lock() {
            g.entry(chat_id).or_insert_with(Chat::new).auth_state = auth;
        }
    }

    pub fn set_chat_state_idle(&self, chat_id: ChatId) {
        self.set_chat_state(chat_id, ChatState::Idle);
    }

    pub fn is_authorized(&self, chat_id: ChatId) -> bool {
        if !self.auth_enabled {
            return true;
        }
        self.get_chat_auth(chat_id) == AuthState::Authenticated
    }

    pub fn is_authorized_to_commmand(&self, chat_id: ChatId) -> bool {
        self.get_chat_auth(chat_id) == AuthState::Authenticated
    }
}

#[derive(Debug, Clone)]
pub struct Chat {
    pub chat_state: ChatState,
    pub auth_state: AuthState,
}

impl Chat {
    pub fn new() -> Self {
        Self {
            chat_state: ChatState::Idle,
            auth_state: AuthState::Unauthenticated,
        }
    }

    pub fn new_authed() -> Self {
        Self {
            chat_state: ChatState::Idle,
            auth_state: AuthState::Authenticated,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Subsystem {
    ProcessTracker,
    ScreenCapture
}

impl Subsystem {
    pub fn label(&self) -> &'static str {
        match self {
            Subsystem::ProcessTracker => "Process Tracker",
            Subsystem::ScreenCapture => "Screen Capture",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ChatState {
    Idle,
    AwaitingUnitName,
    AwaitingAuthToken,
    AwaitingPollInterval { subsystem: Subsystem },
}

/// Inline-button callback actions, encoded as `"action:payload"` strings.
#[derive(Debug, Clone, PartialEq)]
pub enum ProcessCallbackAction {
    /// `"track:1234"`
    Track { pid: u32 },
    /// `"untrack:1234"`
    Untrack { pid: u32 },
    /// `"killtree:1234"`
    KillTree { pid: u32 },
    /// `"signal:1234:kill"` / `"signal:1234:term"` etc. — uses ProcessSignal's Display/TryFrom
    Signal { pid: u32, signal: ProcessSignal },
}

impl ProcessCallbackAction {
    pub fn encode(&self) -> String {
        match self {
            ProcessCallbackAction::Track { pid } => format!("track:{pid}"),
            ProcessCallbackAction::Untrack { pid } => format!("untrack:{pid}"),
            ProcessCallbackAction::KillTree { pid } => format!("killtree:{pid}"),
            ProcessCallbackAction::Signal { pid, signal } => format!("signal:{pid}:{signal}"),
        }
    }

    pub fn decode(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.splitn(3, ':').collect();
        match parts.as_slice() {
            ["track", pid] => Some(ProcessCallbackAction::Track {
                pid: pid.parse().ok()?,
            }),
            ["untrack", pid] => Some(ProcessCallbackAction::Untrack {
                pid: pid.parse().ok()?,
            }),
            ["killtree", pid] => Some(ProcessCallbackAction::KillTree {
                pid: pid.parse().ok()?,
            }),
            ["signal", pid, sig] => {
                let signal = ProcessSignal::try_from(*sig).ok()?;
                Some(ProcessCallbackAction::Signal {
                    pid: pid.parse().ok()?,
                    signal,
                })
            }
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum AuthState {
    Unauthenticated,
    Authenticated,
}

pub struct TelegramBot {
    pub shutdown_token: teloxide::dispatching::ShutdownToken,
}

impl TelegramBot {
    pub async fn shutdown(self) {
        match self.shutdown_token.shutdown() {
            Ok(fut) => fut.await,
            Err(err) => tracing::error!(err = err.to_string(), "Failed to shutdown telegram bot"),
        }
    }
}

pub struct TelegramDisplay<'a, T>(pub &'a T);

impl<'a> std::fmt::Display for TelegramDisplay<'a, crate::process_tracker::ProcessSnapshot> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let s = self.0;
        write!(
            f,
            "🔹 *{name}* `\\(PID {pid}\\)`\n   ├ State: `{state}`\n   ├ CPU: `{cpu:.1}%`\n   └ Mem: `{mem}`",
            pid = s.pid,
            name = escape_mdv2(&s.name),
            state = escape_mdv2(&s.state.to_string()),
            cpu = s.cpu_usage,
            mem = escape_mdv2(&format_bytes(s.memory_bytes)),
        )?;
        #[cfg(target_os = "linux")]
        {
            if let Some(cwd) = &s.cwd {
                write!(f, "\n   ├ CWD: `{}`", escape_mdv2(cwd))?;
            }
            write!(f, "\n   ├ FDs: `{}`", s.open_files.len())?;
            if let Some(io) = &s.io_stats {
                write!(
                    f,
                    "\n   ├ I/O Read: `{}` / Write: `{}`",
                    escape_mdv2(&io.read_bytes.to_string()),
                    escape_mdv2(&io.write_bytes.to_string()),
                )?;
            }
            if !s.cmdline.is_empty() {
                let cmd = s.cmdline.join(" ");
                write!(f, "\n   └ CMD: `{}`", escape_mdv2(&cmd))?;
            }
        }
        Ok(())
    }
}

impl<'a> std::fmt::Display for TelegramDisplay<'a, crate::process_tracker::ProcessTree> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let t = self.0; // Header
        let status_icon = if t.work_done { "✅" } else { "⏳" };
        writeln!(f, "{status_icon} *Process Tree*\n🕐 `{}`\n", t.timestamp)?;

        // Root process
        writeln!(f, "*Root Process*")?;
        match &t.root {
            Some(root) => writeln!(f, "{}", TelegramDisplay(root))?,
            None => writeln!(f, "_No root process_")?,
        }

        // Children
        if t.child_count == 0 {
            writeln!(f, "\n*Children:* _none_")?;
        } else {
            writeln!(f, "\n*Children* \\({}\\):", t.child_count)?;
            for child in &t.children {
                writeln!(f, "{}\n", TelegramDisplay(child))?;
            }
        }
        Ok(())
    }
}

impl<'a> std::fmt::Display for TelegramDisplay<'a, crate::system_resources::SystemSnapshot> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let s = self.0;

        // ── Header ──────────────────────────────────────────────────────────
        writeln!(
            f,
            "🖥️ *{host}* — `{ts}`",
            host = escape_mdv2(s.host.hostname.as_deref().unwrap_or("unknown")),
            ts = escape_mdv2(&s.timestamp),
        )?;

        // ── Host meta ───────────────────────────────────────────────────────
        writeln!(
            f,
            "├ OS: `{os}`\n\
             ├ Kernel: `{kernel}`\n\
             ├ Arch: `{arch}`\n\
             ├ Uptime: `{uptime}`\n\
             └ Processes: `{procs}`",
            os = escape_mdv2(s.host.os_name.as_deref().unwrap_or("?")),
            kernel = escape_mdv2(s.host.kernel_version.as_deref().unwrap_or("?")),
            arch = escape_mdv2(s.host.cpu_arch.as_deref().unwrap_or("?")),
            uptime = format_uptime(s.host.uptime_secs),
            procs = s.host.process_count,
        )?;

        // ── CPU ─────────────────────────────────────────────────────────────
        writeln!(
            f,
            "\n🔲 *CPU* — `{brand}`\n\
             ├ Usage: `{usage:.1}%`\n\
             └ Freq: `{freq} MHz`",
            brand = escape_mdv2(&s.cpu.brand),
            usage = s.cpu.usage_percent,
            freq = s.cpu.frequency_mhz,
        )?;

        #[cfg(any(target_os = "linux", target_os = "macos"))]
        writeln!(
            f,
            "   └ Load avg: `{:.2}` / `{:.2}` / `{:.2}`",
            s.cpu.load_avg.one, s.cpu.load_avg.five, s.cpu.load_avg.fifteen,
        )?;

        // ── Memory ──────────────────────────────────────────────────────────
        write!(
            f,
            "\n🧠 *Memory*\n\
             ├ RAM: `{used}` / `{total}` \\(`{pct:.1}%`\\)\n\
             └ Swap: `{sused}` / `{stotal}`",
            used = escape_mdv2(&format_bytes(s.memory.used_bytes)),
            total = escape_mdv2(&format_bytes(s.memory.total_bytes)),
            pct = s.memory.used_percent,
            sused = escape_mdv2(&format_bytes(s.memory.swap_used_bytes)),
            stotal = escape_mdv2(&format_bytes(s.memory.swap_total_bytes)),
        )?;
        if let Some(swap_pct) = s.memory.swap_used_percent {
            write!(f, " \\(`{:.1}%`\\)", swap_pct)?;
        }
        writeln!(f)?;

        // ── Disks ───────────────────────────────────────────────────────────
        if !s.disks.is_empty() {
            write!(f, "\n💾 *Disks*\n")?;
            let last = s.disks.len() - 1;
            for (i, disk) in s.disks.iter().enumerate() {
                let connector = if i == last { "└" } else { "├" };
                writeln!(
                    f,
                    "{con} `{mount}` \\({fs}, {kind:?}\\): \
                     `{used_pct:.1}%` used \
                     \\(`{used}` / `{total}`\\)",
                    con = connector,
                    mount = escape_mdv2(&disk.mount_point),
                    fs = escape_mdv2(&disk.file_system),
                    kind = disk.kind,
                    used_pct = disk.used_percent,
                    used = escape_mdv2(&format_bytes(disk.used_bytes)),
                    total = escape_mdv2(&format_bytes(disk.total_bytes)),
                )?;
            }
        }

        // ── Networks ────────────────────────────────────────────────────────
        if !s.networks.is_empty() {
            writeln!(f, "\n🌐 *Network*")?;
            let last = s.networks.len() - 1;
            for (i, net) in s.networks.iter().enumerate() {
                let connector = if i == last { "└" } else { "├" };
                writeln!(
                    f,
                    "{con} `{iface}`: ↓ `{rx}/s` ↑ `{tx}/s`",
                    con = connector,
                    iface = escape_mdv2(&net.interface),
                    rx = escape_mdv2(&format_bytes(net.rx_bytes_per_sec)),
                    tx = escape_mdv2(&format_bytes(net.tx_bytes_per_sec)),
                )?;
            }
        }

        // ── GPUs ────────────────────────────────────────────────────────────
        if !s.gpus.is_empty() {
            writeln!(f, "\n🎮 *GPU*")?;
            let last = s.gpus.len() - 1;
            for (i, gpu) in s.gpus.iter().enumerate() {
                let connector = if i == last { "└" } else { "├" };
                writeln!(f, "{connector} *{}*", escape_mdv2(&gpu.name))?;

                if let Some(usage) = gpu.usage_percent {
                    writeln!(f, "   ├ Core: `{usage:.1}%`")?;
                }
                if let (Some(used), Some(total)) = (gpu.vram_used_bytes, gpu.vram_total_bytes) {
                    let pct_str = gpu
                        .vram_used_percent
                        .map(|p| format!(" \\(`{p:.1}%`\\)"))
                        .unwrap_or_default();
                    writeln!(
                        f,
                        "   ├ VRAM: `{used}` / `{total}`{pct_str}",
                        used = escape_mdv2(&format_bytes(used)),
                        total = escape_mdv2(&format_bytes(total)),
                    )?;
                }
                if let Some(temp) = gpu.temperature_celsius {
                    writeln!(f, "   ├ Temp: `{temp:.1}°C`")?;
                }
                if let (Some(draw), Some(limit)) = (gpu.power_draw_watts, gpu.power_limit_watts) {
                    writeln!(f, "   ├ Power: `{draw:.1}W` / `{limit:.1}W`")?;
                } else if let Some(draw) = gpu.power_draw_watts {
                    writeln!(f, "   ├ Power: `{draw:.1}W`")?;
                }
                if !gpu.fan_speed_percent.is_empty() {
                    let fans = gpu
                        .fan_speed_percent
                        .iter()
                        .map(|f| format!("`{f:.0}%`"))
                        .collect::<Vec<_>>()
                        .join(", ");
                    writeln!(
                        f,
                        "   └ Fan{}: {fans}",
                        if gpu.fan_speed_percent.len() > 1 {
                            "s"
                        } else {
                            ""
                        }
                    )?;
                }
            }
        }

        // ── Battery ─────────────────────────────────────────────────────────
        if let Some(bat) = &s.battery {
            writeln!(
                f,
                "\n🔋 *Battery* — `{charge:.1}%` \\({state:?}\\)",
                charge = bat.charge_percent,
                state = bat.state,
            )?;
            if let Some(secs) = bat.time_to_empty_secs {
                writeln!(f, "   ├ Time to empty: `{}`", format_uptime(secs))?;
            }
            if let Some(secs) = bat.time_to_full_secs {
                writeln!(f, "   ├ Time to full: `{}`", format_uptime(secs))?;
            }
            if let Some(watts) = bat.power_draw_watts {
                writeln!(f, "   ├ Draw: `{watts:.1}W`")?;
            }
            if let Some(health) = bat.health_percent {
                writeln!(f, "   ├ Health: `{health:.1}%`")?;
            }
            if let Some(cycles) = bat.cycle_count {
                writeln!(f, "   └ Cycles: `{cycles}`")?;
            }
        }

        // ── Thermals ────────────────────────────────────────────────────────
        if !s.temperatures.is_empty() {
            writeln!(f, "\n🌡️ *Thermals*")?;
            let last = s.temperatures.len() - 1;
            for (i, t) in s.temperatures.iter().enumerate() {
                let connector = if i == last { "└" } else { "├" };
                let temp_str = t
                    .temperature_celsius
                    .map(|v| format!("`{v:.1}°C`"))
                    .unwrap_or_else(|| "`n/a`".into());
                let crit_str = t
                    .temperature_critical_celsius
                    .map(|v| format!(" \\(crit `{v:.1}°C`\\)"))
                    .unwrap_or_default();
                writeln!(
                    f,
                    "{connector} `{label}`: {temp}{crit}",
                    label = escape_mdv2(&t.label),
                    temp = temp_str,
                    crit = crit_str,
                )?;
            }
        }

        // ── Health summary ──────────────────────────────────────────────────
        write!(
            f,
            "\n{emoji} *Health*: `{health:?}`",
            emoji = super::utils::health_emoji(&s.health),
            health = s.health,
        )?;

        Ok(())
    }
}

fn unit_state_emoji(state: &UnitActiveState) -> &'static str {
    match state {
        UnitActiveState::Active => "🟢",
        UnitActiveState::Reloading => "🔄",
        UnitActiveState::Inactive => "⚫",
        UnitActiveState::Failed => "🔴",
        UnitActiveState::Activating => "🟡",
        UnitActiveState::Deactivating => "🟠",
    }
}

impl<'a> std::fmt::Display for TelegramDisplay<'a, crate::systemd::UnitSnapshot> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let u = self.0;
        let emoji = unit_state_emoji(&u.active_state);

        writeln!(
            f,
            "{emoji} *{name}*  `{sub}`",
            name = escape_mdv2(&u.unit_name),
            sub = escape_mdv2(&u.sub_state),
        )?;
        if !u.description.is_empty() && u.description != u.unit_name {
            writeln!(f, "   ├ `{}`", escape_mdv2(&u.description))?;
        }
        if let Some(pid) = u.main_pid {
            writeln!(f, "   ├ PID: `{pid}`")?;
        }
        if let Some(mem) = u.memory_bytes {
            writeln!(f, "   ├ Mem: `{}`", escape_mdv2(&format_bytes(mem)))?;
        }
        if let Some(cpu_ns) = u.cpu_usage_ns {
            let cpu_secs = cpu_ns as f64 / 1_000_000_000.0;
            writeln!(f, "   ├ CPU time: `{cpu_secs:.2}s`")?;
        }
        if let Some(restarts) = u.restart_count {
            writeln!(f, "   ├ Restarts: `{restarts}`")?;
        }
        if let Some(since) = &u.since {
            writeln!(f, "   ├ Since: `{}`", escape_mdv2(since))?;
        }
        if let Some(path) = &u.fragment_path {
            write!(f, "   └ File: `{}`", escape_mdv2(path))?;
        }

        Ok(())
    }
}

impl<'a> std::fmt::Display for TelegramDisplay<'a, crate::systemd::SystemdSnapshot> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let s = self.0;

        writeln!(f, "🔧 *Systemd* — `{ts}`", ts = escape_mdv2(&s.timestamp),)?;
        writeln!(
            f,
            "├ 🟢 Active: `{active}`\n\
             ├ ⚫ Inactive: `{inactive}`\n\
             └ 🔴 Failed: `{failed}`",
            active = s.active_count,
            inactive = s.inactive_count,
            failed = s.failed_count,
        )?;
        if s.failed_count > 0 {
            let failed_units: Vec<_> = s
                .units
                .iter()
                .filter(|u| u.active_state == UnitActiveState::Failed)
                .collect();
            if !failed_units.is_empty() {
                writeln!(f, "\n🔴 *Failed Units:*")?;
                for unit in failed_units {
                    writeln!(
                        f,
                        "• `{name}` — {sub}",
                        name = escape_mdv2(&unit.unit_name),
                        sub = escape_mdv2(&unit.sub_state),
                    )?;
                }
            }
        }

        Ok(())
    }
}
