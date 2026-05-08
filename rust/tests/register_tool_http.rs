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
use factstr_tool_rental_rust::events::{TOOL_REGISTERED_EVENT_TYPE, ToolRegisteredPayload};
use factstr_tool_rental_rust::features::get_inventory::{
    InventoryProjection, start_projection_in_memory,
};
use serde_json::{Value, json};
use tower::util::ServiceExt;
use uuid::Uuid;

#[tokio::test]
async fn post_tools_returns_201_for_valid_request() -> Result<(), Box<dyn Error>> {
    let store = store::AppStore::from_event_store(MemoryStore::new());
    let app = build_app(store)?;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/tools")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "serial_number": "SN-1001",
                        "name": "Rotary Hammer",
                        "category": "drilling"
                    })
                    .to_string(),
                ))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::CREATED);

    let payload = read_json(response).await?;

    assert_eq!(payload["serial_number"], "SN-1001");
    assert_eq!(
        Uuid::parse_str(
            payload["tool_id"]
                .as_str()
                .expect("tool_id should be a string")
        )?
        .to_string(),
        payload["tool_id"]
    );

    Ok(())
}

#[tokio::test]
async fn post_tools_rejects_tool_id_in_request_body() -> Result<(), Box<dyn Error>> {
    let store = store::AppStore::from_event_store(MemoryStore::new());
    let app = build_app(store)?;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/tools")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "tool_id": "not-allowed",
                        "serial_number": "SN-1001",
                        "name": "Rotary Hammer",
                        "category": "drilling"
                    })
                    .to_string(),
                ))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(read_json(response).await?["code"], "invalid_request");

    Ok(())
}

#[tokio::test]
async fn post_tools_returns_400_for_blank_serial_number() -> Result<(), Box<dyn Error>> {
    assert_error_code(
        json!({
            "serial_number": "   ",
            "name": "Rotary Hammer",
            "category": "drilling"
        }),
        StatusCode::BAD_REQUEST,
        "empty_serial_number",
    )
    .await
}

#[tokio::test]
async fn post_tools_returns_400_for_blank_name() -> Result<(), Box<dyn Error>> {
    assert_error_code(
        json!({
            "serial_number": "SN-1001",
            "name": "   ",
            "category": "drilling"
        }),
        StatusCode::BAD_REQUEST,
        "empty_name",
    )
    .await
}

#[tokio::test]
async fn post_tools_returns_400_for_blank_category() -> Result<(), Box<dyn Error>> {
    assert_error_code(
        json!({
            "serial_number": "SN-1001",
            "name": "Rotary Hammer",
            "category": "   "
        }),
        StatusCode::BAD_REQUEST,
        "empty_category",
    )
    .await
}

#[tokio::test]
async fn post_tools_returns_409_for_duplicate_serial_number() -> Result<(), Box<dyn Error>> {
    let store = store::AppStore::from_event_store(MemoryStore::new());
    let app = build_app(store.clone())?;

    let first_response = app
        .clone()
        .oneshot(build_register_tool_request(json!({
            "serial_number": "SN-1001",
            "name": "Rotary Hammer",
            "category": "drilling"
        }))?)
        .await?;
    assert_eq!(first_response.status(), StatusCode::CREATED);

    let duplicate_response = app
        .oneshot(build_register_tool_request(json!({
            "serial_number": "SN-1001",
            "name": "Second Hammer",
            "category": "drilling"
        }))?)
        .await?;

    assert_eq!(duplicate_response.status(), StatusCode::CONFLICT);
    assert_eq!(
        read_json(duplicate_response).await?["code"],
        "serial_number_already_registered"
    );

    let query_result = store.query(&EventQuery::for_event_types([TOOL_REGISTERED_EVENT_TYPE]))?;
    assert_eq!(query_result.event_records.len(), 1);

    let payload: ToolRegisteredPayload =
        serde_json::from_value(query_result.event_records[0].payload.clone())?;
    assert_eq!(payload.serial_number, "SN-1001");

    Ok(())
}

#[tokio::test]
async fn post_tools_returns_500_for_store_error() -> Result<(), Box<dyn Error>> {
    let store = store::AppStore::from_event_store(FailingStore);
    let app = routes::build_routes(store, InventoryProjection::empty());

    let response = app
        .oneshot(build_register_tool_request(json!({
            "serial_number": "SN-1001",
            "name": "Rotary Hammer",
            "category": "drilling"
        }))?)
        .await?;

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(read_json(response).await?["code"], "store_error");

    Ok(())
}

async fn assert_error_code(
    body: Value,
    expected_status: StatusCode,
    expected_code: &str,
) -> Result<(), Box<dyn Error>> {
    let store = store::AppStore::from_event_store(MemoryStore::new());
    let app = build_app(store)?;

    let response = app.oneshot(build_register_tool_request(body)?).await?;

    assert_eq!(response.status(), expected_status);
    assert_eq!(read_json(response).await?["code"], expected_code);

    Ok(())
}

fn build_register_tool_request(body: Value) -> Result<Request<Body>, axum::http::Error> {
    Request::builder()
        .method("POST")
        .uri("/tools")
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
