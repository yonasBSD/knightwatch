use std::sync::Arc;
use teloxide::{
    prelude::*,
    types::{
        ChatId, InputFile, InputMedia, InputMediaPhoto, KeyboardButton, KeyboardMarkup, ParseMode,
        ReplyMarkup,
    },
};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use super::{
    models::{AuthState, ChatState, Command, State, TelegramBot, TelegramDisplay},
    utils::escape_mdv2,
};
use crate::{prelude::*, process_tracker, system_resources, systemd, utils::recv_or_pending};

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
            KeyboardButton::new("📋 Help"),
        ],
        vec![KeyboardButton::new("🔑 Authenticate")],
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

    Update::filter_message()
        .branch(command_handler)
        .branch(dptree::endpoint(handle_plain_message))
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
    let images = crate::screen_capture::get_screenshots().await;
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
    for root_pid in process_tracker::get_root_pids().await {
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
            .await?;
    }
    Ok(())
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
    let body = snapshots
        .iter()
        .map(|s| TelegramDisplay(s).to_string())
        .collect::<Vec<_>>()
        .join("\n\n");
    let header = format!("📊 Top Processes by {label}\n\n");
    bot.send_message(msg.chat.id, format!("{header}{body}"))
        .parse_mode(ParseMode::MarkdownV2)
        .reply_markup(ReplyMarkup::Keyboard(main_keyboard()))
        .await?;
    Ok(())
}

async fn handle_stop(bot: Bot, msg: Message, cancel_token: CancellationToken) -> Result<()> {
    bot.send_message(msg.chat.id, "🛑 Stopping Knight Watch…")
        .await?;
    cancel_token.cancel();
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
