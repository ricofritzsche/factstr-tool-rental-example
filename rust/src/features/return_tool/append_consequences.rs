use factstr::{EventStore, EventStoreError};

use crate::features::return_tool::generate_consequences::ReturnToolConsequences;
use crate::features::return_tool::process_request::{ReturnToolError, ReturnToolResponse};

pub fn append_consequences(
    store: &impl EventStore,
    consequences: ReturnToolConsequences,
) -> Result<ReturnToolResponse, ReturnToolError> {
    match store.append_if(
        vec![consequences.new_event],
        &consequences.context_query,
        consequences.expected_context_version,
    ) {
        Ok(_) => Ok(consequences.response),
        Err(EventStoreError::ConditionalAppendConflict { .. }) => {
            Err(ReturnToolError::ToolNotCheckedOut)
        }
        Err(error) => Err(ReturnToolError::store_error(error)),
    }
}
