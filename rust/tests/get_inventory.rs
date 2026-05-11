#[path = "../src/config.rs"]
mod config;
#[path = "../src/store.rs"]
mod store;

use std::env;
use std::str::FromStr;
use std::thread;
use std::time::Duration;

use factstr::{DurableStream, EventFilter, EventQuery, EventStore, NewEvent, StreamHandlerError};
use factstr_memory::MemoryStore;
use factstr_tool_rental_rust::events::{
    TOOL_CHECKED_OUT_EVENT_TYPE, TOOL_REGISTERED_EVENT_TYPE, TOOL_RETURNED_EVENT_TYPE,
    ToolCheckedOutPayload, ToolRegisteredPayload, ToolReturnedPayload,
};
use factstr_tool_rental_rust::features::{
    check_out_tool::{CheckOutToolRequest, process_request as check_out_tool},
    get_inventory::{
        InventoryChangeNotifier, InventoryItem, InventoryProjection, InventoryStatus,
        get_inventory, projection_schema::schema_statements, start_projection,
        start_projection_in_memory, start_projection_in_memory_with_notifier,
        start_projection_with_notifier,
    },
    register_tool::{RegisterToolRequest, process_request as register_tool},
};
use factstr_tool_rental_rust::projection_database::ProjectionDatabase;
use serde_json::to_value;
use sqlx::{ConnectOptions, Connection, PgConnection};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

#[test]
fn empty_inventory_returns_empty_list() -> Result<(), Box<dyn std::error::Error>> {
    let projection = InventoryProjection::empty();

    assert!(get_inventory(&projection)?.is_empty());

    Ok(())
}

#[test]
fn projection_runtime_has_no_sqlite_or_local_file_dependencies() {
    let cargo_toml = include_str!("../Cargo.toml");
    let config_rs = include_str!("../src/config.rs");
    let inventory_projection_rs =
        include_str!("../src/features/get_inventory/inventory_projection.rs");
    let start_projection_rs = include_str!("../src/features/get_inventory/start_projection.rs");

    assert!(!cargo_toml.contains("rusqlite"));
    assert!(!cargo_toml.contains("\npostgres ="));
    assert!(cargo_toml.contains("\nsqlx ="));
    assert!(!config_rs.contains("inventory_projection_database_path"));
    assert!(!config_rs.contains(".sqlite"));
    assert!(!inventory_projection_rs.contains("ProjectionStore"));
    assert!(!inventory_projection_rs.contains("ProjectionDatabase"));
    assert!(!inventory_projection_rs.contains("open_postgres"));
    assert!(!inventory_projection_rs.contains("store_handle"));
    assert!(!start_projection_rs.contains("ProjectionPersistBridge"));
    assert!(!start_projection_rs.contains("PersistCommand"));
    assert!(!start_projection_rs.contains("mpsc::"));
    assert!(!start_projection_rs.contains("block_in_place"));
    assert!(!start_projection_rs.contains("block_on"));
    assert!(!start_projection_rs.contains("new_current_thread"));
    assert!(!start_projection_rs.contains("thread::spawn"));
}

#[test]
fn applying_tool_registered_creates_available_inventory_item()
-> Result<(), Box<dyn std::error::Error>> {
    let store = MemoryStore::new();
    append_registered_tool(
        &store,
        ToolRegisteredPayload {
            tool_id: "tool-1".to_owned(),
            serial_number: "SN-1001".to_owned(),
            name: "Rotary Hammer".to_owned(),
            category: "drilling".to_owned(),
            manufacturer: "Bosch".to_owned(),
            model: "GBH 2-26".to_owned(),
            home_location: "warehouse-a".to_owned(),
            initial_condition: "ready".to_owned(),
        },
    )?;

    let projection = start_projection_in_memory(&store)?;
    let items = eventually_inventory(&projection, |items| items.len() == 1)?;

    assert_eq!(
        items,
        vec![InventoryItem {
            tool_id: "tool-1".to_owned(),
            serial_number: "SN-1001".to_owned(),
            name: "Rotary Hammer".to_owned(),
            category: "drilling".to_owned(),
            manufacturer: "Bosch".to_owned(),
            model: "GBH 2-26".to_owned(),
            home_location: "warehouse-a".to_owned(),
            current_location: "warehouse-a".to_owned(),
            status: InventoryStatus::Available,
            checked_out_to: None,
            due_back_at: None,
        }]
    );

    Ok(())
}

#[test]
fn applying_tool_checked_out_updates_status_and_checkout_data()
-> Result<(), Box<dyn std::error::Error>> {
    let store = MemoryStore::new();
    seed_registered_tool(&store, "tool-1", "drilling", "Rotary Hammer", "SN-1001")?;
    let due_back_at = parse_time("2026-05-09T09:00:00Z")?;

    append_checked_out_tool(
        &store,
        ToolCheckedOutPayload {
            tool_id: "tool-1".to_owned(),
            checked_out_to: "Team Alpha".to_owned(),
            checked_out_at: parse_time("2026-05-08T09:00:00Z")?,
            due_back_at,
            use_location: "job-site-7".to_owned(),
            condition_at_checkout: "ready".to_owned(),
        },
    )?;

    let projection = start_projection_in_memory(&store)?;
    let items = eventually_inventory(&projection, |items| {
        items
            .first()
            .is_some_and(|item| item.status == InventoryStatus::CheckedOut)
    })?;

    assert_eq!(items[0].status, InventoryStatus::CheckedOut);
    assert_eq!(items[0].current_location, "job-site-7");
    assert_eq!(items[0].checked_out_to.as_deref(), Some("Team Alpha"));
    assert_eq!(items[0].due_back_at, Some(due_back_at));

    Ok(())
}

#[test]
fn applying_tool_returned_updates_status_and_clears_checkout_data()
-> Result<(), Box<dyn std::error::Error>> {
    let store = MemoryStore::new();
    seed_registered_tool(&store, "tool-1", "drilling", "Rotary Hammer", "SN-1001")?;
    append_checked_out_tool(
        &store,
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
        &store,
        ToolReturnedPayload {
            tool_id: "tool-1".to_owned(),
            returned_at: parse_time("2026-05-10T09:00:00Z")?,
            returned_to_location: "warehouse-b".to_owned(),
            condition_at_return: "ready".to_owned(),
        },
    )?;

    let projection = start_projection_in_memory(&store)?;
    let items = eventually_inventory(&projection, |items| {
        items
            .first()
            .is_some_and(|item| item.current_location == "warehouse-b")
    })?;

    assert_eq!(items[0].status, InventoryStatus::Available);
    assert_eq!(items[0].current_location, "warehouse-b");
    assert_eq!(items[0].checked_out_to, None);
    assert_eq!(items[0].due_back_at, None);

    Ok(())
}

#[test]
fn unknown_checkout_and_return_facts_are_ignored() -> Result<(), Box<dyn std::error::Error>> {
    let store = MemoryStore::new();
    seed_registered_tool(&store, "tool-1", "drilling", "Rotary Hammer", "SN-1001")?;
    append_checked_out_tool(
        &store,
        ToolCheckedOutPayload {
            tool_id: "tool-unknown".to_owned(),
            checked_out_to: "Team Alpha".to_owned(),
            checked_out_at: parse_time("2026-05-08T09:00:00Z")?,
            due_back_at: parse_time("2026-05-09T09:00:00Z")?,
            use_location: "job-site-7".to_owned(),
            condition_at_checkout: "ready".to_owned(),
        },
    )?;
    append_returned_tool(
        &store,
        ToolReturnedPayload {
            tool_id: "tool-ghost".to_owned(),
            returned_at: parse_time("2026-05-10T09:00:00Z")?,
            returned_to_location: "warehouse-b".to_owned(),
            condition_at_return: "ready".to_owned(),
        },
    )?;

    let projection = start_projection_in_memory(&store)?;
    let items = eventually_inventory(&projection, |items| items.len() == 1)?;

    assert_eq!(items[0].tool_id, "tool-1");
    assert_eq!(items[0].status, InventoryStatus::Available);
    assert_eq!(items[0].current_location, "warehouse-a");

    Ok(())
}

#[test]
fn inventory_is_ordered_by_category_name_and_serial_number()
-> Result<(), Box<dyn std::error::Error>> {
    let store = MemoryStore::new();
    seed_registered_tool(&store, "tool-2", "saws", "Circular Saw", "SN-2002")?;
    seed_registered_tool(&store, "tool-3", "drilling", "Angle Drill", "SN-3001")?;
    seed_registered_tool(&store, "tool-1", "drilling", "Angle Drill", "SN-1001")?;

    let projection = start_projection_in_memory(&store)?;
    let items = eventually_inventory(&projection, |items| items.len() == 3)?;

    assert_eq!(
        items
            .iter()
            .map(|item| item.tool_id.as_str())
            .collect::<Vec<_>>(),
        vec!["tool-1", "tool-3", "tool-2"]
    );

    Ok(())
}

#[test]
fn durable_stream_replay_builds_projection_from_existing_facts()
-> Result<(), Box<dyn std::error::Error>> {
    let store = MemoryStore::new();
    seed_registered_tool(&store, "tool-1", "drilling", "Rotary Hammer", "SN-1001")?;
    append_checked_out_tool(
        &store,
        ToolCheckedOutPayload {
            tool_id: "tool-1".to_owned(),
            checked_out_to: "Team Alpha".to_owned(),
            checked_out_at: parse_time("2026-05-08T09:00:00Z")?,
            due_back_at: parse_time("2026-05-09T09:00:00Z")?,
            use_location: "job-site-7".to_owned(),
            condition_at_checkout: "ready".to_owned(),
        },
    )?;

    let projection = start_projection_in_memory(&store)?;
    let items = eventually_inventory(&projection, |items| {
        items
            .first()
            .is_some_and(|item| item.status == InventoryStatus::CheckedOut)
    })?;

    assert_eq!(items[0].tool_id, "tool-1");
    assert_eq!(items[0].status, InventoryStatus::CheckedOut);

    Ok(())
}

#[test]
fn future_committed_facts_update_the_projection_after_startup()
-> Result<(), Box<dyn std::error::Error>> {
    let store = MemoryStore::new();
    let projection = start_projection_in_memory(&store)?;

    seed_registered_tool(&store, "tool-1", "drilling", "Rotary Hammer", "SN-1001")?;
    let after_registration = eventually_inventory(&projection, |items| items.len() == 1)?;
    assert_eq!(after_registration[0].status, InventoryStatus::Available);

    append_checked_out_tool(
        &store,
        ToolCheckedOutPayload {
            tool_id: "tool-1".to_owned(),
            checked_out_to: "Team Alpha".to_owned(),
            checked_out_at: parse_time("2026-05-08T09:00:00Z")?,
            due_back_at: parse_time("2026-05-09T09:00:00Z")?,
            use_location: "job-site-7".to_owned(),
            condition_at_checkout: "ready".to_owned(),
        },
    )?;

    let after_checkout = eventually_inventory(&projection, |items| {
        items
            .first()
            .is_some_and(|item| item.status == InventoryStatus::CheckedOut)
    })?;
    assert_eq!(after_checkout[0].current_location, "job-site-7");

    Ok(())
}

#[test]
fn subscribers_receive_inventory_changed_after_projection_updates()
-> Result<(), Box<dyn std::error::Error>> {
    let store = MemoryStore::new();
    let inventory_change_notifier = InventoryChangeNotifier::new();
    let mut subscriber = inventory_change_notifier.subscribe();
    let _projection = start_projection_in_memory_with_notifier(&store, inventory_change_notifier)?;

    append_registered_tool(
        &store,
        ToolRegisteredPayload {
            tool_id: "tool-1".to_owned(),
            serial_number: "SN-1001".to_owned(),
            name: "Rotary Hammer".to_owned(),
            category: "drilling".to_owned(),
            manufacturer: "Bosch".to_owned(),
            model: "GBH 2-26".to_owned(),
            home_location: "warehouse-a".to_owned(),
            initial_condition: "ready".to_owned(),
        },
    )?;

    eventually_notification(&mut subscriber)?;

    Ok(())
}

#[tokio::test]
async fn durable_projection_survives_restart_with_persisted_state()
-> Result<(), Box<dyn std::error::Error>> {
    let _ = dotenvy::dotenv();

    let admin_url = match env::var("FACTSTR_TOOL_RENTAL_POSTGRES_ADMIN_URL") {
        Ok(value) => value,
        Err(_) => {
            eprintln!(
                "skipping inventory restart test: FACTSTR_TOOL_RENTAL_POSTGRES_ADMIN_URL is not set"
            );
            return Ok(());
        }
    };

    let mut test_database = store::TestDatabase::create(&admin_url).await?;

    let tool_id = {
        let store = test_database.open_store().await?;
        let projection_database =
            ProjectionDatabase::connect(&admin_url, test_database.database_name()).await?;
        let projection = start_projection(&store, projection_database).await?;
        let tool_id = register_sample_tool(&store)?;

        check_out_tool(
            &store,
            CheckOutToolRequest {
                tool_id: tool_id.clone(),
                checked_out_to: "Team Alpha".to_owned(),
                checked_out_at: Some(parse_time("2026-05-08T09:00:00Z")?),
                due_back_at: Some(parse_time("2026-05-09T09:00:00Z")?),
                use_location: Some("job-site-7".to_owned()),
                condition_at_checkout: Some("ready".to_owned()),
            },
        )
        .map_err(|error| format!("check out should succeed during restart setup: {error:?}"))?;

        let items = eventually_inventory(&projection, |items| {
            items
                .first()
                .is_some_and(|item| item.status == InventoryStatus::CheckedOut)
        })?;
        assert_eq!(items[0].tool_id, tool_id);
        drop(projection);
        tool_id
    };

    {
        let restarted_store = test_database.open_store().await?;
        let projection_database =
            ProjectionDatabase::connect(&admin_url, test_database.database_name()).await?;
        let restarted_projection = start_projection(&restarted_store, projection_database).await?;
        let items = eventually_inventory(&restarted_projection, |items| items.len() == 1)?;

        assert_eq!(items[0].tool_id, tool_id);
        assert_eq!(items[0].status, InventoryStatus::CheckedOut);
        assert_eq!(items[0].checked_out_to.as_deref(), Some("Team Alpha"));
    }

    test_database.cleanup().await?;

    Ok(())
}

#[tokio::test]
async fn durable_projection_catches_up_after_restart() -> Result<(), Box<dyn std::error::Error>> {
    let _ = dotenvy::dotenv();

    let admin_url = match env::var("FACTSTR_TOOL_RENTAL_POSTGRES_ADMIN_URL") {
        Ok(value) => value,
        Err(_) => {
            eprintln!(
                "skipping inventory catch-up test: FACTSTR_TOOL_RENTAL_POSTGRES_ADMIN_URL is not set"
            );
            return Ok(());
        }
    };

    let mut test_database = store::TestDatabase::create(&admin_url).await?;
    let tool_id = {
        let store = test_database.open_store().await?;
        let projection_database =
            ProjectionDatabase::connect(&admin_url, test_database.database_name()).await?;
        let projection = start_projection(&store, projection_database).await?;
        let tool_id = register_sample_tool(&store)?;

        let items = eventually_inventory(&projection, |items| items.len() == 1)?;
        assert_eq!(items[0].status, InventoryStatus::Available);
        drop(projection);
        tool_id
    };

    {
        let store = test_database.open_store().await?;
        check_out_tool(
            &store,
            CheckOutToolRequest {
                tool_id: tool_id.clone(),
                checked_out_to: "Team Alpha".to_owned(),
                checked_out_at: Some(parse_time("2026-05-08T09:00:00Z")?),
                due_back_at: Some(parse_time("2026-05-09T09:00:00Z")?),
                use_location: Some("job-site-7".to_owned()),
                condition_at_checkout: Some("ready".to_owned()),
            },
        )
        .map_err(|error| format!("check out should succeed during catch-up setup: {error:?}"))?;
    }

    {
        let restarted_store = test_database.open_store().await?;
        let projection_database =
            ProjectionDatabase::connect(&admin_url, test_database.database_name()).await?;
        let restarted_projection = start_projection(&restarted_store, projection_database).await?;
        let items = eventually_inventory(&restarted_projection, |items| {
            items
                .first()
                .is_some_and(|item| item.status == InventoryStatus::CheckedOut)
        })?;

        assert_eq!(items[0].tool_id, tool_id);
        assert_eq!(items[0].current_location, "job-site-7");
    }

    test_database.cleanup().await?;

    Ok(())
}

#[tokio::test]
async fn projection_persistence_failures_do_not_notify_subscribers()
-> Result<(), Box<dyn std::error::Error>> {
    let _ = dotenvy::dotenv();

    let admin_url = match env::var("FACTSTR_TOOL_RENTAL_POSTGRES_ADMIN_URL") {
        Ok(value) => value,
        Err(_) => {
            eprintln!(
                "skipping inventory notification failure test: FACTSTR_TOOL_RENTAL_POSTGRES_ADMIN_URL is not set"
            );
            return Ok(());
        }
    };

    let mut test_database = store::TestDatabase::create(&admin_url).await?;
    let store = test_database.open_store().await?;
    let projection_database =
        ProjectionDatabase::connect(&admin_url, test_database.database_name()).await?;
    let inventory_change_notifier = InventoryChangeNotifier::new();
    let mut subscriber = inventory_change_notifier.subscribe();
    let _projection = start_projection_with_notifier(
        &store,
        projection_database.clone(),
        inventory_change_notifier,
    )
    .await?;

    let projection_pool = projection_database.connect_pool().await?;
    sqlx::query("DROP TABLE projections.inventory_items")
        .execute(&projection_pool)
        .await?;
    projection_pool.close().await;

    append_registered_tool(
        &store,
        ToolRegisteredPayload {
            tool_id: "tool-1".to_owned(),
            serial_number: "SN-1001".to_owned(),
            name: "Rotary Hammer".to_owned(),
            category: "drilling".to_owned(),
            manufacturer: "Bosch".to_owned(),
            model: "GBH 2-26".to_owned(),
            home_location: "warehouse-a".to_owned(),
            initial_condition: "ready".to_owned(),
        },
    )?;

    thread::sleep(Duration::from_millis(100));
    assert!(matches!(
        subscriber.try_recv(),
        Err(tokio::sync::broadcast::error::TryRecvError::Empty)
    ));

    test_database.cleanup().await?;

    Ok(())
}

#[tokio::test]
async fn projection_store_creates_postgresql_schema_and_table()
-> Result<(), Box<dyn std::error::Error>> {
    let _ = dotenvy::dotenv();

    let admin_url = match env::var("FACTSTR_TOOL_RENTAL_POSTGRES_ADMIN_URL") {
        Ok(value) => value,
        Err(_) => {
            eprintln!(
                "skipping inventory projection schema test: FACTSTR_TOOL_RENTAL_POSTGRES_ADMIN_URL is not set"
            );
            return Ok(());
        }
    };

    let mut test_database = store::TestDatabase::create(&admin_url).await?;
    let _store = test_database.open_store().await?;
    let projection_database =
        ProjectionDatabase::connect(&admin_url, test_database.database_name()).await?;
    projection_database
        .initialize_schema("get_inventory", &schema_statements())
        .await?;

    let database_url = projection_database_url(&admin_url, test_database.database_name())?;
    let mut connection = PgConnection::connect(&database_url).await?;

    let schema_exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(
            SELECT 1
            FROM information_schema.schemata
            WHERE schema_name = 'projections'
        )",
    )
    .fetch_one(&mut connection)
    .await?;
    assert!(schema_exists);

    let table_exists = sqlx::query_scalar::<_, Option<String>>(
        "SELECT to_regclass('projections.inventory_items')::text",
    )
    .fetch_one(&mut connection)
    .await?;
    assert_eq!(table_exists.as_deref(), Some("projections.inventory_items"));

    test_database.cleanup().await?;

    Ok(())
}

#[test]
fn projection_failure_does_not_advance_durable_cursor() -> Result<(), Box<dyn std::error::Error>> {
    let store = MemoryStore::new();
    let durable_stream = DurableStream::new("get_inventory");
    let query = EventQuery::all().with_filters([EventFilter::for_event_types([
        TOOL_REGISTERED_EVENT_TYPE,
        TOOL_CHECKED_OUT_EVENT_TYPE,
        TOOL_RETURNED_EVENT_TYPE,
    ])]);
    let failing_stream = store.stream_to_durable(
        &durable_stream,
        &query,
        factstr::HandleStream::new(|_| async {
            Err(StreamHandlerError::new(
                "simulated inventory projection persistence failure",
            ))
        }),
    )?;
    append_registered_tool(
        &store,
        ToolRegisteredPayload {
            tool_id: "tool-1".to_owned(),
            serial_number: "SN-1001".to_owned(),
            name: "Rotary Hammer".to_owned(),
            category: "drilling".to_owned(),
            manufacturer: "Bosch".to_owned(),
            model: "GBH 2-26".to_owned(),
            home_location: "warehouse-a".to_owned(),
            initial_condition: "ready".to_owned(),
        },
    )?;
    thread::sleep(Duration::from_millis(50));
    drop(failing_stream);

    let projection = start_projection_in_memory(&store)?;
    let items = eventually_inventory(&projection, |items| items.len() == 1)?;

    assert_eq!(items[0].tool_id, "tool-1");

    Ok(())
}

fn eventually_notification(
    subscriber: &mut tokio::sync::broadcast::Receiver<()>,
) -> Result<(), Box<dyn std::error::Error>> {
    for _ in 0..40 {
        match subscriber.try_recv() {
            Ok(()) => return Ok(()),
            Err(tokio::sync::broadcast::error::TryRecvError::Empty) => {
                thread::sleep(Duration::from_millis(10));
            }
            Err(error) => {
                return Err(format!("inventory change notification failed: {error}").into());
            }
        }
    }

    Err("timed out waiting for inventory change notification".into())
}

fn seed_registered_tool(
    store: &impl EventStore,
    tool_id: &str,
    category: &str,
    name: &str,
    serial_number: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    append_registered_tool(
        store,
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

fn append_registered_tool(
    store: &impl EventStore,
    payload: ToolRegisteredPayload,
) -> Result<(), Box<dyn std::error::Error>> {
    append_fact(store, TOOL_REGISTERED_EVENT_TYPE, payload)
}

fn append_checked_out_tool(
    store: &impl EventStore,
    payload: ToolCheckedOutPayload,
) -> Result<(), Box<dyn std::error::Error>> {
    append_fact(store, TOOL_CHECKED_OUT_EVENT_TYPE, payload)
}

fn append_returned_tool(
    store: &impl EventStore,
    payload: ToolReturnedPayload,
) -> Result<(), Box<dyn std::error::Error>> {
    append_fact(store, TOOL_RETURNED_EVENT_TYPE, payload)
}

fn append_fact(
    store: &impl EventStore,
    event_type: &str,
    payload: impl serde::Serialize,
) -> Result<(), Box<dyn std::error::Error>> {
    store.append(vec![NewEvent::new(event_type, to_value(payload)?)])?;
    Ok(())
}

fn parse_time(value: &str) -> Result<OffsetDateTime, time::error::Parse> {
    OffsetDateTime::parse(value, &Rfc3339)
}

fn register_sample_tool(store: &impl EventStore) -> Result<String, Box<dyn std::error::Error>> {
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
    .map_err(|error| format!("register tool should succeed for projection tests: {error:?}"))?;

    Ok(response.tool_id)
}

fn projection_database_url(
    postgres_admin_url: &str,
    database_name: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut options = sqlx::postgres::PgConnectOptions::from_str(postgres_admin_url)?;
    options = options.database(database_name);

    Ok(options.to_url_lossy().to_string())
}

fn eventually_inventory(
    projection: &InventoryProjection,
    predicate: impl Fn(&[InventoryItem]) -> bool,
) -> Result<Vec<InventoryItem>, Box<dyn std::error::Error>> {
    for _ in 0..100 {
        let items = get_inventory(projection)?;
        if predicate(&items) {
            return Ok(items);
        }
        thread::sleep(Duration::from_millis(10));
    }

    Err("inventory projection did not reach the expected state".into())
}
