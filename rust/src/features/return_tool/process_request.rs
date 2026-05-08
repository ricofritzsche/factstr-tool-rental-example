use factstr::EventStore;
use time::OffsetDateTime;

use crate::features::return_tool::append_consequences::append_consequences;
use crate::features::return_tool::build_context::build_context;
use crate::features::return_tool::generate_consequences::generate_consequences;
use crate::features::return_tool::load_context::load_context;

const RETURNED_TO_LOCATION_DEFAULT: &str = "unassigned";
const CONDITION_AT_RETURN_DEFAULT: &str = "usable";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReturnToolRequest {
    pub tool_id: String,
    pub returned_at: Option<OffsetDateTime>,
    pub returned_to_location: Option<String>,
    pub condition_at_return: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReturnToolResponse {
    pub tool_id: String,
    pub returned_at: OffsetDateTime,
    pub returned_to_location: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReturnToolError {
    EmptyToolId,
    MissingReturnedAt,
    ToolNotRegistered,
    ToolNotCheckedOut,
    StoreError { message: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReturnToolErrorCode {
    EmptyToolId,
    MissingReturnedAt,
    ToolNotRegistered,
    ToolNotCheckedOut,
    StoreError,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckOutStatus {
    Available,
    CheckedOut,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedReturnToolRequest {
    pub tool_id: String,
    pub returned_at: OffsetDateTime,
    pub returned_to_location: String,
    pub condition_at_return: String,
}

impl ReturnToolError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::EmptyToolId => ReturnToolErrorCode::EmptyToolId.as_str(),
            Self::MissingReturnedAt => ReturnToolErrorCode::MissingReturnedAt.as_str(),
            Self::ToolNotRegistered => ReturnToolErrorCode::ToolNotRegistered.as_str(),
            Self::ToolNotCheckedOut => ReturnToolErrorCode::ToolNotCheckedOut.as_str(),
            Self::StoreError { .. } => ReturnToolErrorCode::StoreError.as_str(),
        }
    }

    pub fn store_error(error: impl std::fmt::Display) -> Self {
        Self::StoreError {
            message: error.to_string(),
        }
    }
}

impl ReturnToolErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::EmptyToolId => "empty_tool_id",
            Self::MissingReturnedAt => "missing_returned_at",
            Self::ToolNotRegistered => "tool_not_registered",
            Self::ToolNotCheckedOut => "tool_not_checked_out",
            Self::StoreError => "store_error",
        }
    }
}

pub fn process_request(
    store: &impl EventStore,
    request: ReturnToolRequest,
) -> Result<ReturnToolResponse, ReturnToolError> {
    let normalized_request = normalize_request(request)?;
    let loaded_context =
        load_context(store, &normalized_request.tool_id).map_err(ReturnToolError::store_error)?;
    let context = build_context(loaded_context);
    let consequences = generate_consequences(normalized_request, context)?;

    append_consequences(store, consequences)
}

fn normalize_request(
    request: ReturnToolRequest,
) -> Result<NormalizedReturnToolRequest, ReturnToolError> {
    let tool_id = trim_required(request.tool_id, ReturnToolError::EmptyToolId)?;
    let returned_at = request
        .returned_at
        .ok_or(ReturnToolError::MissingReturnedAt)?;

    Ok(NormalizedReturnToolRequest {
        tool_id,
        returned_at,
        returned_to_location: trim_optional_or_default(
            request.returned_to_location,
            RETURNED_TO_LOCATION_DEFAULT,
        ),
        condition_at_return: trim_optional_or_default(
            request.condition_at_return,
            CONDITION_AT_RETURN_DEFAULT,
        ),
    })
}

fn trim_required(value: String, error: ReturnToolError) -> Result<String, ReturnToolError> {
    let trimmed = value.trim();

    if trimmed.is_empty() {
        Err(error)
    } else {
        Ok(trimmed.to_owned())
    }
}

fn trim_optional_or_default(value: Option<String>, default: &str) -> String {
    match value {
        Some(value) => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                default.to_owned()
            } else {
                trimmed.to_owned()
            }
        }
        None => default.to_owned(),
    }
}
