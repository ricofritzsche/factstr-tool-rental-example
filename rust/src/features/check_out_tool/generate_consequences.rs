use factstr::{EventQuery, NewEvent};
use serde_json::to_value;

use crate::events::{TOOL_CHECKED_OUT_EVENT_TYPE, ToolCheckedOutPayload};
use crate::features::check_out_tool::build_context::CheckOutToolContext;
use crate::features::check_out_tool::process_request::{
    CheckOutToolError, CheckOutToolResponse, NormalizedCheckOutToolRequest,
};

pub struct CheckOutToolConsequences {
    pub context_query: EventQuery,
    pub expected_context_version: Option<u64>,
    pub new_event: NewEvent,
    pub response: CheckOutToolResponse,
}

pub fn generate_consequences(
    request: NormalizedCheckOutToolRequest,
    context: CheckOutToolContext,
) -> Result<CheckOutToolConsequences, CheckOutToolError> {
    if !context.tool_registered {
        return Err(CheckOutToolError::ToolNotRegistered);
    }

    if context.tool_currently_checked_out {
        return Err(CheckOutToolError::ToolAlreadyCheckedOut);
    }

    let payload = ToolCheckedOutPayload {
        tool_id: request.tool_id.clone(),
        checked_out_to: request.checked_out_to.clone(),
        checked_out_at: request.checked_out_at,
        due_back_at: request.due_back_at,
        use_location: request.use_location,
        condition_at_checkout: request.condition_at_checkout,
    };

    let new_event = NewEvent::new(
        TOOL_CHECKED_OUT_EVENT_TYPE,
        to_value(payload).map_err(CheckOutToolError::store_error)?,
    );

    let response = CheckOutToolResponse {
        tool_id: request.tool_id,
        checked_out_to: request.checked_out_to,
        checked_out_at: request.checked_out_at,
        due_back_at: request.due_back_at,
    };

    Ok(CheckOutToolConsequences {
        context_query: context.context_query,
        expected_context_version: context.expected_context_version,
        new_event,
        response,
    })
}
