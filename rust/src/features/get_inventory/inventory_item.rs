use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::features::get_inventory::inventory_status::InventoryStatus;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InventoryItem {
    pub tool_id: String,
    pub serial_number: String,
    pub name: String,
    pub category: String,
    pub manufacturer: String,
    pub model: String,
    pub home_location: String,
    pub current_location: String,
    pub status: InventoryStatus,
    pub checked_out_to: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub due_back_at: Option<OffsetDateTime>,
}
