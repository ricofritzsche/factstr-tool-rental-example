use axum::{
    Json,
    extract::{Path, State, rejection::JsonRejection},
    http::StatusCode,
    response::IntoResponse,
};
use factstr_tool_rental_rust::features::return_tool::{
    ReturnToolError, ReturnToolRequest, ReturnToolResponse, process_request,
};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use tracing::error;

use crate::routes::AppState;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReturnToolHttpRequest {
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub returned_at: Option<OffsetDateTime>,
    pub returned_to_location: Option<String>,
    pub condition_at_return: Option<String>,
}

#[derive(Serialize)]
struct ReturnToolHttpResponse {
    tool_id: String,
    #[serde(with = "time::serde::rfc3339")]
    returned_at: OffsetDateTime,
    returned_to_location: String,
}

#[derive(Serialize)]
struct ErrorResponse<'a> {
    code: &'a str,
}

pub async fn return_tool(
    State(state): State<AppState>,
    Path(tool_id): Path<String>,
    request: Result<Json<ReturnToolHttpRequest>, JsonRejection>,
) -> impl IntoResponse {
    let request = match request {
        Ok(Json(request)) => request,
        Err(rejection) => {
            error!(error = %rejection, "invalid return tool request body");
            return error_response(StatusCode::BAD_REQUEST, "invalid_request");
        }
    };

    let feature_request = ReturnToolRequest {
        tool_id,
        returned_at: request.returned_at,
        returned_to_location: request.returned_to_location,
        condition_at_return: request.condition_at_return,
    };

    match process_request(&state.store, feature_request) {
        Ok(response) => (
            StatusCode::CREATED,
            Json(ReturnToolHttpResponse::from(response)),
        )
            .into_response(),
        Err(error) => {
            error!(code = error.code(), error = ?error, "return tool failed");
            map_error(error)
        }
    }
}

impl From<ReturnToolResponse> for ReturnToolHttpResponse {
    fn from(response: ReturnToolResponse) -> Self {
        Self {
            tool_id: response.tool_id,
            returned_at: response.returned_at,
            returned_to_location: response.returned_to_location,
        }
    }
}

fn map_error(error: ReturnToolError) -> axum::response::Response {
    match error {
        ReturnToolError::EmptyToolId => error_response(StatusCode::BAD_REQUEST, "empty_tool_id"),
        ReturnToolError::MissingReturnedAt => {
            error_response(StatusCode::BAD_REQUEST, "missing_returned_at")
        }
        ReturnToolError::ToolNotRegistered => {
            error_response(StatusCode::NOT_FOUND, "tool_not_registered")
        }
        ReturnToolError::ToolNotCheckedOut => {
            error_response(StatusCode::CONFLICT, "tool_not_checked_out")
        }
        ReturnToolError::StoreError { .. } => {
            error_response(StatusCode::INTERNAL_SERVER_ERROR, "store_error")
        }
    }
}

fn error_response(status: StatusCode, code: &'static str) -> axum::response::Response {
    (status, Json(ErrorResponse { code })).into_response()
}
