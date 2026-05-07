use axum::{Router, routing::get};

use crate::health::health;
use crate::store::AppStore;

#[derive(Clone)]
pub struct AppState {
    pub store: AppStore,
}

pub fn build_routes(store: AppStore) -> Router {
    Router::new()
        .route("/health", get(health))
        .with_state(AppState { store })
}
