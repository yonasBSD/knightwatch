mod client;
pub mod enums;
pub mod structs;
mod tracker;
mod utils;

mod process_state_serde {
    pub fn serialize<S>(state: &super::enums::ProcessState, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&state.to_string())
    }
}

pub use client::*;
pub use tracker::init_process_tracker;
