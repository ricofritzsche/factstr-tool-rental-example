use axum::{
    Router,
    routing::{get, post},
};

use crate::health::health;
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
        .with_state(AppState { store })
}
