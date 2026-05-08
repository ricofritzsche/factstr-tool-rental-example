use std::fmt::{Display, Formatter};

use factstr::EventRecord;

use crate::events::{
    TOOL_CHECKED_OUT_EVENT_TYPE, TOOL_REGISTERED_EVENT_TYPE, TOOL_RETURNED_EVENT_TYPE,
    ToolCheckedOutPayload, ToolRegisteredPayload, ToolReturnedPayload,
};
use crate::features::get_inventory::inventory_projection::InventoryProjectionState;

#[derive(Debug, Clone)]
pub enum InventoryFact {
    Registered(ToolRegisteredPayload),
    CheckedOut(ToolCheckedOutPayload),
    Returned(ToolReturnedPayload),
    Ignored,
}

#[derive(Debug)]
pub enum ApplyFactError {
    PayloadDecode { event_type: String, message: String },
}

pub fn decode_fact(event_record: &EventRecord) -> Result<InventoryFact, ApplyFactError> {
    match event_record.event_type.as_str() {
        TOOL_REGISTERED_EVENT_TYPE => {
            let payload: ToolRegisteredPayload =
                serde_json::from_value(event_record.payload.clone())
                    .map_err(|error| ApplyFactError::payload_decode(event_record, error))?;

            Ok(InventoryFact::Registered(payload))
        }
        TOOL_CHECKED_OUT_EVENT_TYPE => {
            let payload: ToolCheckedOutPayload =
                serde_json::from_value(event_record.payload.clone())
                    .map_err(|error| ApplyFactError::payload_decode(event_record, error))?;

            Ok(InventoryFact::CheckedOut(payload))
        }
        TOOL_RETURNED_EVENT_TYPE => {
            let payload: ToolReturnedPayload = serde_json::from_value(event_record.payload.clone())
                .map_err(|error| ApplyFactError::payload_decode(event_record, error))?;

            Ok(InventoryFact::Returned(payload))
        }
        _ => Ok(InventoryFact::Ignored),
    }
}

pub fn apply_fact(state: &mut InventoryProjectionState, fact: &InventoryFact) {
    match fact {
        InventoryFact::Registered(payload) => state.apply_registered(payload),
        InventoryFact::CheckedOut(payload) => state.apply_checked_out(payload),
        InventoryFact::Returned(payload) => state.apply_returned(payload),
        InventoryFact::Ignored => {}
    }
}

impl ApplyFactError {
    fn payload_decode(event_record: &EventRecord, error: serde_json::Error) -> Self {
        Self::PayloadDecode {
            event_type: event_record.event_type.clone(),
            message: error.to_string(),
        }
    }
}

impl Display for ApplyFactError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PayloadDecode {
                event_type,
                message,
            } => write!(
                formatter,
                "failed to decode inventory projection payload for {event_type}: {message}"
            ),
        }
    }
}

impl std::error::Error for ApplyFactError {}
