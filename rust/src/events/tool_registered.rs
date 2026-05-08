use serde::{Deserialize, Serialize};

pub const TOOL_REGISTERED_EVENT_TYPE: &str = "tool-registered";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolRegisteredPayload {
    pub tool_id: String,
    pub serial_number: String,
    pub name: String,
    pub category: String,
    pub manufacturer: String,
    pub model: String,
    pub home_location: String,
    pub initial_condition: String,
}
