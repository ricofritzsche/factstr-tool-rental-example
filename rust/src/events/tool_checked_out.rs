use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

pub const TOOL_CHECKED_OUT_EVENT_TYPE: &str = "tool-checked-out";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolCheckedOutPayload {
    pub tool_id: String,
    pub checked_out_to: String,
    #[serde(with = "time::serde::rfc3339")]
    pub checked_out_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub due_back_at: OffsetDateTime,
    pub use_location: String,
    pub condition_at_checkout: String,
}
