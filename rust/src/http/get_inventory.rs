use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use factstr_tool_rental_rust::features::get_inventory::{InventoryItem, get_inventory};
use serde::Serialize;
use tracing::error;

use crate::routes::AppState;

#[derive(Serialize)]
struct GetInventoryHttpResponse {
    items: Vec<InventoryItem>,
}

#[derive(Serialize)]
struct ErrorResponse<'a> {
    code: &'a str,
}

pub async fn get_inventory_handler(State(state): State<AppState>) -> impl IntoResponse {
    match get_inventory(&state.inventory_projection) {
        Ok(items) => (StatusCode::OK, Json(GetInventoryHttpResponse { items })).into_response(),
        Err(error) => {
            error!(error = %error, "get inventory failed");
            error_response(StatusCode::INTERNAL_SERVER_ERROR, "store_error")
        }
    }
}

fn error_response(status: StatusCode, code: &'static str) -> axum::response::Response {
    (status, Json(ErrorResponse { code })).into_response()
}
