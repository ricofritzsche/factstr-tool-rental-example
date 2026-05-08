use factstr::EventQuery;

use crate::features::register_tool::load_context::LoadedContext;

pub struct RegisterToolContext {
    pub context_query: EventQuery,
    pub expected_context_version: Option<u64>,
    pub serial_number_already_registered: bool,
}

pub fn build_context(loaded_context: LoadedContext) -> RegisterToolContext {
    RegisterToolContext {
        context_query: loaded_context.context_query,
        expected_context_version: loaded_context.query_result.current_context_version,
        serial_number_already_registered: !loaded_context.query_result.event_records.is_empty(),
    }
}
