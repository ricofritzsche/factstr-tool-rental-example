use axum::{
    Router,
    routing::{get, post},
};

use crate::health::health;
use crate::http::check_out_tool::check_out_tool;
use crate::http::register_tool::register_tool;
use crate::store::AppStore;

#[derive(Clone)]
pub struct AppState {
    pub store: AppStore,
}

pub fn build_routes(store: AppStore) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/tools", post(register_tool))
        .route("/tools/{tool_id}/checkout", post(check_out_tool))
        .with_state(AppState { store })
}
