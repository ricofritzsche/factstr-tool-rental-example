use factstr::{EventStore, EventStoreError};

use crate::features::register_tool::generate_consequences::RegisterToolConsequences;
use crate::features::register_tool::process_request::{RegisterToolError, RegisterToolResponse};

pub fn append_consequences(
    store: &impl EventStore,
    consequences: RegisterToolConsequences,
) -> Result<RegisterToolResponse, RegisterToolError> {
    match store.append_if(
        vec![consequences.new_event],
        &consequences.context_query,
        consequences.expected_context_version,
    ) {
        Ok(_) => Ok(consequences.response),
        Err(EventStoreError::ConditionalAppendConflict { .. }) => {
            Err(RegisterToolError::SerialNumberAlreadyRegistered)
        }
        Err(error) => Err(RegisterToolError::store_error(error)),
    }
}
