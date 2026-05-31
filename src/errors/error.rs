use std::io::Error as IoError;
use teloxide::errors::RequestError;

#[derive(Debug)]
pub enum Error {
    Network(String),
    ChannelClosed(String),
    #[cfg(feature = "screenshot")]
    Screen(String),
    Config(String),
    ProcessTracker(String),
    SystemResources(String),
    #[cfg(target_os = "linux")]
    Systemd(String),
    Other(String),
    TelegramBot(String),
}

impl Error {
    pub fn bind_address(address: &str, err: IoError) -> Self {
        Self::Network(format!("Failed to bind address: {address}, {err}"))
    }
    pub fn channel_closed(err: tokio::sync::oneshot::error::RecvError) -> Self {
        Self::ChannelClosed(format!("Channel was closed: {err}"))
    }
    pub fn unsupported_signal(signal: crate::process_tracker::ProcessSignal) -> Self {
        Self::ProcessTracker(format!(
            "Signal '{signal:?}' is not supported on this platform"
        ))
    }
    pub fn process_commands_disabled() -> Self {
        Self::ProcessTracker(
            "Process commands are disabled — rerun with --allow-process-commands".into(),
        )
    }
    pub fn screen_commands_disabled() -> Self {
        Self::Screen(
            "Screen commands are disabled — rerun with --allow-screen-commands".into(),
        )
    }
    pub fn system_resources_commands_disabled() -> Self {
        Self::SystemResources(
            "System resources commands are disabled — rerun with --allow-system-resources-commands".into(),
        )
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::Other(msg)
            | Error::Config(msg)
            | Error::Network(msg)
            | Error::ChannelClosed(msg)
            | Error::TelegramBot(msg)
            | Error::SystemResources(msg)
            | Error::ProcessTracker(msg) => {
                write!(f, "{msg}")
            }
            #[cfg(target_os = "linux")]
            Error::Systemd(msg) => {
                write!(f, "{msg}")
            }
            #[cfg(feature = "screenshot")]
            Error::Screen(msg) => {
                write!(f, "{msg}")
            }
        }
    }
}

impl From<RequestError> for Error {
    fn from(err: RequestError) -> Self {
        Error::TelegramBot(err.to_string())
    }
}
