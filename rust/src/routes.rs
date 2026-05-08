use axum::{
    Router,
    routing::{get, post},
};
use factstr_tool_rental_rust::features::get_inventory::InventoryProjection;

use crate::health::health;
use crate::http::check_out_tool::check_out_tool;
use crate::http::get_inventory::get_inventory_handler;
use crate::http::register_tool::register_tool;
use crate::http::return_tool::return_tool;
use crate::store::AppStore;

#[derive(Clone)]
pub struct AppState {
    pub store: AppStore,
    pub inventory_projection: InventoryProjection,
}

pub fn build_routes(store: AppStore, inventory_projection: InventoryProjection) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/tools", get(get_inventory_handler).post(register_tool))
        .route("/tools/{tool_id}/checkout", post(check_out_tool))
        .route("/tools/{tool_id}/return", post(return_tool))
        .with_state(AppState {
            store,
            inventory_projection,
        })
}
