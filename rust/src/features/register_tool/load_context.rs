use factstr::{EventFilter, EventQuery, EventStore, EventStoreError, QueryResult};
use serde_json::json;

use crate::events::TOOL_REGISTERED_EVENT_TYPE;

pub struct LoadedContext {
    pub context_query: EventQuery,
    pub query_result: QueryResult,
}

pub fn load_context(
    store: &impl EventStore,
    serial_number: &str,
) -> Result<LoadedContext, EventStoreError> {
    let context_query = EventQuery::all()
        .with_filters([EventFilter::for_event_types([TOOL_REGISTERED_EVENT_TYPE])
            .with_payload_predicates([json!({ "serial_number": serial_number })])]);

    let query_result = store.query(&context_query)?;

    Ok(LoadedContext {
        context_query,
        query_result,
    })
}
