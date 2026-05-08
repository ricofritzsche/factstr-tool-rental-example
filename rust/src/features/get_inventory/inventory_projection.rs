use std::collections::HashMap;
use std::fmt::{Display, Formatter};

use crate::events::{ToolCheckedOutPayload, ToolRegisteredPayload, ToolReturnedPayload};
use crate::features::get_inventory::inventory_item::InventoryItem;
use crate::features::get_inventory::inventory_status::InventoryStatus;

#[derive(Debug, Default)]
pub struct InventoryProjectionState {
    items_by_tool_id: HashMap<String, InventoryItem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InventoryProjectionError {
    StoreError { message: String },
    LockPoisoned,
    InvalidStatus(String),
}

impl InventoryProjectionState {
    pub fn from_items(items: Vec<InventoryItem>) -> Self {
        Self {
            items_by_tool_id: items
                .into_iter()
                .map(|item| (item.tool_id.clone(), item))
                .collect(),
        }
    }

    pub fn list_items(&self) -> Vec<InventoryItem> {
        let mut items = self.items_by_tool_id.values().cloned().collect::<Vec<_>>();
        items.sort_by(|left, right| {
            left.category
                .cmp(&right.category)
                .then_with(|| left.name.cmp(&right.name))
                .then_with(|| left.serial_number.cmp(&right.serial_number))
        });
        items
    }

    pub fn apply_registered(&mut self, payload: &ToolRegisteredPayload) {
        self.items_by_tool_id.insert(
            payload.tool_id.clone(),
            InventoryItem {
                tool_id: payload.tool_id.clone(),
                serial_number: payload.serial_number.clone(),
                name: payload.name.clone(),
                category: payload.category.clone(),
                manufacturer: payload.manufacturer.clone(),
                model: payload.model.clone(),
                home_location: payload.home_location.clone(),
                current_location: payload.home_location.clone(),
                status: InventoryStatus::Available,
                checked_out_to: None,
                due_back_at: None,
            },
        );
    }

    pub fn apply_checked_out(&mut self, payload: &ToolCheckedOutPayload) {
        let Some(item) = self.items_by_tool_id.get_mut(&payload.tool_id) else {
            return;
        };

        item.current_location = payload.use_location.clone();
        item.status = InventoryStatus::CheckedOut;
        item.checked_out_to = Some(payload.checked_out_to.clone());
        item.due_back_at = Some(payload.due_back_at);
    }

    pub fn apply_returned(&mut self, payload: &ToolReturnedPayload) {
        let Some(item) = self.items_by_tool_id.get_mut(&payload.tool_id) else {
            return;
        };

        item.current_location = payload.returned_to_location.clone();
        item.status = InventoryStatus::Available;
        item.checked_out_to = None;
        item.due_back_at = None;
    }
}

impl InventoryProjectionError {
    pub fn store_error(error: impl Display) -> Self {
        Self::StoreError {
            message: error.to_string(),
        }
    }
}

impl Display for InventoryProjectionError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StoreError { message } => {
                write!(formatter, "inventory projection store error: {message}")
            }
            Self::LockPoisoned => write!(formatter, "inventory projection lock poisoned"),
            Self::InvalidStatus(status) => {
                write!(
                    formatter,
                    "invalid inventory status stored in projection: {status}"
                )
            }
        }
    }
}

impl std::error::Error for InventoryProjectionError {}
