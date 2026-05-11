use std::convert::Infallible;

use axum::{
    extract::State,
    response::sse::{Event, KeepAlive, Sse},
};
use tokio_stream::{StreamExt, wrappers::BroadcastStream};

use crate::routes::AppState;

pub async fn get_inventory_events_handler(
    State(state): State<AppState>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let receiver = state.inventory_change_notifier.subscribe();
    let stream = BroadcastStream::new(receiver).filter_map(|message| match message {
        Ok(_) => Some(Ok(inventory_changed_event())),
        Err(tokio_stream::wrappers::errors::BroadcastStreamRecvError::Lagged(_)) => {
            Some(Ok(inventory_changed_event()))
        }
    });

    Sse::new(stream).keep_alive(KeepAlive::default())
}

fn inventory_changed_event() -> Event {
    Event::default().event("inventory-changed").data("{}")
}
