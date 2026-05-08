use factstr::EventStore;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::features::register_tool::append_consequences::append_consequences;
use crate::features::register_tool::build_context::build_context;
use crate::features::register_tool::generate_consequences::generate_consequences;
use crate::features::register_tool::load_context::load_context;

const UNKNOWN_DEFAULT: &str = "unknown";
const HOME_LOCATION_DEFAULT: &str = "unassigned";
const INITIAL_CONDITION_DEFAULT: &str = "usable";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegisterToolRequest {
    pub serial_number: String,
    pub name: String,
    pub category: String,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub home_location: Option<String>,
    pub initial_condition: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisterToolResponse {
    pub tool_id: String,
    pub serial_number: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegisterToolError {
    EmptySerialNumber,
    EmptyName,
    EmptyCategory,
    SerialNumberAlreadyRegistered,
    StoreError { message: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegisterToolErrorCode {
    EmptySerialNumber,
    EmptyName,
    EmptyCategory,
    SerialNumberAlreadyRegistered,
    StoreError,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedRegisterToolRequest {
    pub serial_number: String,
    pub name: String,
    pub category: String,
    pub manufacturer: String,
    pub model: String,
    pub home_location: String,
    pub initial_condition: String,
}

impl RegisterToolError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::EmptySerialNumber => RegisterToolErrorCode::EmptySerialNumber.as_str(),
            Self::EmptyName => RegisterToolErrorCode::EmptyName.as_str(),
            Self::EmptyCategory => RegisterToolErrorCode::EmptyCategory.as_str(),
            Self::SerialNumberAlreadyRegistered => {
                RegisterToolErrorCode::SerialNumberAlreadyRegistered.as_str()
            }
            Self::StoreError { .. } => RegisterToolErrorCode::StoreError.as_str(),
        }
    }

    pub fn store_error(error: impl std::fmt::Display) -> Self {
        Self::StoreError {
            message: error.to_string(),
        }
    }
}

impl RegisterToolErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::EmptySerialNumber => "empty_serial_number",
            Self::EmptyName => "empty_name",
            Self::EmptyCategory => "empty_category",
            Self::SerialNumberAlreadyRegistered => "serial_number_already_registered",
            Self::StoreError => "store_error",
        }
    }
}

pub fn process_request(
    store: &impl EventStore,
    request: RegisterToolRequest,
) -> Result<RegisterToolResponse, RegisterToolError> {
    let normalized_request = normalize_request(request)?;
    let loaded_context = load_context(store, &normalized_request.serial_number)
        .map_err(RegisterToolError::store_error)?;
    let context = build_context(loaded_context);
    let generated_tool_id = Uuid::new_v4().to_string();
    let consequences = generate_consequences(normalized_request, generated_tool_id, context)?;

    append_consequences(store, consequences)
}

fn normalize_request(
    request: RegisterToolRequest,
) -> Result<NormalizedRegisterToolRequest, RegisterToolError> {
    let serial_number = trim_required(request.serial_number, RegisterToolError::EmptySerialNumber)?;
    let name = trim_required(request.name, RegisterToolError::EmptyName)?;
    let category = trim_required(request.category, RegisterToolError::EmptyCategory)?;

    Ok(NormalizedRegisterToolRequest {
        serial_number,
        name,
        category,
        manufacturer: trim_optional_or_default(request.manufacturer, UNKNOWN_DEFAULT),
        model: trim_optional_or_default(request.model, UNKNOWN_DEFAULT),
        home_location: trim_optional_or_default(request.home_location, HOME_LOCATION_DEFAULT),
        initial_condition: trim_optional_or_default(
            request.initial_condition,
            INITIAL_CONDITION_DEFAULT,
        ),
    })
}

fn trim_required(value: String, error: RegisterToolError) -> Result<String, RegisterToolError> {
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
