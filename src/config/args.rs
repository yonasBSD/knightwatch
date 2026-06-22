use clap::{ArgAction, Subcommand};

/// Raw CLI arguments. All flags are `Option<T>` so we can detect
/// "user explicitly passed this" vs "not passed, fall back to stored defaults".
#[derive(clap::Parser, Debug)]
#[command(
    name = "knightwatch",
    about = "Screen monitoring and notification tool",
    version
)]
pub struct CliArgs {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Host address for the API server
    #[arg(long)]
    pub host: Option<String>,

    /// Port for the API server
    #[arg(long, short)]
    pub port: Option<u16>,

    /// Enable authentication
    #[arg(long, num_args(0..=1), default_missing_value = "true")]
    pub enable_auth: Option<bool>,

    /// Disable the API server entirely
    #[arg(long, num_args(0..=1), default_missing_value = "true")]
    pub no_api: Option<bool>,

    /// Disable the web dashboard
    #[arg(long, num_args(0..=1), default_missing_value = "true")]
    pub no_dashboard: Option<bool>,

    /// Disable the Screen Capture API, which may require elevated permissions on some platforms
    #[cfg(feature = "screenshot")]
    #[arg(long, num_args(0..=1), default_missing_value = "true")]
    pub blind: Option<bool>,

    /// Process ID to track
    #[arg(long)]
    pub pid: Vec<u32>,

    /// Enable Telegram bot
    #[arg(long, num_args(0..=1), default_missing_value = "true")]
    pub telegram: Option<bool>,

    /// Webhook URLs to notify on process events (repeatable)
    #[arg(long = "webhook")]
    pub webhook_urls: Vec<String>,

    /// Enable Webhooks
    #[arg(long, num_args(0..=1), default_missing_value = "true")]
    pub with_webhook: Option<bool>,

    /// Enable top processes tracker
    #[arg(long, num_args(0..=1), default_missing_value = "true")]
    pub top_processes: Option<bool>,

    /// Limit number of top processes to track
    #[arg(long)]
    pub limit_processes: Option<usize>,

    /// Enable system resources
    #[arg(long, num_args(0..=1), default_missing_value = "true")]
    pub system_resources: Option<bool>,

    /// Enable systemd
    #[arg(long, num_args(0..=1), default_missing_value = "true")]
    pub systemd: Option<bool>,

    /// Enable docker tracker
    #[arg(long, num_args(0..=1), default_missing_value = "true")]
    pub docker: Option<bool>,

    /// Allow process commands (kill, track, etc.)
    #[arg(long, num_args(0..=1), default_missing_value = "true")]
    pub allow_process_commands: Option<bool>,

    /// Allow screen commands
    #[arg(long, num_args(0..=1), default_missing_value = "true")]
    pub allow_screen_commands: Option<bool>,

    /// Allow system_resources commands
    #[arg(long, num_args(0..=1), default_missing_value = "true")]
    pub allow_system_resources_commands: Option<bool>,

    /// Allow systemd commands
    #[arg(long, num_args(0..=1), default_missing_value = "true")]
    pub allow_systemd_commands: Option<bool>,

    /// Allow docker commands
    #[arg(long, num_args(0..=1), default_missing_value = "true")]
    pub allow_docker_commands: Option<bool>,
}

/// The fully resolved, concrete configuration after merging CLI → stored defaults → hardcoded defaults.
/// This is what the rest of the app uses; it has no `Option` anywhere.
#[derive(Debug, Clone)]
pub struct ResolvedArgs {
    pub host: String,
    pub port: u16,
    pub enable_auth: bool,
    pub no_api: bool,
    pub no_dashboard: bool,
    pub blind: bool,
    pub pid: Vec<u32>,
    pub telegram: bool,
    pub webhook_urls: Vec<String>,
    pub with_webhook: bool,
    pub top_processes: bool,
    pub limit_processes: usize,
    pub system_resources: bool,
    pub systemd: bool,
    pub docker: bool,
    pub allow_process_commands: bool,
    pub allow_screen_commands: bool,
    pub allow_system_resources_commands: bool,
    pub allow_systemd_commands: bool,
    pub allow_docker_commands: bool,
}

impl ResolvedArgs {
    pub fn is_blind(&self) -> bool {
        self.blind
    }
}

/// Hardcoded fallback defaults — identical to the old `default_value` annotations.
impl Default for ResolvedArgs {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8083,
            enable_auth: false,
            no_api: false,
            no_dashboard: false,
            blind: cfg!(not(feature = "screenshot")), // true when screenshot feature is absent
            pid: vec![],
            telegram: false,
            webhook_urls: vec![],
            with_webhook: false,
            top_processes: false,
            limit_processes: 5,
            system_resources: false,
            systemd: false,
            docker: false,
            allow_process_commands: false,
            allow_screen_commands: false,
            allow_system_resources_commands: false,
            allow_systemd_commands: false,
            allow_docker_commands: false,
        }
    }
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Manage persistent configuration
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    /// Manage users
    Users {
        #[command(subcommand)]
        action: UsersAction,
    },
}

#[derive(Subcommand, Debug)]
pub enum ConfigAction {
    /// Set a config value
    Set {
        #[command(subcommand)]
        field: ConfigField,
    },
    /// Get a config value
    Get {
        #[command(subcommand)]
        field: ConfigField,
    },
    /// Set a stored runtime default
    SetDefault {
        #[command(subcommand)]
        field: DefaultField,
    },
    /// Get a stored runtime default
    GetDefault {
        #[command(subcommand)]
        field: DefaultField,
    },
    /// Clear all stored runtime defaults
    ClearDefaults,
}

#[derive(Subcommand, Debug)]
pub enum UsersAction {
    /// Add a new user
    Add { username: String },
    /// Remove a user
    Remove { username: String },
    /// List all users
    List,
    /// Remove all users
    Clear,
    /// Show the Telegram authentication token for a user
    Token { username: String },
}

#[derive(Subcommand, Debug)]
pub enum ConfigField {
    TelegramToken {
        value: Option<String>,
        #[arg(long, default_value_t = false, conflicts_with = "value")]
        clear: bool,
    },
    WebhookUrls {
        #[arg(long)]
        add: Vec<String>,
        #[arg(long)]
        remove: Vec<String>,
        #[arg(
            long,
            default_value_t = false,
            conflicts_with = "add",
            conflicts_with = "remove"
        )]
        clear: bool,
    },
}

/// Each field that can be stored as a default.
/// Boolean fields use `ArgAction::Set` so the user passes an explicit `true`/`false`
/// rather than a presence flag (which clap would interpret as `SetTrue`).
#[derive(Subcommand, Debug)]
pub enum DefaultField {
    Host {
        value: String,
    },
    Port {
        value: u16,
    },
    EnableAuth {
        #[arg(action = ArgAction::Set)]
        value: bool,
    },
    NoApi {
        #[arg(action = ArgAction::Set)]
        value: bool,
    },
    NoDashboard {
        #[arg(action = ArgAction::Set)]
        value: bool,
    },
    #[cfg(feature = "screenshot")]
    Blind {
        #[arg(action = ArgAction::Set)]
        value: bool,
    },
    Telegram {
        #[arg(action = ArgAction::Set)]
        value: bool,
    },
    WithWebhook {
        #[arg(action = ArgAction::Set)]
        value: bool,
    },
    TopProcesses {
        #[arg(action = ArgAction::Set)]
        value: bool,
    },
    LimitProcesses {
        value: usize,
    },
    SystemResources {
        #[arg(action = ArgAction::Set)]
        value: bool,
    },
    Systemd {
        #[arg(action = ArgAction::Set)]
        value: bool,
    },
    Docker {
        #[arg(action = ArgAction::Set)]
        value: bool,
    },
    AllowProcessCommands {
        #[arg(action = ArgAction::Set)]
        value: bool,
    },
    AllowScreenCommands {
        #[arg(action = ArgAction::Set)]
        value: bool,
    },
    AllowSystemResourcesCommands {
        #[arg(action = ArgAction::Set)]
        value: bool,
    },
    AllowSystemdCommands {
        #[arg(action = ArgAction::Set)]
        value: bool,
    },
    AllowDockerCommands {
        #[arg(action = ArgAction::Set)]
        value: bool,
    },
}
