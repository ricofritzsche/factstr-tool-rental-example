use factstr::{EventFilter, EventQuery, EventStore, EventStoreError, QueryResult};
use serde_json::json;

use crate::events::{
    TOOL_CHECKED_OUT_EVENT_TYPE, TOOL_REGISTERED_EVENT_TYPE, TOOL_RETURNED_EVENT_TYPE,
};

pub struct LoadedContext {
    pub context_query: EventQuery,
    pub query_result: QueryResult,
}

pub fn load_context(
    store: &impl EventStore,
    tool_id: &str,
) -> Result<LoadedContext, EventStoreError> {
    let context_query = EventQuery::all().with_filters([EventFilter::for_event_types([
        TOOL_REGISTERED_EVENT_TYPE,
        TOOL_CHECKED_OUT_EVENT_TYPE,
        TOOL_RETURNED_EVENT_TYPE,
    ])
    .with_payload_predicates([json!({ "tool_id": tool_id })])]);

    let query_result = store.query(&context_query)?;

    Ok(LoadedContext {
        context_query,
        query_result,
    })
}
