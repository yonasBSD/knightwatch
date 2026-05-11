use std::sync::OnceLock;

pub static INDEX_HTML: &str = include_str!("assets/index.html");
pub static MAIN_CSS: &str = include_str!("assets/main.css");
pub static MAIN_JS: &str = include_str!("assets/main.js");

pub static START_TIME: OnceLock<std::time::Instant> = OnceLock::new();
