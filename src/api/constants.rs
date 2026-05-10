use std::sync::OnceLock;

pub static VIEW_HTML: &str = include_str!("assets/view.html");
pub static VIEW_CSS: &str = include_str!("assets/view.css");
pub static VIEW_JS: &str = include_str!("assets/view.js");

pub static START_TIME: OnceLock<std::time::Instant> = OnceLock::new();
