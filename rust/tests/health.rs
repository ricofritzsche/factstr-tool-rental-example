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

use std::env;
use std::error::Error;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use serde_json::Value;
use tower::util::ServiceExt;

#[tokio::test]
async fn health_returns_ok_when_store_is_reachable() -> Result<(), Box<dyn Error>> {
    let _ = dotenvy::dotenv();

    let admin_url = match env::var("FACTSTR_TOOL_RENTAL_POSTGRES_ADMIN_URL") {
        Ok(value) => value,
        Err(_) => {
            eprintln!(
                "skipping health integration test: FACTSTR_TOOL_RENTAL_POSTGRES_ADMIN_URL is not set"
            );
            return Ok(());
        }
    };

    let mut test_database = store::TestDatabase::create(&admin_url).await?;
    let app = routes::build_routes(test_database.open_store().await?);

    let response = app
        .clone()
        .oneshot(Request::builder().uri("/health").body(Body::empty())?)
        .await?;

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await?;
    let payload: Value = serde_json::from_slice(&body)?;

    assert_eq!(payload["status"], "ok");

    drop(app);
    test_database.cleanup().await?;

    Ok(())
}
