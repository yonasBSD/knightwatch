use super::args::{CliArgs, DefaultField, ResolvedArgs};

/// Stored runtime defaults. Every field is `Option<T>`:
/// - `None`  → not stored; fall through to the hardcoded default
/// - `Some`  → use this value unless the CLI overrides it
///
/// Merge priority (highest wins):
///   CLI flag explicitly passed  >  stored ArgsConfig  >  ResolvedArgs::default()
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct ArgsConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_auth: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub no_api: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub no_dashboard: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blind: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub telegram: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub with_webhook: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_processes: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit_processes: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_resources: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub systemd: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub docker: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_process_commands: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_screen_commands: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_system_resources_commands: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_systemd_commands: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_docker_commands: Option<bool>,
}

impl super::store::JsonStore for ArgsConfig {
    const NAME: &'static str = "args";
    fn path() -> std::path::PathBuf {
        super::paths::conig_file_path("args.json")
    }
}

impl ArgsConfig {
    /// Merge CLI → stored → hardcoded defaults and produce a `ResolvedArgs`.
    ///
    /// For `Vec` fields (`pid`, `webhook_urls`): CLI values are always used as-is
    /// (there's no "stored default" concept for lists — use `PersistentConfig` for those).
    pub fn resolve(cli: CliArgs, stored: ArgsConfig) -> ResolvedArgs {
        let defaults = ResolvedArgs::default();

        macro_rules! resolve {
            ($field:ident) => {
                cli.$field.or(stored.$field).unwrap_or(defaults.$field)
            };
        }

        ResolvedArgs {
            host: cli.host.or(stored.host).unwrap_or(defaults.host),
            port: resolve!(port),
            enable_auth: resolve!(enable_auth),
            no_api: resolve!(no_api),
            no_dashboard: resolve!(no_dashboard),
            blind: {
                #[cfg(feature = "screenshot")]
                {
                    cli.blind.or(stored.blind).unwrap_or(defaults.blind)
                }
                #[cfg(not(feature = "screenshot"))]
                {
                    true
                }
            },
            pid: cli.pid,
            telegram: resolve!(telegram),
            webhook_urls: cli.webhook_urls,
            with_webhook: resolve!(with_webhook),
            top_processes: resolve!(top_processes),
            limit_processes: resolve!(limit_processes),
            system_resources: resolve!(system_resources),
            systemd: resolve!(systemd),
            docker: resolve!(docker),
            allow_process_commands: resolve!(allow_process_commands),
            allow_screen_commands: resolve!(allow_screen_commands),
            allow_system_resources_commands: resolve!(allow_system_resources_commands),
            allow_systemd_commands: resolve!(allow_systemd_commands),
            allow_docker_commands: resolve!(allow_docker_commands),
        }
    }

    /// Apply a `DefaultField` mutation and return the updated config (caller must `.save()`).
    pub fn apply_set(&mut self, field: &DefaultField) {
        match field {
            DefaultField::Host { value } => self.host = Some(value.clone()),
            DefaultField::Port { value } => self.port = Some(*value),
            DefaultField::EnableAuth { value } => self.enable_auth = Some(*value),
            DefaultField::NoApi { value } => self.no_api = Some(*value),
            DefaultField::NoDashboard { value } => self.no_dashboard = Some(*value),
            #[cfg(feature = "screenshot")]
            DefaultField::Blind { value } => self.blind = Some(*value),
            DefaultField::Telegram { value } => self.telegram = Some(*value),
            DefaultField::WithWebhook { value } => self.with_webhook = Some(*value),
            DefaultField::TopProcesses { value } => self.top_processes = Some(*value),
            DefaultField::LimitProcesses { value } => self.limit_processes = Some(*value),
            DefaultField::SystemResources { value } => self.system_resources = Some(*value),
            DefaultField::Systemd { value } => self.systemd = Some(*value),
            DefaultField::Docker { value } => self.docker = Some(*value),
            DefaultField::AllowProcessCommands { value } => {
                self.allow_process_commands = Some(*value)
            }
            DefaultField::AllowScreenCommands { value } => {
                self.allow_screen_commands = Some(*value)
            }
            DefaultField::AllowSystemResourcesCommands { value } => {
                self.allow_system_resources_commands = Some(*value)
            }
            DefaultField::AllowSystemdCommands { value } => {
                self.allow_systemd_commands = Some(*value)
            }
            DefaultField::AllowDockerCommands { value } => {
                self.allow_docker_commands = Some(*value)
            }
        }
    }

    /// Print the current stored value for a `DefaultField` (for `get-default`).
    pub fn print_field(&self, field: &DefaultField) {
        macro_rules! print_opt {
            ($label:expr, $opt:expr, $default:expr) => {
                match $opt {
                    Some(v) => println!("{} = {} (stored)", $label, v),
                    None => println!("{} = {} (default, not stored)", $label, $default),
                }
            };
        }

        let d = ResolvedArgs::default();
        match field {
            DefaultField::Host { .. } => print_opt!("host", &self.host, d.host),
            DefaultField::Port { .. } => print_opt!("port", self.port, d.port),
            DefaultField::EnableAuth { .. } => {
                print_opt!("enable_auth", self.enable_auth, d.enable_auth)
            }
            DefaultField::NoApi { .. } => print_opt!("no_api", self.no_api, d.no_api),
            DefaultField::NoDashboard { .. } => {
                print_opt!("no_dashboard", self.no_dashboard, d.no_dashboard)
            }
            #[cfg(feature = "screenshot")]
            DefaultField::Blind { .. } => print_opt!("blind", self.blind, d.blind),
            DefaultField::Telegram { .. } => print_opt!("telegram", self.telegram, d.telegram),
            DefaultField::WithWebhook { .. } => {
                print_opt!("with_webhook", self.with_webhook, d.with_webhook)
            }
            DefaultField::TopProcesses { .. } => {
                print_opt!("top_processes", self.top_processes, d.top_processes)
            }
            DefaultField::LimitProcesses { .. } => {
                print_opt!("limit_processes", self.limit_processes, d.limit_processes)
            }
            DefaultField::SystemResources { .. } => {
                print_opt!(
                    "system_resources",
                    self.system_resources,
                    d.system_resources
                )
            }
            DefaultField::Systemd { .. } => print_opt!("systemd", self.systemd, d.systemd),
            DefaultField::Docker { .. } => print_opt!("docker", self.docker, d.docker),
            DefaultField::AllowProcessCommands { .. } => print_opt!(
                "allow_process_commands",
                self.allow_process_commands,
                d.allow_process_commands
            ),
            DefaultField::AllowScreenCommands { .. } => print_opt!(
                "allow_screen_commands",
                self.allow_screen_commands,
                d.allow_screen_commands
            ),
            DefaultField::AllowSystemResourcesCommands { .. } => print_opt!(
                "allow_system_resources_commands",
                self.allow_system_resources_commands,
                d.allow_system_resources_commands
            ),
            DefaultField::AllowSystemdCommands { .. } => print_opt!(
                "allow_systemd_commands",
                self.allow_systemd_commands,
                d.allow_systemd_commands
            ),
            DefaultField::AllowDockerCommands { .. } => print_opt!(
                "allow_docker_commands",
                self.allow_docker_commands,
                d.allow_docker_commands
            ),
        }
    }
}
