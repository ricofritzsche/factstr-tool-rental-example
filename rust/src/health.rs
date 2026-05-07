use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use serde::Serialize;
use tracing::error;

use crate::routes::AppState;

#[derive(Serialize)]
struct HealthResponse<'a> {
    status: &'a str,
}

#[derive(Serialize)]
struct ErrorResponse<'a> {
    status: &'a str,
    message: &'a str,
}

pub async fn health(State(state): State<AppState>) -> impl IntoResponse {
    match state.store.check_connectivity().await {
        Ok(()) => (StatusCode::OK, Json(HealthResponse { status: "ok" })).into_response(),
        Err(store_error) => {
            error!(error = %store_error, "health check failed");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ErrorResponse {
                    status: "error",
                    message: "store unavailable",
                }),
            )
                .into_response()
        }
    }
}
