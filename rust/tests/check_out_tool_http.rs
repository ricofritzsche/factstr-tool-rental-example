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
use factstr_tool_rental_rust::events::{TOOL_CHECKED_OUT_EVENT_TYPE, ToolCheckedOutPayload};
use factstr_tool_rental_rust::features::register_tool::{
    RegisterToolRequest, process_request as register_tool,
};
use serde_json::{Value, json};
use tower::util::ServiceExt;
use uuid::Uuid;

#[tokio::test]
async fn post_checkout_returns_201_for_valid_request() -> Result<(), Box<dyn Error>> {
    let memory_store = MemoryStore::new();
    let tool_id = register_sample_tool(&memory_store)?;
    let app_store = store::AppStore::from_event_store(memory_store);
    let app = routes::build_routes(app_store);

    let response = app
        .oneshot(build_checkout_request(
            &tool_id,
            json!({
                "checked_out_to": "Team Alpha",
                "checked_out_at": "2026-05-08T09:00:00Z",
                "due_back_at": "2026-05-09T09:00:00Z",
                "use_location": "job-site-7",
                "condition_at_checkout": "ready"
            }),
        )?)
        .await?;

    assert_eq!(response.status(), StatusCode::CREATED);

    let payload = read_json(response).await?;

    assert_eq!(payload["tool_id"], tool_id);
    assert_eq!(payload["checked_out_to"], "Team Alpha");
    assert_eq!(
        time::OffsetDateTime::parse(
            payload["checked_out_at"]
                .as_str()
                .expect("checked_out_at should be a string"),
            &time::format_description::well_known::Rfc3339,
        )?
        .format(&time::format_description::well_known::Rfc3339)?,
        payload["checked_out_at"]
    );
    assert_eq!(
        time::OffsetDateTime::parse(
            payload["due_back_at"]
                .as_str()
                .expect("due_back_at should be a string"),
            &time::format_description::well_known::Rfc3339,
        )?
        .format(&time::format_description::well_known::Rfc3339)?,
        payload["due_back_at"]
    );

    Ok(())
}

#[tokio::test]
async fn post_checkout_body_does_not_need_tool_id() -> Result<(), Box<dyn Error>> {
    let memory_store = MemoryStore::new();
    let tool_id = register_sample_tool(&memory_store)?;
    let app = routes::build_routes(store::AppStore::from_event_store(memory_store));

    let response = app
        .oneshot(build_checkout_request(
            &tool_id,
            json!({
                "checked_out_to": "Team Alpha",
                "checked_out_at": "2026-05-08T09:00:00Z",
                "due_back_at": "2026-05-09T09:00:00Z"
            }),
        )?)
        .await?;

    assert_eq!(response.status(), StatusCode::CREATED);

    Ok(())
}

#[tokio::test]
async fn blank_path_tool_id_returns_400() -> Result<(), Box<dyn Error>> {
    let app = routes::build_routes(store::AppStore::from_event_store(MemoryStore::new()));

    let response = app
        .oneshot(build_checkout_request(
            "%20%20",
            json!({
                "checked_out_to": "Team Alpha",
                "checked_out_at": "2026-05-08T09:00:00Z",
                "due_back_at": "2026-05-09T09:00:00Z"
            }),
        )?)
        .await?;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(read_json(response).await?["code"], "empty_tool_id");

    Ok(())
}

#[tokio::test]
async fn blank_and_missing_body_fields_map_to_400() -> Result<(), Box<dyn Error>> {
    assert_checkout_error(
        setup_registered_app().await?,
        "SN-ignored",
        json!({
            "checked_out_to": "   ",
            "checked_out_at": "2026-05-08T09:00:00Z",
            "due_back_at": "2026-05-09T09:00:00Z"
        }),
        StatusCode::BAD_REQUEST,
        "empty_checked_out_to",
    )
    .await?;

    let (tool_id, app) = setup_registered_app().await?;
    assert_checkout_error(
        (tool_id, app),
        "",
        json!({
            "checked_out_to": "Team Alpha",
            "due_back_at": "2026-05-09T09:00:00Z"
        }),
        StatusCode::BAD_REQUEST,
        "missing_checked_out_at",
    )
    .await?;

    let (tool_id, app) = setup_registered_app().await?;
    assert_checkout_error(
        (tool_id, app),
        "",
        json!({
            "checked_out_to": "Team Alpha",
            "checked_out_at": "2026-05-08T09:00:00Z"
        }),
        StatusCode::BAD_REQUEST,
        "missing_due_back_at",
    )
    .await?;

    Ok(())
}

#[tokio::test]
async fn invalid_due_back_order_returns_400() -> Result<(), Box<dyn Error>> {
    let (tool_id, app) = setup_registered_app().await?;

    let response = app
        .oneshot(build_checkout_request(
            &tool_id,
            json!({
                "checked_out_to": "Team Alpha",
                "checked_out_at": "2026-05-08T09:00:00Z",
                "due_back_at": "2026-05-08T09:00:00Z"
            }),
        )?)
        .await?;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        read_json(response).await?["code"],
        "due_back_must_be_later_than_checked_out"
    );

    Ok(())
}

#[tokio::test]
async fn unknown_tool_returns_404() -> Result<(), Box<dyn Error>> {
    let app = routes::build_routes(store::AppStore::from_event_store(MemoryStore::new()));

    let response = app
        .oneshot(build_checkout_request(
            "tool-unknown",
            json!({
                "checked_out_to": "Team Alpha",
                "checked_out_at": "2026-05-08T09:00:00Z",
                "due_back_at": "2026-05-09T09:00:00Z"
            }),
        )?)
        .await?;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(read_json(response).await?["code"], "tool_not_registered");

    Ok(())
}

#[tokio::test]
async fn already_checked_out_tool_returns_409() -> Result<(), Box<dyn Error>> {
    let memory_store = MemoryStore::new();
    let tool_id = register_sample_tool(&memory_store)?;
    let app_store = store::AppStore::from_event_store(memory_store);
    let app = routes::build_routes(app_store.clone());

    let first_response = app
        .clone()
        .oneshot(build_checkout_request(
            &tool_id,
            json!({
                "checked_out_to": "Team Alpha",
                "checked_out_at": "2026-05-08T09:00:00Z",
                "due_back_at": "2026-05-09T09:00:00Z"
            }),
        )?)
        .await?;
    assert_eq!(first_response.status(), StatusCode::CREATED);

    let second_response = app
        .oneshot(build_checkout_request(
            &tool_id,
            json!({
                "checked_out_to": "Team Beta",
                "checked_out_at": "2026-05-08T10:00:00Z",
                "due_back_at": "2026-05-09T10:00:00Z"
            }),
        )?)
        .await?;

    assert_eq!(second_response.status(), StatusCode::CONFLICT);
    assert_eq!(
        read_json(second_response).await?["code"],
        "tool_already_checked_out"
    );

    let query_result =
        app_store.query(&EventQuery::for_event_types([TOOL_CHECKED_OUT_EVENT_TYPE]))?;
    assert_eq!(query_result.event_records.len(), 1);

    let payload: ToolCheckedOutPayload =
        serde_json::from_value(query_result.event_records[0].payload.clone())?;
    assert_eq!(payload.tool_id, tool_id);

    Ok(())
}

#[tokio::test]
async fn store_error_maps_to_500_without_exposing_raw_error() -> Result<(), Box<dyn Error>> {
    let app = routes::build_routes(store::AppStore::from_event_store(FailingStore));

    let response = app
        .oneshot(build_checkout_request(
            "tool-123",
            json!({
                "checked_out_to": "Team Alpha",
                "checked_out_at": "2026-05-08T09:00:00Z",
                "due_back_at": "2026-05-09T09:00:00Z"
            }),
        )?)
        .await?;

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let payload = read_json(response).await?;
    assert_eq!(payload, json!({ "code": "store_error" }));

    Ok(())
}

async fn setup_registered_app() -> Result<(String, axum::Router), Box<dyn Error>> {
    let memory_store = MemoryStore::new();
    let tool_id = register_sample_tool(&memory_store)?;
    let app = routes::build_routes(store::AppStore::from_event_store(memory_store));

    Ok((tool_id, app))
}

async fn assert_checkout_error(
    setup: (String, axum::Router),
    override_tool_id: &str,
    body: Value,
    expected_status: StatusCode,
    expected_code: &str,
) -> Result<(), Box<dyn Error>> {
    let (tool_id, app) = setup;
    let target_tool_id = if override_tool_id.is_empty() {
        tool_id.as_str()
    } else {
        override_tool_id
    };

    let response = app
        .oneshot(build_checkout_request(target_tool_id, body)?)
        .await?;

    assert_eq!(response.status(), expected_status);
    assert_eq!(read_json(response).await?["code"], expected_code);

    Ok(())
}

fn build_checkout_request(tool_id: &str, body: Value) -> Result<Request<Body>, axum::http::Error> {
    Request::builder()
        .method("POST")
        .uri(format!("/tools/{tool_id}/checkout"))
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
}

async fn read_json(response: axum::response::Response) -> Result<Value, Box<dyn Error>> {
    let body = to_bytes(response.into_body(), usize::MAX).await?;
    Ok(serde_json::from_slice(&body)?)
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
