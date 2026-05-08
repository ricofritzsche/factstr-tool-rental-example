use factstr::{EventStore, EventStoreError};

use crate::features::check_out_tool::generate_consequences::CheckOutToolConsequences;
use crate::features::check_out_tool::process_request::{CheckOutToolError, CheckOutToolResponse};

pub fn append_consequences(
    store: &impl EventStore,
    consequences: CheckOutToolConsequences,
) -> Result<CheckOutToolResponse, CheckOutToolError> {
    match store.append_if(
        vec![consequences.new_event],
        &consequences.context_query,
        consequences.expected_context_version,
    ) {
        Ok(_) => Ok(consequences.response),
        Err(EventStoreError::ConditionalAppendConflict { .. }) => {
            Err(CheckOutToolError::ToolAlreadyCheckedOut)
        }
        Err(error) => Err(CheckOutToolError::store_error(error)),
    }
}
