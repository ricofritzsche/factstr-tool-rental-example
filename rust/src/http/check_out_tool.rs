use axum::{
    Json,
    extract::{Path, State, rejection::JsonRejection},
    http::StatusCode,
    response::IntoResponse,
};
use factstr_tool_rental_rust::features::check_out_tool::{
    CheckOutToolError, CheckOutToolRequest, CheckOutToolResponse, process_request,
};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use tracing::error;

use crate::routes::AppState;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckOutToolHttpRequest {
    pub checked_out_to: String,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub checked_out_at: Option<OffsetDateTime>,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub due_back_at: Option<OffsetDateTime>,
    pub use_location: Option<String>,
    pub condition_at_checkout: Option<String>,
}

#[derive(Serialize)]
struct CheckOutToolHttpResponse {
    tool_id: String,
    checked_out_to: String,
    #[serde(with = "time::serde::rfc3339")]
    checked_out_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    due_back_at: OffsetDateTime,
}

#[derive(Serialize)]
struct ErrorResponse<'a> {
    code: &'a str,
}

pub async fn check_out_tool(
    State(state): State<AppState>,
    Path(tool_id): Path<String>,
    request: Result<Json<CheckOutToolHttpRequest>, JsonRejection>,
) -> impl IntoResponse {
    let request = match request {
        Ok(Json(request)) => request,
        Err(rejection) => {
            error!(error = %rejection, "invalid check out tool request body");
            return error_response(StatusCode::BAD_REQUEST, "invalid_request");
        }
    };

    let feature_request = CheckOutToolRequest {
        tool_id,
        checked_out_to: request.checked_out_to,
        checked_out_at: request.checked_out_at,
        due_back_at: request.due_back_at,
        use_location: request.use_location,
        condition_at_checkout: request.condition_at_checkout,
    };

    match process_request(&state.store, feature_request) {
        Ok(response) => (
            StatusCode::CREATED,
            Json(CheckOutToolHttpResponse::from(response)),
        )
            .into_response(),
        Err(error) => {
            error!(code = error.code(), error = ?error, "check out tool failed");
            map_error(error)
        }
    }
}

impl From<CheckOutToolResponse> for CheckOutToolHttpResponse {
    fn from(response: CheckOutToolResponse) -> Self {
        Self {
            tool_id: response.tool_id,
            checked_out_to: response.checked_out_to,
            checked_out_at: response.checked_out_at,
            due_back_at: response.due_back_at,
        }
    }
}

fn map_error(error: CheckOutToolError) -> axum::response::Response {
    match error {
        CheckOutToolError::EmptyToolId => error_response(StatusCode::BAD_REQUEST, "empty_tool_id"),
        CheckOutToolError::EmptyCheckedOutTo => {
            error_response(StatusCode::BAD_REQUEST, "empty_checked_out_to")
        }
        CheckOutToolError::MissingCheckedOutAt => {
            error_response(StatusCode::BAD_REQUEST, "missing_checked_out_at")
        }
        CheckOutToolError::MissingDueBackAt => {
            error_response(StatusCode::BAD_REQUEST, "missing_due_back_at")
        }
        CheckOutToolError::DueBackMustBeLaterThanCheckedOut => error_response(
            StatusCode::BAD_REQUEST,
            "due_back_must_be_later_than_checked_out",
        ),
        CheckOutToolError::ToolNotRegistered => {
            error_response(StatusCode::NOT_FOUND, "tool_not_registered")
        }
        CheckOutToolError::ToolAlreadyCheckedOut => {
            error_response(StatusCode::CONFLICT, "tool_already_checked_out")
        }
        CheckOutToolError::StoreError { .. } => {
            error_response(StatusCode::INTERNAL_SERVER_ERROR, "store_error")
        }
    }
}

fn error_response(status: StatusCode, code: &'static str) -> axum::response::Response {
    (status, Json(ErrorResponse { code })).into_response()
}
