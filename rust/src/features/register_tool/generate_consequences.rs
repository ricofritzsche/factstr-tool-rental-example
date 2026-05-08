use factstr::{EventQuery, NewEvent};
use serde_json::to_value;

use crate::events::{TOOL_REGISTERED_EVENT_TYPE, ToolRegisteredPayload};
use crate::features::register_tool::build_context::RegisterToolContext;
use crate::features::register_tool::process_request::{
    NormalizedRegisterToolRequest, RegisterToolError, RegisterToolResponse,
};

pub struct RegisterToolConsequences {
    pub context_query: EventQuery,
    pub expected_context_version: Option<u64>,
    pub new_event: NewEvent,
    pub response: RegisterToolResponse,
}

pub fn generate_consequences(
    request: NormalizedRegisterToolRequest,
    generated_tool_id: String,
    context: RegisterToolContext,
) -> Result<RegisterToolConsequences, RegisterToolError> {
    if context.serial_number_already_registered {
        return Err(RegisterToolError::SerialNumberAlreadyRegistered);
    }

    let payload = ToolRegisteredPayload {
        tool_id: generated_tool_id.clone(),
        serial_number: request.serial_number.clone(),
        name: request.name,
        category: request.category,
        manufacturer: request.manufacturer,
        model: request.model,
        home_location: request.home_location,
        initial_condition: request.initial_condition,
    };

    let new_event = NewEvent::new(
        TOOL_REGISTERED_EVENT_TYPE,
        to_value(payload).map_err(RegisterToolError::store_error)?,
    );

    let response = RegisterToolResponse {
        tool_id: generated_tool_id,
        serial_number: request.serial_number,
    };

    Ok(RegisterToolConsequences {
        context_query: context.context_query,
        expected_context_version: context.expected_context_version,
        new_event,
        response,
    })
}
