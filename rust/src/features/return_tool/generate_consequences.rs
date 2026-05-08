use factstr::{EventQuery, NewEvent};
use serde_json::to_value;

use crate::events::{TOOL_RETURNED_EVENT_TYPE, ToolReturnedPayload};
use crate::features::return_tool::build_context::ReturnToolContext;
use crate::features::return_tool::process_request::{
    CheckOutStatus, NormalizedReturnToolRequest, ReturnToolError, ReturnToolResponse,
};

pub struct ReturnToolConsequences {
    pub context_query: EventQuery,
    pub expected_context_version: Option<u64>,
    pub new_event: NewEvent,
    pub response: ReturnToolResponse,
}

pub fn generate_consequences(
    request: NormalizedReturnToolRequest,
    context: ReturnToolContext,
) -> Result<ReturnToolConsequences, ReturnToolError> {
    if !context.tool_registered {
        return Err(ReturnToolError::ToolNotRegistered);
    }

    if context.current_status != CheckOutStatus::CheckedOut {
        return Err(ReturnToolError::ToolNotCheckedOut);
    }

    let payload = ToolReturnedPayload {
        tool_id: request.tool_id.clone(),
        returned_at: request.returned_at,
        returned_to_location: request.returned_to_location.clone(),
        condition_at_return: request.condition_at_return,
    };

    let new_event = NewEvent::new(
        TOOL_RETURNED_EVENT_TYPE,
        to_value(payload).map_err(ReturnToolError::store_error)?,
    );

    let response = ReturnToolResponse {
        tool_id: request.tool_id,
        returned_at: request.returned_at,
        returned_to_location: request.returned_to_location,
    };

    Ok(ReturnToolConsequences {
        context_query: context.context_query,
        expected_context_version: context.expected_context_version,
        new_event,
        response,
    })
}
