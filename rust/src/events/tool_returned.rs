use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

pub const TOOL_RETURNED_EVENT_TYPE: &str = "tool-returned";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolReturnedPayload {
    pub tool_id: String,
    #[serde(with = "time::serde::rfc3339")]
    pub returned_at: OffsetDateTime,
    pub returned_to_location: String,
    pub condition_at_return: String,
}
