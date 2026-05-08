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
use factstr::{
    AppendResult, DurableStream, EventQuery, EventStore, EventStoreError, EventStream,
    HandleStream, NewEvent, QueryResult,
};
use factstr_memory::MemoryStore;
use factstr_tool_rental_rust::features::check_out_tool::{
    CheckOutToolRequest, process_request as check_out_tool,
};
use factstr_tool_rental_rust::features::get_inventory::{
    InventoryProjection, start_projection_in_memory,
};
use factstr_tool_rental_rust::features::register_tool::{
    RegisterToolRequest, process_request as register_tool,
};
use factstr_tool_rental_rust::features::return_tool::process_request as return_tool;
use serde_json::{Value, json};
use tower::util::ServiceExt;
use uuid::Uuid;

#[tokio::test]
async fn post_return_returns_201_for_valid_request() -> Result<(), Box<dyn Error>> {
    let memory_store = MemoryStore::new();
    let tool_id = register_and_check_out_tool(&memory_store)?;
    let app = build_app(store::AppStore::from_event_store(memory_store))?;

    let response = app
        .oneshot(build_return_request(
            &tool_id,
            json!({
                "returned_at": "2026-05-10T09:00:00Z",
                "returned_to_location": "warehouse-a",
                "condition_at_return": "ready"
            }),
        )?)
        .await?;

    assert_eq!(response.status(), StatusCode::CREATED);

    let payload = read_json(response).await?;

    assert_eq!(payload["tool_id"], tool_id);
    assert_eq!(payload["returned_to_location"], "warehouse-a");
    assert_eq!(
        time::OffsetDateTime::parse(
            payload["returned_at"]
                .as_str()
                .expect("returned_at should be a string"),
            &time::format_description::well_known::Rfc3339,
        )?
        .format(&time::format_description::well_known::Rfc3339)?,
        payload["returned_at"]
    );

    Ok(())
}

#[tokio::test]
async fn post_return_body_does_not_need_tool_id() -> Result<(), Box<dyn Error>> {
    let memory_store = MemoryStore::new();
    let tool_id = register_and_check_out_tool(&memory_store)?;
    let app = build_app(store::AppStore::from_event_store(memory_store))?;

    let response = app
        .oneshot(build_return_request(
            &tool_id,
            json!({
                "returned_at": "2026-05-10T09:00:00Z"
            }),
        )?)
        .await?;

    assert_eq!(response.status(), StatusCode::CREATED);

    Ok(())
}

#[tokio::test]
async fn blank_path_tool_id_returns_400() -> Result<(), Box<dyn Error>> {
    let app = build_app(store::AppStore::from_event_store(MemoryStore::new()))?;

    let response = app
        .oneshot(build_return_request(
            "%20%20",
            json!({
                "returned_at": "2026-05-10T09:00:00Z"
            }),
        )?)
        .await?;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(read_json(response).await?["code"], "empty_tool_id");

    Ok(())
}

#[tokio::test]
async fn missing_returned_at_returns_400() -> Result<(), Box<dyn Error>> {
    let (tool_id, app) = setup_checked_out_app().await?;

    let response = app
        .oneshot(build_return_request(&tool_id, json!({}))?)
        .await?;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(read_json(response).await?["code"], "missing_returned_at");

    Ok(())
}

#[tokio::test]
async fn unknown_tool_returns_404() -> Result<(), Box<dyn Error>> {
    let app = build_app(store::AppStore::from_event_store(MemoryStore::new()))?;

    let response = app
        .oneshot(build_return_request(
            "tool-unknown",
            json!({
                "returned_at": "2026-05-10T09:00:00Z"
            }),
        )?)
        .await?;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(read_json(response).await?["code"], "tool_not_registered");

    Ok(())
}

#[tokio::test]
async fn available_but_not_checked_out_tool_returns_409() -> Result<(), Box<dyn Error>> {
    let memory_store = MemoryStore::new();
    let tool_id = register_sample_tool(&memory_store)?;
    let app = build_app(store::AppStore::from_event_store(memory_store))?;

    let response = app
        .oneshot(build_return_request(
            &tool_id,
            json!({
                "returned_at": "2026-05-10T09:00:00Z"
            }),
        )?)
        .await?;

    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(read_json(response).await?["code"], "tool_not_checked_out");

    Ok(())
}

#[tokio::test]
async fn already_returned_tool_returns_409() -> Result<(), Box<dyn Error>> {
    let memory_store = MemoryStore::new();
    let tool_id = register_and_check_out_tool(&memory_store)?;
    return_tool(
        &memory_store,
        factstr_tool_rental_rust::features::return_tool::ReturnToolRequest {
            tool_id: tool_id.clone(),
            returned_at: Some(parse_time("2026-05-10T09:00:00Z")?),
            returned_to_location: None,
            condition_at_return: None,
        },
    )
    .expect("first return should succeed");
    let app = build_app(store::AppStore::from_event_store(memory_store))?;

    let response = app
        .oneshot(build_return_request(
            &tool_id,
            json!({
                "returned_at": "2026-05-10T10:00:00Z"
            }),
        )?)
        .await?;

    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(read_json(response).await?["code"], "tool_not_checked_out");

    Ok(())
}

#[tokio::test]
async fn store_error_maps_to_500_without_exposing_raw_error() -> Result<(), Box<dyn Error>> {
    let app = routes::build_routes(
        store::AppStore::from_event_store(FailingStore),
        InventoryProjection::empty(),
    );

    let response = app
        .oneshot(build_return_request(
            "tool-123",
            json!({
                "returned_at": "2026-05-10T09:00:00Z"
            }),
        )?)
        .await?;

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(read_json(response).await?, json!({ "code": "store_error" }));

    Ok(())
}

async fn setup_checked_out_app() -> Result<(String, axum::Router), Box<dyn Error>> {
    let memory_store = MemoryStore::new();
    let tool_id = register_and_check_out_tool(&memory_store)?;
    let app = build_app(store::AppStore::from_event_store(memory_store))?;

    Ok((tool_id, app))
}

fn register_and_check_out_tool(store: &impl EventStore) -> Result<String, Box<dyn Error>> {
    let tool_id = register_sample_tool(store)?;

    check_out_tool(
        store,
        CheckOutToolRequest {
            tool_id: tool_id.clone(),
            checked_out_to: "Team Alpha".to_owned(),
            checked_out_at: Some(parse_time("2026-05-08T09:00:00Z")?),
            due_back_at: Some(parse_time("2026-05-09T09:00:00Z")?),
            use_location: None,
            condition_at_checkout: None,
        },
    )
    .expect("check out should succeed");

    Ok(tool_id)
}

fn register_sample_tool(store: &impl EventStore) -> Result<String, Box<dyn Error>> {
    let response = register_tool(
        store,
        RegisterToolRequest {
            serial_number: format!("SN-{}", Uuid::new_v4()),
            name: "Rotary Hammer".to_owned(),
            category: "drilling".to_owned(),
            manufacturer: None,
            model: None,
            home_location: None,
            initial_condition: None,
        },
    )
    .expect("tool registration should succeed");

    Ok(response.tool_id)
}

fn build_return_request(tool_id: &str, body: Value) -> Result<Request<Body>, axum::http::Error> {
    Request::builder()
        .method("POST")
        .uri(format!("/tools/{tool_id}/return"))
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
}

async fn read_json(response: axum::response::Response) -> Result<Value, Box<dyn Error>> {
    let body = to_bytes(response.into_body(), usize::MAX).await?;
    Ok(serde_json::from_slice(&body)?)
}

fn build_app(store: store::AppStore) -> Result<axum::Router, Box<dyn Error>> {
    let inventory_projection = start_projection_in_memory(&store)?;
    Ok(routes::build_routes(store, inventory_projection))
}

fn parse_time(value: &str) -> Result<time::OffsetDateTime, time::error::Parse> {
    time::OffsetDateTime::parse(value, &time::format_description::well_known::Rfc3339)
}

struct FailingStore;

impl EventStore for FailingStore {
    fn query(&self, _event_query: &EventQuery) -> Result<QueryResult, EventStoreError> {
        Err(EventStoreError::BackendFailure {
            message: "simulated failure".to_owned(),
        })
    }

    fn append(&self, _new_events: Vec<NewEvent>) -> Result<AppendResult, EventStoreError> {
        Err(EventStoreError::NotImplemented {
            store: "FailingStore",
            operation: "append",
        })
    }

    fn append_if(
        &self,
        _new_events: Vec<NewEvent>,
        _context_query: &EventQuery,
        _expected_context_version: Option<u64>,
    ) -> Result<AppendResult, EventStoreError> {
        Err(EventStoreError::BackendFailure {
            message: "simulated failure".to_owned(),
        })
    }

    fn stream_all(&self, _handle: HandleStream) -> Result<EventStream, EventStoreError> {
        Err(EventStoreError::NotImplemented {
            store: "FailingStore",
            operation: "stream_all",
        })
    }

    fn stream_to(
        &self,
        _event_query: &EventQuery,
        _handle: HandleStream,
    ) -> Result<EventStream, EventStoreError> {
        Err(EventStoreError::NotImplemented {
            store: "FailingStore",
            operation: "stream_to",
        })
    }

    fn stream_all_durable(
        &self,
        _durable_stream: &DurableStream,
        _handle: HandleStream,
    ) -> Result<EventStream, EventStoreError> {
        Err(EventStoreError::NotImplemented {
            store: "FailingStore",
            operation: "stream_all_durable",
        })
    }

    fn stream_to_durable(
        &self,
        _durable_stream: &DurableStream,
        _event_query: &EventQuery,
        _handle: HandleStream,
    ) -> Result<EventStream, EventStoreError> {
        Err(EventStoreError::NotImplemented {
            store: "FailingStore",
            operation: "stream_to_durable",
        })
    }
}
