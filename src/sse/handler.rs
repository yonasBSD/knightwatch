use axum::response::sse::{Event, Sse};
use tokio_stream::StreamExt;

use crate::prelude::*;

pub async fn sse_stream()
-> Sse<impl futures::stream::Stream<Item = Result<Event, std::convert::Infallible>>> {
    let stream =
        tokio_stream::wrappers::BroadcastStream::new(super::subscribe()).filter_map(|result| {
            match result {
                Ok(payload) => {
                    let event_name = payload.event;
                    match serde_json::to_string(&payload) {
                        Ok(json) => Some(Ok(Event::default().event(event_name).data(json))),
                        Err(e) => {
                            warn!("sse: serialize error: {e}");
                            None
                        }
                    }
                }
                Err(tokio_stream::wrappers::errors::BroadcastStreamRecvError::Lagged(n)) => {
                    warn!("sse: client lagged, dropped {n} events");
                    None
                }
            }
        });
    Sse::new(stream).keep_alive(axum::response::sse::KeepAlive::default())
}
