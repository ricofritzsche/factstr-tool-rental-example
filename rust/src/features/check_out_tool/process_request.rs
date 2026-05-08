use factstr::EventStore;
use time::OffsetDateTime;

use crate::features::check_out_tool::append_consequences::append_consequences;
use crate::features::check_out_tool::build_context::build_context;
use crate::features::check_out_tool::generate_consequences::generate_consequences;
use crate::features::check_out_tool::load_context::load_context;

const USE_LOCATION_DEFAULT: &str = "unknown";
const CONDITION_AT_CHECKOUT_DEFAULT: &str = "usable";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckOutToolRequest {
    pub tool_id: String,
    pub checked_out_to: String,
    pub checked_out_at: Option<OffsetDateTime>,
    pub due_back_at: Option<OffsetDateTime>,
    pub use_location: Option<String>,
    pub condition_at_checkout: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckOutToolResponse {
    pub tool_id: String,
    pub checked_out_to: String,
    pub checked_out_at: OffsetDateTime,
    pub due_back_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckOutToolError {
    EmptyToolId,
    EmptyCheckedOutTo,
    MissingCheckedOutAt,
    MissingDueBackAt,
    DueBackMustBeLaterThanCheckedOut,
    ToolNotRegistered,
    ToolAlreadyCheckedOut,
    StoreError { message: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckOutToolErrorCode {
    EmptyToolId,
    EmptyCheckedOutTo,
    MissingCheckedOutAt,
    MissingDueBackAt,
    DueBackMustBeLaterThanCheckedOut,
    ToolNotRegistered,
    ToolAlreadyCheckedOut,
    StoreError,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedCheckOutToolRequest {
    pub tool_id: String,
    pub checked_out_to: String,
    pub checked_out_at: OffsetDateTime,
    pub due_back_at: OffsetDateTime,
    pub use_location: String,
    pub condition_at_checkout: String,
}

impl CheckOutToolError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::EmptyToolId => CheckOutToolErrorCode::EmptyToolId.as_str(),
            Self::EmptyCheckedOutTo => CheckOutToolErrorCode::EmptyCheckedOutTo.as_str(),
            Self::MissingCheckedOutAt => CheckOutToolErrorCode::MissingCheckedOutAt.as_str(),
            Self::MissingDueBackAt => CheckOutToolErrorCode::MissingDueBackAt.as_str(),
            Self::DueBackMustBeLaterThanCheckedOut => {
                CheckOutToolErrorCode::DueBackMustBeLaterThanCheckedOut.as_str()
            }
            Self::ToolNotRegistered => CheckOutToolErrorCode::ToolNotRegistered.as_str(),
            Self::ToolAlreadyCheckedOut => CheckOutToolErrorCode::ToolAlreadyCheckedOut.as_str(),
            Self::StoreError { .. } => CheckOutToolErrorCode::StoreError.as_str(),
        }
    }

    pub fn store_error(error: impl std::fmt::Display) -> Self {
        Self::StoreError {
            message: error.to_string(),
        }
    }
}

impl CheckOutToolErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::EmptyToolId => "empty_tool_id",
            Self::EmptyCheckedOutTo => "empty_checked_out_to",
            Self::MissingCheckedOutAt => "missing_checked_out_at",
            Self::MissingDueBackAt => "missing_due_back_at",
            Self::DueBackMustBeLaterThanCheckedOut => "due_back_must_be_later_than_checked_out",
            Self::ToolNotRegistered => "tool_not_registered",
            Self::ToolAlreadyCheckedOut => "tool_already_checked_out",
            Self::StoreError => "store_error",
        }
    }
}

pub fn process_request(
    store: &impl EventStore,
    request: CheckOutToolRequest,
) -> Result<CheckOutToolResponse, CheckOutToolError> {
    let normalized_request = normalize_request(request)?;
    let loaded_context =
        load_context(store, &normalized_request.tool_id).map_err(CheckOutToolError::store_error)?;
    let context = build_context(loaded_context);
    let consequences = generate_consequences(normalized_request, context)?;

    append_consequences(store, consequences)
}

fn normalize_request(
    request: CheckOutToolRequest,
) -> Result<NormalizedCheckOutToolRequest, CheckOutToolError> {
    let tool_id = trim_required(request.tool_id, CheckOutToolError::EmptyToolId)?;
    let checked_out_to =
        trim_required(request.checked_out_to, CheckOutToolError::EmptyCheckedOutTo)?;
    let checked_out_at = request
        .checked_out_at
        .ok_or(CheckOutToolError::MissingCheckedOutAt)?;
    let due_back_at = request
        .due_back_at
        .ok_or(CheckOutToolError::MissingDueBackAt)?;

    if due_back_at <= checked_out_at {
        return Err(CheckOutToolError::DueBackMustBeLaterThanCheckedOut);
    }

    Ok(NormalizedCheckOutToolRequest {
        tool_id,
        checked_out_to,
        checked_out_at,
        due_back_at,
        use_location: trim_optional_or_default(request.use_location, USE_LOCATION_DEFAULT),
        condition_at_checkout: trim_optional_or_default(
            request.condition_at_checkout,
            CONDITION_AT_CHECKOUT_DEFAULT,
        ),
    })
}

fn trim_required(value: String, error: CheckOutToolError) -> Result<String, CheckOutToolError> {
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
