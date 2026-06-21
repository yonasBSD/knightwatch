mod dispatcher;
mod handler;

use std::sync::OnceLock;
use tokio::sync::broadcast;

use crate::{prelude::*, events::EventPayload};
pub use handler::sse_stream;

const CHANNEL_CAPACITY: usize = 256;

static TX: OnceLock<broadcast::Sender<EventPayload>> = OnceLock::new();

fn tx() -> &'static broadcast::Sender<EventPayload> {
    TX.get_or_init(|| broadcast::channel(CHANNEL_CAPACITY).0)
}

fn subscribe() -> broadcast::Receiver<EventPayload> {
    tx().subscribe()
}

fn publish(payload: EventPayload) {
    let _ = tx().send(payload);
}

pub fn init_sse_dispatcher(cancel_token: tokio_util::sync::CancellationToken) {
    if get_config().args.no_api {
        return;
    }
    tokio::spawn(dispatcher::run_dispatcher(cancel_token));
}
