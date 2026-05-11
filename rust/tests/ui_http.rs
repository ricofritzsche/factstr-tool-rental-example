#[path = "../src/config.rs"]
mod config;
#[path = "../src/health.rs"]
mod health;
#[path = "../src/http/mod.rs"]
mod http;
#[path = "../src/routes.rs"]
mod routes;
#[path = "../src/store.rs"]
mod store;

use std::error::Error;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use factstr_memory::MemoryStore;
use factstr_tool_rental_rust::features::get_inventory::{
    InventoryChangeNotifier, start_projection_in_memory_with_notifier,
};
use tower::util::ServiceExt;

#[tokio::test]
async fn get_root_returns_ui_html() -> Result<(), Box<dyn Error>> {
    let app = build_app()?;

    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty())?)
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await?;
    let html = String::from_utf8(body.to_vec())?;
    assert!(html.contains("FACTSTR Tool Rental"));

    Ok(())
}

#[tokio::test]
async fn static_assets_return_200() -> Result<(), Box<dyn Error>> {
    let app = build_app()?;

    let app_js = app
        .clone()
        .oneshot(Request::builder().uri("/app.js").body(Body::empty())?)
        .await?;
    assert_eq!(app_js.status(), StatusCode::OK);

    let styles_css = app
        .oneshot(Request::builder().uri("/styles.css").body(Body::empty())?)
        .await?;
    assert_eq!(styles_css.status(), StatusCode::OK);

    Ok(())
}

fn build_app() -> Result<axum::Router, Box<dyn Error>> {
    let store = store::AppStore::from_event_store(MemoryStore::new());
    let inventory_change_notifier = InventoryChangeNotifier::new();
    let inventory_projection =
        start_projection_in_memory_with_notifier(&store, inventory_change_notifier.clone())?;

    Ok(routes::build_routes(
        store,
        inventory_projection,
        inventory_change_notifier,
    ))
}
