use factstr::EventQuery;

use crate::events::{TOOL_CHECKED_OUT_EVENT_TYPE, TOOL_REGISTERED_EVENT_TYPE};
use crate::features::check_out_tool::load_context::LoadedContext;

pub struct CheckOutToolContext {
    pub context_query: EventQuery,
    pub expected_context_version: Option<u64>,
    pub tool_registered: bool,
    pub tool_currently_checked_out: bool,
}

pub fn build_context(loaded_context: LoadedContext) -> CheckOutToolContext {
    let tool_registered = loaded_context
        .query_result
        .event_records
        .iter()
        .any(|event_record| event_record.event_type == TOOL_REGISTERED_EVENT_TYPE);
    let tool_currently_checked_out = loaded_context
        .query_result
        .event_records
        .last()
        .is_some_and(|event_record| event_record.event_type == TOOL_CHECKED_OUT_EVENT_TYPE);

    CheckOutToolContext {
        context_query: loaded_context.context_query,
        expected_context_version: loaded_context.query_result.current_context_version,
        tool_registered,
        tool_currently_checked_out,
    }
}
