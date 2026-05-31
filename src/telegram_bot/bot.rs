use std::sync::Arc;
use teloxide::{
    prelude::*,
    types::{
        ChatId, InlineKeyboardButton, InlineKeyboardMarkup, InputFile, InputMedia, InputMediaPhoto,
        KeyboardButton, KeyboardMarkup, ParseMode, ReplyMarkup,
    },
};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use super::{
    models::{
        AuthState, ChatState, Command, ProcessCallbackAction, State, Subsystem,
        SystemResourcesCallbackAction, TelegramBot, TelegramDisplay,
    },
    utils::escape_mdv2,
};
use crate::{
    prelude::*,
    process_tracker, screen_capture,
    system_resources::{self, RefreshMask, Thresholds},
    systemd,
    utils::recv_or_pending,
};

pub fn init_bot(cancel_token: CancellationToken) -> Option<TelegramBot> {
    let config = get_config();
    if !config.args.telegram {
        return None;
    }
    let Some(token) = &config.persistent.telegram_token else {
        error!("No telegram token is provided");
        return None;
    };
    let bot = Bot::new(token);
    let (sender, receiver) = mpsc::channel(64);
    let sender = Arc::new(sender);
    let state = State::new();
    let mut dispatcher = Dispatcher::builder(bot.clone(), schema())
        .dependencies(dptree::deps![cancel_token.clone(), sender, state.clone()])
        .build();
    let shutdown_token = dispatcher.shutdown_token();
    let bot_clone = bot.clone();
    let cancel_clone = cancel_token.clone();
    tokio::spawn(async move {
        loop {
            match bot_clone.get_me().await {
                Ok(_) => {
                    info!("Telegram connection established, starting dispatcher");
                    break;
                }
                Err(err) => {
                    warn!("Telegram unreachable, retrying in 10s: {err}");
                    tokio::select! {
                        _ = tokio::time::sleep(tokio::time::Duration::from_secs(10)) => {}
                        _ = cancel_clone.cancelled() => {
                            info!("Cancelled while waiting for Telegram");
                            return;
                        }
                    }
                }
            }
        }
        dispatcher.dispatch().await;
    });
    tokio::spawn(async move {
        process_tracker_event_notifier(bot, receiver, state, cancel_token).await
    });
    info!("Telegram Bot started");
    Some(TelegramBot { shutdown_token })
}

pub async fn process_tracker_event_notifier(
    bot: Bot,
    mut chat_state_rx: mpsc::Receiver<(ChatId, AuthState)>,
    mut state: State,
    cancel_token: CancellationToken,
) {
    let mut process_tracker_rx = process_tracker::subscribe_events();
    let mut system_resources_rx = system_resources::subscribe_events();
    let mut systemd_rx = systemd::subscribe_events();
    if crate::all_none!(process_tracker_rx, system_resources_rx, systemd_rx) {
        return;
    }
    loop {
        tokio::select! {
            _ = cancel_token.cancelled() => {
                info!("Cancelled while waiting for events");
                return;
            }
            Some((chat_id, auth_state)) = chat_state_rx.recv() => {
                state.set_chat_auth(chat_id, auth_state);
                info!("Chat id registered or authenticated: {chat_id}");
            }
            event = recv_or_pending(&mut process_tracker_rx, "telegram: process tracker") => {
                let message = super::utils::format_process_tracker_event(&event);
                broadcast_message(&bot, &mut state, &message).await;
            }
            event = recv_or_pending(&mut system_resources_rx, "telegram: system resources") => {
                let message = super::utils::format_system_resources_event(&event);
                if let Some(msg) = message {
                    broadcast_message(&bot, &mut state, &msg).await;
                }
            }
            event = recv_or_pending(&mut systemd_rx, "telegram: systemd") => {
                let message = super::utils::format_systemd_event(&event);
                if let Some(msg) = message {
                    broadcast_message(&bot, &mut state, &msg).await;
                }
            }
        }
    }
}

async fn broadcast_message(bot: &Bot, state: &mut State, message: &str) {
    let mut dead = vec![];
    let chat_ids = state.get_relevant_chat_ids();
    for (i, &chat_id) in chat_ids.iter().enumerate() {
        if let Err(err) = bot
            .send_message(chat_id, message)
            .parse_mode(ParseMode::MarkdownV2)
            .await
        {
            warn!("Failed to send event to chat {chat_id}: {err}");
            dead.push(i);
        }
    }
    for i in dead.iter().rev() {
        state.remove_chat(chat_ids[*i]);
    }
}

async fn send_chat_state(tx: &mpsc::Sender<(ChatId, AuthState)>, value: (ChatId, AuthState)) {
    if tx.is_closed() {
        return;
    }
    if let Err(err) = tx.send(value).await {
        warn!("Chat state channel closed unexpectedly: {err}");
    }
}

fn main_keyboard() -> KeyboardMarkup {
    KeyboardMarkup::new([
        vec![
            KeyboardButton::new("📊 Process"),
            KeyboardButton::new("📊 Top Processes"),
        ],
        vec![
            KeyboardButton::new("🖼️ Screenshot"),
            KeyboardButton::new("🖥️ System Resources"),
        ],
        vec![
            KeyboardButton::new("🔧 Systemd"),
            KeyboardButton::new("⚙️ Settings"),
        ],
        vec![
            KeyboardButton::new("🔑 Authenticate"),
            KeyboardButton::new("📋 Help"),
        ],
        vec![KeyboardButton::new("🔴 Stop")],
    ])
    .resize_keyboard()
}

fn top_processes_keyboard() -> KeyboardMarkup {
    KeyboardMarkup::new([
        vec![
            KeyboardButton::new("🔥 By CPU"),
            KeyboardButton::new("🧠 By Memory"),
        ],
        vec![KeyboardButton::new("❌ Cancel")],
    ])
    .resize_keyboard()
}

fn systemd_keyboard() -> KeyboardMarkup {
    KeyboardMarkup::new([
        vec![
            KeyboardButton::new("📋 Systemd Overview"),
            KeyboardButton::new("🔴 Failed Units"),
        ],
        vec![KeyboardButton::new("🔍 Unit Status")],
        vec![KeyboardButton::new("❌ Cancel")],
    ])
    .resize_keyboard()
}

fn awaiting_unit_name_keyboard() -> KeyboardMarkup {
    KeyboardMarkup::new([[KeyboardButton::new("❌ Cancel")]]).resize_keyboard()
}

fn awaiting_auth_token_keyboard() -> KeyboardMarkup {
    KeyboardMarkup::new([[KeyboardButton::new("❌ Cancel")]]).resize_keyboard()
}

fn settings_keyboard() -> KeyboardMarkup {
    KeyboardMarkup::new([
        vec![KeyboardButton::new("⏱️ Polling")],
        vec![KeyboardButton::new("❌ Cancel")],
    ])
    .resize_keyboard()
}

fn polling_subsystem_keyboard() -> KeyboardMarkup {
    KeyboardMarkup::new([
        vec![
            KeyboardButton::new("⏱️ Process Tracker Polling"),
            KeyboardButton::new("⏱️ Screen Capture Polling"),
        ],
        vec![KeyboardButton::new("⏱️ System Resources Polling")],
        vec![KeyboardButton::new("❌ Cancel")],
    ])
    .resize_keyboard()
}

fn subsystem_polling_keyboard(subsystem: &Subsystem) -> KeyboardMarkup {
    let label = subsystem.label();
    KeyboardMarkup::new([
        vec![
            KeyboardButton::new(format!("⏸️ Pause {label}")),
            KeyboardButton::new(format!("▶️ Resume {label}")),
        ],
        vec![KeyboardButton::new(format!("🕐 Set {label} Interval"))],
        vec![KeyboardButton::new("❌ Cancel")],
    ])
    .resize_keyboard()
}

/// Inline keyboard attached to a tracked-process message (shows Untrack + Kill Tree + signals).
fn tracked_process_keyboard(root_pid: u32) -> InlineKeyboardMarkup {
    let mut rows = vec![vec![
        InlineKeyboardButton::callback(
            "➖ Untrack",
            ProcessCallbackAction::Untrack { pid: root_pid }.encode(),
        ),
        InlineKeyboardButton::callback(
            "🌲 Kill Tree",
            ProcessCallbackAction::KillTree { pid: root_pid }.encode(),
        ),
    ]];

    let signal_row = process_tracker::ProcessSignal::get_supported_signals()
        .into_iter()
        .map(|signal| {
            let label = signal.to_string().to_uppercase();
            let data = ProcessCallbackAction::Signal {
                pid: root_pid,
                signal,
            }
            .encode();
            InlineKeyboardButton::callback(label, data)
        })
        .collect::<Vec<_>>();

    rows.push(signal_row);
    InlineKeyboardMarkup::new(rows)
}

/// Inline keyboard attached to a top-process message (shows Track).
fn top_process_keyboard(pid: u32) -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new([[InlineKeyboardButton::callback(
        "➕ Track",
        ProcessCallbackAction::Track { pid }.encode(),
    )]])
}

fn schema() -> teloxide::dispatching::UpdateHandler<Error> {
    let command_handler = teloxide::filter_command::<Command, _>()
        .branch(dptree::case![Command::Start].endpoint(handle_start))
        .branch(dptree::case![Command::Menu].endpoint(handle_start))
        .branch(dptree::case![Command::Help].endpoint(handle_help))
        .branch(dptree::case![Command::Auth].endpoint(handle_auth_prompt))
        .branch(dptree::case![Command::Screenshot].endpoint(handle_screenshot))
        .branch(dptree::case![Command::SystemSnapshot].endpoint(handle_system_resources))
        .branch(dptree::case![Command::Process].endpoint(handle_process))
        .branch(dptree::case![Command::TopProcesses].endpoint(handle_top_processes_menu))
        .branch(dptree::case![Command::StopKnightWatch].endpoint(handle_stop));

    dptree::entry()
        .branch(
            Update::filter_message()
                .branch(command_handler)
                .branch(dptree::endpoint(handle_plain_message)),
        )
        .branch(Update::filter_callback_query().endpoint(handle_callback_query))
}

async fn handle_start(
    bot: Bot,
    msg: Message,
    chat_state_tx: Arc<mpsc::Sender<(ChatId, AuthState)>>,
    state: State,
) -> Result<()> {
    send_chat_state(
        &chat_state_tx,
        (msg.chat.id, state.get_chat_auth(msg.chat.id)),
    )
    .await;
    bot.send_message(
        msg.chat.id,
        "🤖 *Knight Watch BOT*\n\nChoose an action below:",
    )
    .parse_mode(ParseMode::MarkdownV2)
    .reply_markup(ReplyMarkup::Keyboard(main_keyboard()))
    .await?;
    Ok(())
}

async fn handle_help(bot: Bot, msg: Message) -> Result<()> {
    bot.send_message(
        msg.chat.id,
        "🚀 *This is a bot ran by Knight Watch:*\n\
         • Receive Screenshot of Monitors\n\
         • Get Process Info\n\
         • Get System Resources\n\
         • Monitor Systemd Units\n\
         • Stop the knight Watch",
    )
    .parse_mode(ParseMode::MarkdownV2)
    .await?;
    Ok(())
}

async fn handle_screenshot(bot: Bot, msg: Message, state: State) -> Result<()> {
    if !state.is_authorized(msg.chat.id) {
        return send_auth_first_message(bot, msg.chat.id).await;
    }
    bot.send_message(msg.chat.id, "🖼️ Taking Screenshots...")
        .await?;
    let images = screen_capture::get_screenshots().await;
    if images.is_empty() {
        bot.send_message(msg.chat.id, "🖼️ No Images were provided.")
            .await?;
        return Ok(());
    }
    for (chunk_idx, chunk) in images.chunks(10).enumerate() {
        if chunk.len() == 1 {
            let s = &chunk[0];
            bot.send_photo(msg.chat.id, InputFile::memory(s.image.clone()))
                .caption(format!(
                    "🖼️ {} | {}x{} | {}",
                    s.monitor_name, s.width, s.height, s.timestamp
                ))
                .await?;
        } else {
            let media: Vec<InputMedia> = chunk
                .iter()
                .enumerate()
                .map(|(i, s)| {
                    let mut photo = InputMediaPhoto::new(InputFile::memory(s.image.clone()));
                    if i == 0 {
                        photo = photo.caption(format!("🖼️ Screenshot — batch {}", chunk_idx + 1));
                    } else {
                        photo = photo.caption(format!(
                            "{} | {}x{} | {}",
                            s.monitor_name, s.width, s.height, s.timestamp
                        ));
                    }
                    InputMedia::Photo(photo)
                })
                .collect();
            bot.send_media_group(msg.chat.id, media).await?;
        }
    }
    Ok(())
}

async fn handle_process(bot: Bot, msg: Message, state: State) -> Result<()> {
    if !state.is_authorized(msg.chat.id) {
        return send_auth_first_message(bot, msg.chat.id).await;
    }
    let root_pids = process_tracker::get_root_pids().await;
    if root_pids.is_empty() {
        bot.send_message(msg.chat.id, "📊 No tracked processes found.")
            .reply_markup(ReplyMarkup::Keyboard(main_keyboard()))
            .await?;
        return Ok(());
    }
    for root_pid in root_pids {
        let (root, children, work_done) = tokio::join!(
            process_tracker::get_root(root_pid),
            process_tracker::get_children(root_pid),
            process_tracker::is_work_done(root_pid),
        );
        let child_count = children.len();
        let process_tree_snapshot = super::models::TelegramDisplay(&process_tracker::ProcessTree {
            root,
            children,
            child_count,
            work_done,
            timestamp: crate::utils::now_rfc3339(),
        });
        bot.send_message(msg.chat.id, process_tree_snapshot.to_string())
            .parse_mode(ParseMode::MarkdownV2)
            .reply_markup(tracked_process_keyboard(root_pid))
            .await?;
    }
    Ok(())
}

fn system_resources_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new([
        vec![
            InlineKeyboardButton::callback(
                "⚠️ Thresholds: Default",
                SystemResourcesCallbackAction::SetThresholds(Thresholds::default()).encode(),
            ),
            InlineKeyboardButton::callback(
                "⚠️ Thresholds: Strict",
                SystemResourcesCallbackAction::SetThresholds(Thresholds {
                    cpu_warn: 75.0,
                    memory_warn: 75.0,
                    disk_warn: 75.0,
                    battery_low: 25.0,
                })
                .encode(),
            ),
        ],
        vec![
            InlineKeyboardButton::callback(
                "🔄 Mask: All On",
                SystemResourcesCallbackAction::SetRefreshMask(RefreshMask::default()).encode(),
            ),
            InlineKeyboardButton::callback(
                "🔄 Mask: CPU+Mem Only",
                SystemResourcesCallbackAction::SetRefreshMask(RefreshMask {
                    cpu: true,
                    memory: true,
                    disks: false,
                    networks: false,
                    temperatures: false,
                    gpus: false,
                })
                .encode(),
            ),
        ],
    ])
}

async fn handle_system_resources(bot: Bot, msg: Message, state: State) -> Result<()> {
    if !state.is_authorized(msg.chat.id) {
        return send_auth_first_message(bot, msg.chat.id).await;
    }
    let system_snapshot = system_resources::get_snapshot().await;
    let message = match system_snapshot {
        Some(snap) => TelegramDisplay(&snap).to_string(),
        None => "*No System Snapshot found*".to_string(),
    };
    bot.send_message(msg.chat.id, message)
        .parse_mode(ParseMode::MarkdownV2)
        .reply_markup(system_resources_keyboard())
        .await?;
    Ok(())
}

async fn handle_top_processes_menu(bot: Bot, msg: Message, state: State) -> Result<()> {
    if !state.is_authorized(msg.chat.id) {
        return send_auth_first_message(bot, msg.chat.id).await;
    }
    bot.send_message(msg.chat.id, "📊 *Top Processes* — sort by:")
        .parse_mode(ParseMode::MarkdownV2)
        .reply_markup(ReplyMarkup::Keyboard(top_processes_keyboard()))
        .await?;
    Ok(())
}

async fn handle_top_processes_by(
    bot: Bot,
    msg: Message,
    by: process_tracker::SortKey,
) -> Result<()> {
    let label = by.to_string();
    bot.send_message(
        msg.chat.id,
        format!("⏳ Fetching top processes by {label}…"),
    )
    .await?;
    let snapshots = process_tracker::get_top_processes(by, 0).await;
    if snapshots.is_empty() {
        bot.send_message(msg.chat.id, "No processes found.")
            .reply_markup(ReplyMarkup::Keyboard(main_keyboard()))
            .await?;
        return Ok(());
    }
    let header = format!("📊 *Top Processes by {label}*\n\n");
    bot.send_message(msg.chat.id, header)
        .parse_mode(ParseMode::MarkdownV2)
        .reply_markup(ReplyMarkup::Keyboard(main_keyboard()))
        .await?;
    for snapshot in &snapshots {
        let body = TelegramDisplay(snapshot).to_string();
        bot.send_message(msg.chat.id, body)
            .parse_mode(ParseMode::MarkdownV2)
            .reply_markup(top_process_keyboard(snapshot.pid))
            .await?;
    }
    Ok(())
}

async fn handle_stop(bot: Bot, msg: Message, cancel_token: CancellationToken) -> Result<()> {
    bot.send_message(msg.chat.id, "🛑 Stopping Knight Watch…")
        .await?;
    cancel_token.cancel();
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Settings / polling handlers
// ─────────────────────────────────────────────────────────────────────────────

async fn handle_settings_menu(bot: Bot, msg: Message, state: State) -> Result<()> {
    if !state.is_authorized_to_commmand(msg.chat.id) {
        return send_auth_first_message(bot, msg.chat.id).await;
    }
    state.set_chat_state_idle(msg.chat.id);
    bot.send_message(msg.chat.id, "⚙️ *Settings*")
        .parse_mode(ParseMode::MarkdownV2)
        .reply_markup(ReplyMarkup::Keyboard(settings_keyboard()))
        .await?;
    Ok(())
}

async fn handle_polling_menu(bot: Bot, msg: Message, state: State) -> Result<()> {
    if !state.is_authorized_to_commmand(msg.chat.id) {
        return send_auth_first_message(bot, msg.chat.id).await;
    }
    state.set_chat_state_idle(msg.chat.id);
    bot.send_message(msg.chat.id, "⏱️ *Polling* — choose a subsystem:")
        .parse_mode(ParseMode::MarkdownV2)
        .reply_markup(ReplyMarkup::Keyboard(polling_subsystem_keyboard()))
        .await?;
    Ok(())
}

async fn handle_subsystem_polling_menu(
    bot: Bot,
    msg: Message,
    state: State,
    subsystem: Subsystem,
) -> Result<()> {
    if !state.is_authorized_to_commmand(msg.chat.id) {
        return send_auth_first_message(bot, msg.chat.id).await;
    }
    state.set_chat_state_idle(msg.chat.id);
    let label = subsystem.label();
    bot.send_message(
        msg.chat.id,
        format!("⏱️ *{label} Polling*", label = escape_mdv2(label)),
    )
    .parse_mode(ParseMode::MarkdownV2)
    .reply_markup(ReplyMarkup::Keyboard(subsystem_polling_keyboard(
        &subsystem,
    )))
    .await?;
    Ok(())
}

async fn handle_pause_polling(
    bot: Bot,
    msg: Message,
    state: State,
    subsystem: Subsystem,
) -> Result<()> {
    if !state.is_authorized_to_commmand(msg.chat.id) {
        return send_auth_first_message(bot, msg.chat.id).await;
    }
    let reply = match &subsystem {
        Subsystem::ProcessTracker => match process_tracker::pause_poll().await {
            Ok(()) => format!("⏸️ {} polling paused\\.", escape_mdv2(subsystem.label())),
            Err(e) => format!("❌ Failed: `{}`", escape_mdv2(&e.to_string())),
        },
        Subsystem::ScreenCapture => match screen_capture::pause_poll().await {
            Ok(()) => format!("⏸️ {} polling paused\\.", escape_mdv2(subsystem.label())),
            Err(e) => format!("❌ Failed: `{}`", escape_mdv2(&e.to_string())),
        },
        Subsystem::SystemResources => match system_resources::pause_poll().await {
            Ok(()) => format!("⏸️ {} polling paused\\.", escape_mdv2(subsystem.label())),
            Err(e) => format!("❌ Failed: `{}`", escape_mdv2(&e.to_string())),
        },
    };
    bot.send_message(msg.chat.id, reply)
        .parse_mode(ParseMode::MarkdownV2)
        .reply_markup(ReplyMarkup::Keyboard(subsystem_polling_keyboard(
            &subsystem,
        )))
        .await?;
    Ok(())
}

async fn handle_resume_polling(
    bot: Bot,
    msg: Message,
    state: State,
    subsystem: Subsystem,
) -> Result<()> {
    if !state.is_authorized_to_commmand(msg.chat.id) {
        return send_auth_first_message(bot, msg.chat.id).await;
    }
    let reply = match &subsystem {
        Subsystem::ProcessTracker => match process_tracker::resume_poll().await {
            Ok(()) => format!("▶️ {} polling resumed\\.", escape_mdv2(subsystem.label())),
            Err(e) => format!("❌ Failed: `{}`", escape_mdv2(&e.to_string())),
        },
        Subsystem::ScreenCapture => match screen_capture::resume_poll().await {
            Ok(()) => format!("▶️ {} polling resumed\\.", escape_mdv2(subsystem.label())),
            Err(e) => format!("❌ Failed: `{}`", escape_mdv2(&e.to_string())),
        },
        Subsystem::SystemResources => match system_resources::resume_poll().await {
            Ok(()) => format!("▶️ {} polling resumed\\.", escape_mdv2(subsystem.label())),
            Err(e) => format!("❌ Failed: `{}`", escape_mdv2(&e.to_string())),
        },
    };
    bot.send_message(msg.chat.id, reply)
        .parse_mode(ParseMode::MarkdownV2)
        .reply_markup(ReplyMarkup::Keyboard(subsystem_polling_keyboard(
            &subsystem,
        )))
        .await?;
    Ok(())
}

async fn handle_poll_interval_prompt(
    bot: Bot,
    msg: Message,
    state: State,
    subsystem: Subsystem,
) -> Result<()> {
    if !state.is_authorized_to_commmand(msg.chat.id) {
        return send_auth_first_message(bot, msg.chat.id).await;
    }
    let label = subsystem.label();
    state.set_chat_state(msg.chat.id, ChatState::AwaitingPollInterval { subsystem });
    bot.send_message(
        msg.chat.id,
        format!(
            "🕐 Enter the new poll interval for *{}* in seconds \\(e\\.g\\. `5`\\):",
            escape_mdv2(label)
        ),
    )
    .parse_mode(ParseMode::MarkdownV2)
    .reply_markup(ReplyMarkup::Keyboard(
        KeyboardMarkup::new([[KeyboardButton::new("❌ Cancel")]]).resize_keyboard(),
    ))
    .await?;
    Ok(())
}

async fn handle_poll_interval_input(
    bot: Bot,
    msg: Message,
    state: State,
    subsystem: Subsystem,
    input: String,
) -> Result<()> {
    state.set_chat_state_idle(msg.chat.id);
    let reply = match input.trim().parse::<u64>() {
        Ok(secs) if secs > 0 => {
            let interval = std::time::Duration::from_secs(secs);
            let label = escape_mdv2(subsystem.label());
            let result = match &subsystem {
                Subsystem::ProcessTracker => process_tracker::set_poll_interval(interval).await,
                Subsystem::ScreenCapture => screen_capture::set_poll_interval(interval).await,
                Subsystem::SystemResources => system_resources::set_poll_interval(interval).await,
            };
            match result {
                Ok(()) => format!("✅ *{label}* poll interval set to `{secs}s`\\."),
                Err(e) => format!("❌ Failed: `{}`", escape_mdv2(&e.to_string())),
            }
        }
        _ => "⚠️ Invalid input\\. Please enter a positive integer number of seconds\\.".to_string(),
    };
    bot.send_message(msg.chat.id, reply)
        .parse_mode(ParseMode::MarkdownV2)
        .reply_markup(ReplyMarkup::Keyboard(subsystem_polling_keyboard(
            &subsystem,
        )))
        .await?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Inline button callback handler
// ─────────────────────────────────────────────────────────────────────────────

async fn handle_callback_query(bot: Bot, q: CallbackQuery, state: State) -> Result<()> {
    let _ = bot.answer_callback_query(q.id.clone()).await;
    let chat_id = match q.message.as_ref().map(|m| m.chat().id) {
        Some(id) => id,
        None => return Ok(()),
    };
    if !state.is_authorized_to_commmand(chat_id) {
        bot.send_message(chat_id, "🔒 Please authenticate first\\.")
            .parse_mode(ParseMode::MarkdownV2)
            .await?;
        return Ok(());
    }
    let data = match &q.data {
        Some(d) => d,
        None => return Ok(()),
    };
    if let Some(action) = ProcessCallbackAction::decode(data) {
        return handle_process_callback(bot, q, chat_id, action).await;
    }
    if let Some(action) = SystemResourcesCallbackAction::decode(data) {
        return handle_system_resources_callback(bot, chat_id, action).await;
    }
    bot.send_message(chat_id, "❓ Unknown action\\.").await?;
    Ok(())
}

async fn handle_process_callback(
    bot: Bot,
    q: CallbackQuery,
    chat_id: ChatId,
    action: ProcessCallbackAction,
) -> Result<()> {
    match action {
        ProcessCallbackAction::Track { pid } => {
            let reply = match process_tracker::track_pid(pid).await {
                Ok(()) => format!("✅ Now tracking PID `{pid}`\\."),
                Err(e) => format!("❌ Failed: `{}`", escape_mdv2(&e.to_string())),
            };
            if let Some(msg) = &q.message {
                let _ = bot
                    .edit_message_reply_markup(chat_id, msg.id())
                    .reply_markup(InlineKeyboardMarkup::new([[
                        InlineKeyboardButton::callback("✅ Tracking", "noop"),
                    ]]))
                    .await;
            }
            bot.send_message(chat_id, reply)
                .parse_mode(ParseMode::MarkdownV2)
                .await?;
        }
        ProcessCallbackAction::Untrack { pid } => {
            let reply = match process_tracker::untrack_pid(pid).await {
                Ok(()) => format!("✅ Untracked PID `{pid}`\\."),
                Err(e) => format!("❌ Failed: `{}`", escape_mdv2(&e.to_string())),
            };
            if let Some(msg) = &q.message {
                let _ = bot
                    .edit_message_reply_markup(chat_id, msg.id())
                    .reply_markup(InlineKeyboardMarkup::new([[
                        InlineKeyboardButton::callback("🚫 Untracked", "noop"),
                    ]]))
                    .await;
            }
            bot.send_message(chat_id, reply)
                .parse_mode(ParseMode::MarkdownV2)
                .await?;
        }
        ProcessCallbackAction::KillTree { pid } => {
            let reply = match process_tracker::kill_tree(pid).await {
                Ok(killed) => {
                    let pids = killed
                        .iter()
                        .map(|p| format!("`{p}`"))
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!("🌲 Kill tree sent to PID `{pid}`\\.\nSignalled: {pids}")
                }
                Err(e) => format!("❌ Failed: `{}`", escape_mdv2(&e.to_string())),
            };
            bot.send_message(chat_id, reply)
                .parse_mode(ParseMode::MarkdownV2)
                .await?;
        }
        ProcessCallbackAction::Signal { pid, signal } => {
            let reply = match process_tracker::kill_process(pid, signal).await {
                Ok(true) => format!("✅ Signal sent to PID `{pid}`\\."),
                Ok(false) => format!("⚠️ OS rejected signal for PID `{pid}`\\."),
                Err(e) => format!("❌ Failed: `{}`", escape_mdv2(&e.to_string())),
            };
            bot.send_message(chat_id, reply)
                .parse_mode(ParseMode::MarkdownV2)
                .await?;
        }
    }
    Ok(())
}

async fn handle_system_resources_callback(
    bot: Bot,
    chat_id: ChatId,
    action: SystemResourcesCallbackAction,
) -> Result<()> {
    match action {
        SystemResourcesCallbackAction::SetThresholds(thresholds) => {
            let reply = match system_resources::set_thresholds(thresholds).await {
                Ok(()) => "✅ Alert thresholds updated\\.".to_string(),
                Err(e) => format!("❌ Failed: `{}`", escape_mdv2(&e.to_string())),
            };
            bot.send_message(chat_id, reply)
                .parse_mode(ParseMode::MarkdownV2)
                .await?;
        }
        SystemResourcesCallbackAction::SetRefreshMask(mask) => {
            let reply = match system_resources::set_refresh_mask(mask).await {
                Ok(()) => "✅ Refresh mask updated\\.".to_string(),
                Err(e) => format!("❌ Failed: `{}`", escape_mdv2(&e.to_string())),
            };
            bot.send_message(chat_id, reply)
                .parse_mode(ParseMode::MarkdownV2)
                .await?;
        }
    }
    Ok(())
}

async fn handle_systemd_menu(bot: Bot, msg: Message, state: State) -> Result<()> {
    state.set_chat_state_idle(msg.chat.id);
    bot.send_message(msg.chat.id, "🔧 *Systemd* — choose an action:")
        .parse_mode(ParseMode::MarkdownV2)
        .reply_markup(ReplyMarkup::Keyboard(systemd_keyboard()))
        .await?;
    Ok(())
}

async fn handle_systemd_overview(bot: Bot, msg: Message) -> Result<()> {
    let snapshot = systemd::get_snapshot().await;
    let message = match snapshot {
        Some(snap) => TelegramDisplay(&snap).to_string(),
        None => escape_mdv2("⚠️ No systemd snapshot available."),
    };
    bot.send_message(msg.chat.id, message)
        .parse_mode(ParseMode::MarkdownV2)
        .reply_markup(ReplyMarkup::Keyboard(systemd_keyboard()))
        .await?;
    Ok(())
}

async fn handle_systemd_failed(bot: Bot, msg: Message) -> Result<()> {
    let units = systemd::get_failed_units().await;
    let message = if units.is_empty() {
        "✅ *No failed units*".to_string()
    } else {
        let body = units
            .iter()
            .map(|u| TelegramDisplay(u).to_string())
            .collect::<Vec<_>>()
            .join("\n\n");
        format!("🔴 *Failed Units* \\({}\\)\n\n{body}", units.len())
    };
    bot.send_message(msg.chat.id, message)
        .parse_mode(ParseMode::MarkdownV2)
        .reply_markup(ReplyMarkup::Keyboard(systemd_keyboard()))
        .await?;
    Ok(())
}

async fn handle_systemd_unit_prompt(bot: Bot, msg: Message, state: State) -> Result<()> {
    state.set_chat_state(msg.chat.id, ChatState::AwaitingUnitName);
    bot.send_message(msg.chat.id, "🔍 Type the unit name (e.g. nginx.service):")
        .reply_markup(ReplyMarkup::Keyboard(awaiting_unit_name_keyboard()))
        .await?;
    Ok(())
}

async fn handle_systemd_unit_lookup(
    bot: Bot,
    msg: Message,
    state: State,
    unit_name: String,
) -> Result<()> {
    state.set_chat_state_idle(msg.chat.id);
    let unit = systemd::get_unit(unit_name.clone()).await;
    let message = match unit {
        Some(u) => TelegramDisplay(&u).to_string(),
        None => format!("❓ Unit `{}` not found\\.", escape_mdv2(&unit_name)),
    };
    bot.send_message(msg.chat.id, message)
        .parse_mode(ParseMode::MarkdownV2)
        .reply_markup(ReplyMarkup::Keyboard(systemd_keyboard()))
        .await?;
    Ok(())
}

async fn handle_auth_prompt(bot: Bot, msg: Message, state: State) -> Result<()> {
    state.set_chat_state(msg.chat.id, ChatState::AwaitingAuthToken);
    bot.send_message(
        msg.chat.id,
        "🔑 Please enter your authentication token \\(6–8 digits\\):",
    )
    .parse_mode(ParseMode::MarkdownV2)
    .reply_markup(ReplyMarkup::Keyboard(awaiting_auth_token_keyboard()))
    .await?;
    Ok(())
}

async fn handle_auth_token(
    bot: Bot,
    msg: Message,
    chat_state_tx: Arc<mpsc::Sender<(ChatId, AuthState)>>,
    state: State,
    token_input: String,
) -> Result<()> {
    let is_valid_format = token_input.len() >= 6
        && token_input.len() <= 8
        && token_input.chars().all(|c| c.is_ascii_digit());
    if !is_valid_format {
        bot.send_message(
            msg.chat.id,
            "⚠️ Invalid token\\. Please enter a numeric token between 6 and 8 digits:",
        )
        .parse_mode(ParseMode::MarkdownV2)
        .reply_markup(ReplyMarkup::Keyboard(awaiting_auth_token_keyboard()))
        .await?;
        return Ok(());
    }
    if crate::config::set_user_chat_id(token_input, msg.chat.id.0)? {
        state.set_chat_state_idle(msg.chat.id);
        state.set_chat_auth(msg.chat.id, AuthState::Authenticated);
        state.set_chat_state_idle(msg.chat.id);
        send_chat_state(&chat_state_tx, (msg.chat.id, AuthState::Authenticated)).await;
        bot.send_message(msg.chat.id, "✅ You are now *authenticated*\\!")
            .parse_mode(ParseMode::MarkdownV2)
            .reply_markup(ReplyMarkup::Keyboard(main_keyboard()))
            .await?;
    } else {
        state.set_chat_state_idle(msg.chat.id);
        bot.send_message(msg.chat.id, "❌ Authentication failed\\. Invalid token\\.")
            .parse_mode(ParseMode::MarkdownV2)
            .reply_markup(ReplyMarkup::Keyboard(main_keyboard()))
            .await?;
    }
    Ok(())
}

async fn send_auth_first_message(bot: Bot, chat_id: ChatId) -> Result<()> {
    bot.send_message(
        chat_id,
        "🔒 Please authenticate first\\. Use /auth or the 🔑 Authenticate button\\.",
    )
    .parse_mode(ParseMode::MarkdownV2)
    .await?;
    Ok(())
}

async fn handle_plain_message(
    bot: Bot,
    msg: Message,
    chat_state_tx: Arc<mpsc::Sender<(ChatId, AuthState)>>,
    cancel_token: CancellationToken,
    state: State,
) -> Result<()> {
    let msg_clone = msg.clone();
    let text = match msg_clone.text() {
        Some(t) => t,
        None => return Ok(()),
    };
    let chat_state = state.get_chat_state(msg.chat.id);
    if chat_state == ChatState::AwaitingUnitName && text != "❌ Cancel" {
        return handle_systemd_unit_lookup(bot, msg, state, text.to_string()).await;
    }
    if chat_state == ChatState::AwaitingAuthToken && text != "❌ Cancel" {
        return handle_auth_token(bot, msg, chat_state_tx, state, text.to_string()).await;
    }
    if let ChatState::AwaitingPollInterval { subsystem } = chat_state
        && text != "❌ Cancel"
    {
        return handle_poll_interval_input(bot, msg, state, subsystem, text.to_string()).await;
    }

    match text {
        "📋 Help" => handle_help(bot, msg).await?,
        "🖼️ Screenshot" => handle_screenshot(bot, msg, state).await?,
        "🖥️ System Resources" => handle_system_resources(bot, msg, state).await?,
        "📊 Process" => handle_process(bot, msg, state).await?,
        "📊 Top Processes" => handle_top_processes_menu(bot, msg, state).await?,
        "🔥 By CPU" => handle_top_processes_by(bot, msg, process_tracker::SortKey::Cpu).await?,
        "🧠 By Memory" => {
            handle_top_processes_by(bot, msg, process_tracker::SortKey::Memory).await?
        }
        "🔧 Systemd" => handle_systemd_menu(bot, msg, state).await?,
        "📋 Systemd Overview" => handle_systemd_overview(bot, msg).await?,
        "🔴 Failed Units" => handle_systemd_failed(bot, msg).await?,
        "🔍 Unit Status" => handle_systemd_unit_prompt(bot, msg, state).await?,
        "⚙️ Settings" => handle_settings_menu(bot, msg, state).await?,
        "⏱️ Polling" => handle_polling_menu(bot, msg, state).await?,
        // Subsystem polling pickers
        "⏱️ Process Tracker Polling" => {
            handle_subsystem_polling_menu(bot, msg, state, Subsystem::ProcessTracker).await?
        }
        "⏱️ Screen Capture Polling" => {
            handle_subsystem_polling_menu(bot, msg, state, Subsystem::ScreenCapture).await?
        }
        "⏱️ System Resources Polling" => {
            handle_subsystem_polling_menu(bot, msg, state, Subsystem::SystemResources).await?
        }
        // Per-subsystem pause
        "⏸️ Pause Process Tracker" => {
            handle_pause_polling(bot, msg, state, Subsystem::ProcessTracker).await?
        }
        "⏸️ Pause Screen Capture" => {
            handle_pause_polling(bot, msg, state, Subsystem::ScreenCapture).await?
        }
        "⏸️ Pause System Resources" => {
            handle_pause_polling(bot, msg, state, Subsystem::SystemResources).await?
        }
        // Per-subsystem resume
        "▶️ Resume Process Tracker" => {
            handle_resume_polling(bot, msg, state, Subsystem::ProcessTracker).await?
        }
        "▶️ Resume Screen Capture" => {
            handle_resume_polling(bot, msg, state, Subsystem::ScreenCapture).await?
        }
        "▶️ Resume System Resources" => {
            handle_resume_polling(bot, msg, state, Subsystem::SystemResources).await?
        }
        // Per-subsystem set interval
        "🕐 Set Process Tracker Interval" => {
            handle_poll_interval_prompt(bot, msg, state, Subsystem::ProcessTracker).await?
        }
        "🕐 Set Screen Capture Interval" => {
            handle_poll_interval_prompt(bot, msg, state, Subsystem::ScreenCapture).await?
        }
        "🕐 Set System Resources Interval" => {
            handle_poll_interval_prompt(bot, msg, state, Subsystem::SystemResources).await?
        }
        "🔑 Authenticate" => handle_auth_prompt(bot, msg, state).await?,
        "❌ Cancel" => {
            state.set_chat_state_idle(msg.chat.id);
            bot.send_message(msg.chat.id, "Cancelled")
                .reply_markup(ReplyMarkup::Keyboard(main_keyboard()))
                .await?;
        }
        "🔴 Stop" => handle_stop(bot, msg, cancel_token).await?,
        text => {
            bot.send_message(
                msg.chat.id,
                escape_mdv2(&format!(
                    "You said: \"{text}\"\n\nUse the buttons below or type /start."
                )),
            )
            .await?;
        }
    }
    Ok(())
}
