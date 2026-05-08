use crate::routes::AppState;
use axum::{
    Json,
    extract::{State, rejection::JsonRejection},
    http::StatusCode,
    response::IntoResponse,
};
use factstr_tool_rental_rust::features::register_tool::{
    RegisterToolError, RegisterToolRequest, RegisterToolResponse, process_request,
};
use serde::Serialize;
use tracing::error;

#[derive(Serialize)]
struct RegisterToolHttpResponse {
    tool_id: String,
    serial_number: String,
}

#[derive(Serialize)]
struct ErrorResponse<'a> {
    code: &'a str,
}

pub async fn register_tool(
    State(state): State<AppState>,
    request: Result<Json<RegisterToolRequest>, JsonRejection>,
) -> impl IntoResponse {
    let request = match request {
        Ok(Json(request)) => request,
        Err(rejection) => {
            error!(error = %rejection, "invalid register tool request body");
            return error_response(StatusCode::BAD_REQUEST, "invalid_request");
        }
    };

    match process_request(&state.store, request) {
        Ok(response) => (
            StatusCode::CREATED,
            Json(RegisterToolHttpResponse::from(response)),
        )
            .into_response(),
        Err(error) => {
            error!(code = error.code(), error = ?error, "register tool failed");
            map_error(error)
        }
    }
}

impl From<RegisterToolResponse> for RegisterToolHttpResponse {
    fn from(response: RegisterToolResponse) -> Self {
        Self {
            tool_id: response.tool_id,
            serial_number: response.serial_number,
        }
    }
}

fn map_error(error: RegisterToolError) -> axum::response::Response {
    match error {
        RegisterToolError::EmptySerialNumber => {
            error_response(StatusCode::BAD_REQUEST, "empty_serial_number")
        }
        RegisterToolError::EmptyName => error_response(StatusCode::BAD_REQUEST, "empty_name"),
        RegisterToolError::EmptyCategory => {
            error_response(StatusCode::BAD_REQUEST, "empty_category")
        }
        RegisterToolError::SerialNumberAlreadyRegistered => {
            error_response(StatusCode::CONFLICT, "serial_number_already_registered")
        }
        RegisterToolError::StoreError { .. } => {
            error_response(StatusCode::INTERNAL_SERVER_ERROR, "store_error")
        }
    }
}

fn error_response(status: StatusCode, code: &'static str) -> axum::response::Response {
    (status, Json(ErrorResponse { code })).into_response()
}
