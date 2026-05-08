use factstr::EventQuery;

use crate::events::{TOOL_CHECKED_OUT_EVENT_TYPE, TOOL_REGISTERED_EVENT_TYPE};
use crate::features::return_tool::load_context::LoadedContext;
use crate::features::return_tool::process_request::CheckOutStatus;

pub struct ReturnToolContext {
    pub context_query: EventQuery,
    pub expected_context_version: Option<u64>,
    pub tool_registered: bool,
    pub current_status: CheckOutStatus,
}

pub fn build_context(loaded_context: LoadedContext) -> ReturnToolContext {
    let tool_registered = loaded_context
        .query_result
        .event_records
        .iter()
        .any(|event_record| event_record.event_type == TOOL_REGISTERED_EVENT_TYPE);
    let current_status = if loaded_context
        .query_result
        .event_records
        .last()
        .is_some_and(|event_record| event_record.event_type == TOOL_CHECKED_OUT_EVENT_TYPE)
    {
        CheckOutStatus::CheckedOut
    } else {
        CheckOutStatus::Available
    };

    ReturnToolContext {
        context_query: loaded_context.context_query,
        expected_context_version: loaded_context.query_result.current_context_version,
        tool_registered,
        current_status,
    }
}
