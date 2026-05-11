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
use std::thread;
use std::time::Duration;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use factstr::{EventStore, NewEvent};
use factstr_memory::MemoryStore;
use factstr_tool_rental_rust::events::{
    TOOL_CHECKED_OUT_EVENT_TYPE, TOOL_REGISTERED_EVENT_TYPE, TOOL_RETURNED_EVENT_TYPE,
    ToolCheckedOutPayload, ToolRegisteredPayload, ToolReturnedPayload,
};
use factstr_tool_rental_rust::features::get_inventory::{
    InventoryChangeNotifier, InventoryProjection, get_inventory,
    start_projection_in_memory_with_notifier,
};
use serde_json::{Value, json, to_value};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use tower::util::ServiceExt;

#[tokio::test]
async fn get_tools_returns_200_with_empty_inventory() -> Result<(), Box<dyn Error>> {
    let memory_store = MemoryStore::new();
    let app_store = store::AppStore::from_event_store(memory_store);
    let inventory_change_notifier = InventoryChangeNotifier::new();
    let projection =
        start_projection_in_memory_with_notifier(&app_store, inventory_change_notifier.clone())?;
    let app = routes::build_routes(app_store, projection, inventory_change_notifier);

    let response = app
        .oneshot(Request::builder().uri("/tools").body(Body::empty())?)
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(read_json(response).await?, json!({ "items": [] }));

    Ok(())
}

#[tokio::test]
async fn registered_checked_out_and_returned_tools_appear_in_inventory()
-> Result<(), Box<dyn Error>> {
    let memory_store = MemoryStore::new();
    seed_registered_tool(&memory_store, "tool-2", "saws", "Circular Saw", "SN-2002")?;
    seed_registered_tool(
        &memory_store,
        "tool-1",
        "drilling",
        "Angle Drill",
        "SN-1001",
    )?;
    append_checked_out_tool(
        &memory_store,
        ToolCheckedOutPayload {
            tool_id: "tool-1".to_owned(),
            checked_out_to: "Team Alpha".to_owned(),
            checked_out_at: parse_time("2026-05-08T09:00:00Z")?,
            due_back_at: parse_time("2026-05-09T09:00:00Z")?,
            use_location: "job-site-7".to_owned(),
            condition_at_checkout: "ready".to_owned(),
        },
    )?;

    let app_store = store::AppStore::from_event_store(memory_store);
    let inventory_change_notifier = InventoryChangeNotifier::new();
    let projection =
        start_projection_in_memory_with_notifier(&app_store, inventory_change_notifier.clone())?;
    let app = routes::build_routes(app_store, projection, inventory_change_notifier);
    let payload = eventually_get_tools(&app, |payload| {
        payload["items"]
            .as_array()
            .is_some_and(|items| items.len() == 2)
    })
    .await?;

    let items = payload["items"]
        .as_array()
        .expect("items should be an array");
    assert_eq!(items[0]["tool_id"], "tool-1");
    assert_eq!(items[0]["status"], "checked_out");
    assert_eq!(items[0]["checked_out_to"], "Team Alpha");
    assert_eq!(items[0]["due_back_at"], "2026-05-09T09:00:00Z");
    assert_eq!(items[1]["tool_id"], "tool-2");
    assert_eq!(items[1]["status"], "available");
    assert_eq!(items[1]["checked_out_to"], Value::Null);
    assert_eq!(items[1]["due_back_at"], Value::Null);

    Ok(())
}

#[tokio::test]
async fn returned_tools_appear_as_available() -> Result<(), Box<dyn Error>> {
    let memory_store = MemoryStore::new();
    seed_registered_tool(
        &memory_store,
        "tool-1",
        "drilling",
        "Angle Drill",
        "SN-1001",
    )?;
    append_checked_out_tool(
        &memory_store,
        ToolCheckedOutPayload {
            tool_id: "tool-1".to_owned(),
            checked_out_to: "Team Alpha".to_owned(),
            checked_out_at: parse_time("2026-05-08T09:00:00Z")?,
            due_back_at: parse_time("2026-05-09T09:00:00Z")?,
            use_location: "job-site-7".to_owned(),
            condition_at_checkout: "ready".to_owned(),
        },
    )?;
    append_returned_tool(
        &memory_store,
        ToolReturnedPayload {
            tool_id: "tool-1".to_owned(),
            returned_at: parse_time("2026-05-10T09:00:00Z")?,
            returned_to_location: "warehouse-b".to_owned(),
            condition_at_return: "ready".to_owned(),
        },
    )?;

    let app_store = store::AppStore::from_event_store(memory_store);
    let inventory_change_notifier = InventoryChangeNotifier::new();
    let projection =
        start_projection_in_memory_with_notifier(&app_store, inventory_change_notifier.clone())?;
    let app = routes::build_routes(app_store, projection, inventory_change_notifier);
    let payload = eventually_get_tools(&app, |payload| {
        payload["items"]
            .as_array()
            .and_then(|items| items.first())
            .is_some_and(|item| item["current_location"] == "warehouse-b")
    })
    .await?;

    let item = &payload["items"][0];
    assert_eq!(item["status"], "available");
    assert_eq!(item["checked_out_to"], Value::Null);
    assert_eq!(item["due_back_at"], Value::Null);

    Ok(())
}

#[tokio::test]
async fn future_committed_facts_update_http_inventory_after_startup() -> Result<(), Box<dyn Error>>
{
    let app_store = store::AppStore::from_event_store(MemoryStore::new());
    let inventory_change_notifier = InventoryChangeNotifier::new();
    let projection =
        start_projection_in_memory_with_notifier(&app_store, inventory_change_notifier.clone())?;
    let app = routes::build_routes(app_store.clone(), projection, inventory_change_notifier);

    seed_registered_tool(&app_store, "tool-1", "drilling", "Angle Drill", "SN-1001")?;

    let payload = eventually_get_tools(&app, |payload| {
        payload["items"]
            .as_array()
            .is_some_and(|items| items.len() == 1)
    })
    .await?;

    assert_eq!(payload["items"][0]["tool_id"], "tool-1");

    Ok(())
}

#[tokio::test]
async fn get_tools_orders_by_category_name_and_serial_number() -> Result<(), Box<dyn Error>> {
    let memory_store = MemoryStore::new();
    seed_registered_tool(&memory_store, "tool-3", "saws", "Circular Saw", "SN-2002")?;
    seed_registered_tool(
        &memory_store,
        "tool-2",
        "drilling",
        "Angle Drill",
        "SN-3001",
    )?;
    seed_registered_tool(
        &memory_store,
        "tool-1",
        "drilling",
        "Angle Drill",
        "SN-1001",
    )?;

    let app_store = store::AppStore::from_event_store(memory_store);
    let inventory_change_notifier = InventoryChangeNotifier::new();
    let projection =
        start_projection_in_memory_with_notifier(&app_store, inventory_change_notifier.clone())?;
    let app = routes::build_routes(app_store, projection, inventory_change_notifier);
    let payload = eventually_get_tools(&app, |payload| {
        payload["items"]
            .as_array()
            .is_some_and(|items| items.len() == 3)
    })
    .await?;

    let ordered_tool_ids = payload["items"]
        .as_array()
        .expect("items should be an array")
        .iter()
        .map(|item| {
            item["tool_id"]
                .as_str()
                .expect("tool_id should be a string")
        })
        .collect::<Vec<_>>();

    assert_eq!(ordered_tool_ids, vec!["tool-1", "tool-2", "tool-3"]);

    Ok(())
}

#[tokio::test]
async fn get_tools_reads_the_maintained_projection_without_direct_store_queries()
-> Result<(), Box<dyn Error>> {
    let projection_source_store = store::AppStore::from_event_store(MemoryStore::new());
    seed_registered_tool(
        &projection_source_store,
        "tool-1",
        "drilling",
        "Angle Drill",
        "SN-1001",
    )?;
    let projection = start_projection_in_memory_with_notifier(
        &projection_source_store,
        InventoryChangeNotifier::new(),
    )?;
    let payload = eventually_inventory_payload(&projection, |payload| {
        payload["items"]
            .as_array()
            .is_some_and(|items| items.len() == 1)
    })?;
    assert_eq!(payload["items"][0]["tool_id"], "tool-1");

    let app = routes::build_routes(
        store::AppStore::from_event_store(FailingStore),
        projection,
        InventoryChangeNotifier::new(),
    );
    let response = app
        .oneshot(Request::builder().uri("/tools").body(Body::empty())?)
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(read_json(response).await?["items"][0]["tool_id"], "tool-1");

    Ok(())
}

#[tokio::test]
async fn get_tools_events_returns_sse_response() -> Result<(), Box<dyn Error>> {
    let app_store = store::AppStore::from_event_store(MemoryStore::new());
    let inventory_change_notifier = InventoryChangeNotifier::new();
    let projection =
        start_projection_in_memory_with_notifier(&app_store, inventory_change_notifier.clone())?;
    let app = routes::build_routes(app_store, projection, inventory_change_notifier);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/tools/events")
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get(axum::http::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok()),
        Some("text/event-stream")
    );

    Ok(())
}

fn seed_registered_tool(
    store: &impl EventStore,
    tool_id: &str,
    category: &str,
    name: &str,
    serial_number: &str,
) -> Result<(), Box<dyn Error>> {
    append_fact(
        store,
        TOOL_REGISTERED_EVENT_TYPE,
        ToolRegisteredPayload {
            tool_id: tool_id.to_owned(),
            serial_number: serial_number.to_owned(),
            name: name.to_owned(),
            category: category.to_owned(),
            manufacturer: "Bosch".to_owned(),
            model: "GBH 2-26".to_owned(),
            home_location: "warehouse-a".to_owned(),
            initial_condition: "ready".to_owned(),
        },
    )
}

fn append_checked_out_tool(
    store: &impl EventStore,
    payload: ToolCheckedOutPayload,
) -> Result<(), Box<dyn Error>> {
    append_fact(store, TOOL_CHECKED_OUT_EVENT_TYPE, payload)
}

fn append_returned_tool(
    store: &impl EventStore,
    payload: ToolReturnedPayload,
) -> Result<(), Box<dyn Error>> {
    append_fact(store, TOOL_RETURNED_EVENT_TYPE, payload)
}

fn append_fact(
    store: &impl EventStore,
    event_type: &str,
    payload: impl serde::Serialize,
) -> Result<(), Box<dyn Error>> {
    store.append(vec![NewEvent::new(event_type, to_value(payload)?)])?;
    Ok(())
}

async fn eventually_get_tools(
    app: &axum::Router,
    predicate: impl Fn(&Value) -> bool,
) -> Result<Value, Box<dyn Error>> {
    for _ in 0..100 {
        let response = app
            .clone()
            .oneshot(Request::builder().uri("/tools").body(Body::empty())?)
            .await?;
        let payload = read_json(response).await?;
        if predicate(&payload) {
            return Ok(payload);
        }
        thread::sleep(Duration::from_millis(10));
    }

    Err("inventory endpoint did not reach the expected state".into())
}

fn eventually_inventory_payload(
    projection: &InventoryProjection,
    predicate: impl Fn(&Value) -> bool,
) -> Result<Value, Box<dyn Error>> {
    for _ in 0..100 {
        let payload = json!({
            "items": get_inventory(projection)?
        });
        if predicate(&payload) {
            return Ok(payload);
        }
        thread::sleep(Duration::from_millis(10));
    }

    Err("inventory projection did not reach the expected state".into())
}

async fn read_json(response: axum::response::Response) -> Result<Value, Box<dyn Error>> {
    let body = to_bytes(response.into_body(), usize::MAX).await?;
    Ok(serde_json::from_slice(&body)?)
}

fn parse_time(value: &str) -> Result<OffsetDateTime, time::error::Parse> {
    OffsetDateTime::parse(value, &Rfc3339)
}

struct FailingStore;

impl EventStore for FailingStore {
    fn query(
        &self,
        _event_query: &factstr::EventQuery,
    ) -> Result<factstr::QueryResult, factstr::EventStoreError> {
        Err(factstr::EventStoreError::BackendFailure {
            message: "simulated failure".to_owned(),
        })
    }

    fn append(
        &self,
        _new_events: Vec<factstr::NewEvent>,
    ) -> Result<factstr::AppendResult, factstr::EventStoreError> {
        Err(factstr::EventStoreError::NotImplemented {
            store: "FailingStore",
            operation: "append",
        })
    }

    fn append_if(
        &self,
        _new_events: Vec<factstr::NewEvent>,
        _context_query: &factstr::EventQuery,
        _expected_context_version: Option<u64>,
    ) -> Result<factstr::AppendResult, factstr::EventStoreError> {
        Err(factstr::EventStoreError::BackendFailure {
            message: "simulated failure".to_owned(),
        })
    }

    fn stream_all(
        &self,
        _handle: factstr::HandleStream,
    ) -> Result<factstr::EventStream, factstr::EventStoreError> {
        Err(factstr::EventStoreError::NotImplemented {
            store: "FailingStore",
            operation: "stream_all",
        })
    }

    fn stream_to(
        &self,
        _event_query: &factstr::EventQuery,
        _handle: factstr::HandleStream,
    ) -> Result<factstr::EventStream, factstr::EventStoreError> {
        Err(factstr::EventStoreError::NotImplemented {
            store: "FailingStore",
            operation: "stream_to",
        })
    }

    fn stream_all_durable(
        &self,
        _durable_stream: &factstr::DurableStream,
        _handle: factstr::HandleStream,
    ) -> Result<factstr::EventStream, factstr::EventStoreError> {
        Err(factstr::EventStoreError::NotImplemented {
            store: "FailingStore",
            operation: "stream_all_durable",
        })
    }

    fn stream_to_durable(
        &self,
        _durable_stream: &factstr::DurableStream,
        _event_query: &factstr::EventQuery,
        _handle: factstr::HandleStream,
    ) -> Result<factstr::EventStream, factstr::EventStoreError> {
        Err(factstr::EventStoreError::NotImplemented {
            store: "FailingStore",
            operation: "stream_to_durable",
        })
    }
}
