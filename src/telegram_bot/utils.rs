use super::models::TelegramDisplay;
use crate::process_tracker::{enums::ProcessTrackerEvent, structs::ProcessInfo};

const SPECIAL: &[char] = &[
    '_', '*', '[', ']', '(', ')', '~', '`', '>', '#', '+', '-', '=', '|', '{', '}', '.', '!', '\\',
];

pub fn format_event(event: &ProcessTrackerEvent) -> String {
    match event {
        ProcessTrackerEvent::InitialSnapshot { root, children } => {
            let root_info = TelegramDisplay(&ProcessInfo::from(root));
            let mut msg = format!("🟢 *Initial Snapshot*\n\n*Root:*\n{root_info}");
            if children.is_empty() {
                msg.push_str("\n\n*Children:* _none_");
            } else {
                msg.push_str(&format!("\n\n*Children* ({}):", children.len()));
                for child in children {
                    let child_info = TelegramDisplay(&ProcessInfo::from(child));
                    msg.push_str(&format!("\n{child_info}\n"));
                }
            }
            msg
        }
        ProcessTrackerEvent::ChildrenAppeared { pid, children } => {
            let mut msg = format!(
                "🆕 *New Children Appeared* ({})\n*Root:* `{pid}`",
                children.len()
            );
            for child in children {
                let info = TelegramDisplay(&ProcessInfo::from(child));
                msg.push_str(&format!("\n{info}\n"));
            }
            msg
        }
        ProcessTrackerEvent::ChildrenExited { pid, children } => {
            let pid_list = children
                .iter()
                .map(|p| format!("`{p}`"))
                .collect::<Vec<_>>()
                .join(", ");
            format!("🔴 *Children Exited*\n*Root:* `{pid}`\nchildren PIDs: {pid_list}")
        }
        ProcessTrackerEvent::AllChildrenGone { pid } => {
            format!(
                "✅ *All children have exited*\n*Root:* `{pid}`\nThe root process may still be running\\."
            )
        }
        ProcessTrackerEvent::RootExited { pid } => {
            format!("💀 *Root Process Exited*\nPID: `{pid}`")
        }
        ProcessTrackerEvent::WorkComplete { pid } => {
            format!("✅ *Work Complete*\nPID: `{pid}`")
        }
    }
}

pub fn escape_mdv2(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if SPECIAL.contains(&c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}
