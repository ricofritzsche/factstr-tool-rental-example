use factstr::{
    AppendResult, DurableStream, EventQuery, EventStore, EventStoreError, EventStream,
    HandleStream, NewEvent, QueryResult,
};
use factstr_memory::MemoryStore;
use factstr_tool_rental_rust::events::{TOOL_REGISTERED_EVENT_TYPE, ToolRegisteredPayload};
use factstr_tool_rental_rust::features::register_tool::{
    RegisterToolErrorCode, RegisterToolRequest, process_request,
};
use serde_json::json;
use uuid::Uuid;

#[test]
fn valid_registration_appends_one_tool_registered_fact() -> Result<(), Box<dyn std::error::Error>> {
    let store = MemoryStore::new();

    let response = process_request(
        &store,
        RegisterToolRequest {
            serial_number: "SN-1001".to_owned(),
            name: "Rotary Hammer".to_owned(),
            category: "drilling".to_owned(),
            manufacturer: Some("Bosch".to_owned()),
            model: Some("GBH 2-26".to_owned()),
            home_location: Some("warehouse-a".to_owned()),
            initial_condition: Some("ready".to_owned()),
        },
    )
    .expect("valid registration should succeed");

    let query_result = store.query(&EventQuery::for_event_types([TOOL_REGISTERED_EVENT_TYPE]))?;

    assert_eq!(query_result.event_records.len(), 1);
    assert_eq!(
        Uuid::parse_str(&response.tool_id)?.to_string(),
        response.tool_id
    );
    assert_eq!(response.serial_number, "SN-1001");

    let appended_payload: ToolRegisteredPayload =
        serde_json::from_value(query_result.event_records[0].payload.clone())?;

    assert_eq!(appended_payload.tool_id, response.tool_id);
    assert_eq!(appended_payload.serial_number, "SN-1001");

    Ok(())
}

#[test]
fn request_deserialization_rejects_tool_id_field() {
    let request = serde_json::from_value::<RegisterToolRequest>(json!({
        "tool_id": "not-allowed",
        "serial_number": "SN-1001",
        "name": "Rotary Hammer",
        "category": "drilling"
    }));

    assert!(request.is_err());
}

#[test]
fn missing_optional_fields_use_domain_defaults() -> Result<(), Box<dyn std::error::Error>> {
    let store = MemoryStore::new();

    process_request(
        &store,
        RegisterToolRequest {
            serial_number: "SN-1001".to_owned(),
            name: "Rotary Hammer".to_owned(),
            category: "drilling".to_owned(),
            manufacturer: None,
            model: None,
            home_location: None,
            initial_condition: None,
        },
    )
    .expect("registration with missing optional fields should succeed");

    let query_result = store.query(&EventQuery::for_event_types([TOOL_REGISTERED_EVENT_TYPE]))?;
    let payload: ToolRegisteredPayload =
        serde_json::from_value(query_result.event_records[0].payload.clone())?;

    assert_eq!(payload.manufacturer, "unknown");
    assert_eq!(payload.model, "unknown");
    assert_eq!(payload.home_location, "unassigned");
    assert_eq!(payload.initial_condition, "usable");

    Ok(())
}

#[test]
fn blank_optional_fields_use_domain_defaults() -> Result<(), Box<dyn std::error::Error>> {
    let store = MemoryStore::new();

    process_request(
        &store,
        RegisterToolRequest {
            serial_number: "SN-1001".to_owned(),
            name: "Rotary Hammer".to_owned(),
            category: "drilling".to_owned(),
            manufacturer: Some("   ".to_owned()),
            model: Some("\t".to_owned()),
            home_location: Some("   ".to_owned()),
            initial_condition: Some("".to_owned()),
        },
    )
    .expect("registration with blank optional fields should succeed");

    let query_result = store.query(&EventQuery::for_event_types([TOOL_REGISTERED_EVENT_TYPE]))?;
    let payload: ToolRegisteredPayload =
        serde_json::from_value(query_result.event_records[0].payload.clone())?;

    assert_eq!(payload.manufacturer, "unknown");
    assert_eq!(payload.model, "unknown");
    assert_eq!(payload.home_location, "unassigned");
    assert_eq!(payload.initial_condition, "usable");

    Ok(())
}

#[test]
fn input_strings_are_trimmed_before_storage() -> Result<(), Box<dyn std::error::Error>> {
    let store = MemoryStore::new();

    let response = process_request(
        &store,
        RegisterToolRequest {
            serial_number: "  SN-1001  ".to_owned(),
            name: "  Rotary Hammer ".to_owned(),
            category: " drilling ".to_owned(),
            manufacturer: Some(" Bosch ".to_owned()),
            model: Some(" GBH 2-26 ".to_owned()),
            home_location: Some(" warehouse-a ".to_owned()),
            initial_condition: Some(" ready ".to_owned()),
        },
    )
    .expect("registration with trimmable input should succeed");

    let query_result = store.query(&EventQuery::for_event_types([TOOL_REGISTERED_EVENT_TYPE]))?;
    let payload: ToolRegisteredPayload =
        serde_json::from_value(query_result.event_records[0].payload.clone())?;

    assert_eq!(response.serial_number, "SN-1001");
    assert_eq!(payload.serial_number, "SN-1001");
    assert_eq!(payload.name, "Rotary Hammer");
    assert_eq!(payload.category, "drilling");
    assert_eq!(payload.manufacturer, "Bosch");
    assert_eq!(payload.model, "GBH 2-26");
    assert_eq!(payload.home_location, "warehouse-a");
    assert_eq!(payload.initial_condition, "ready");

    Ok(())
}

#[test]
fn blank_required_fields_return_stable_error_codes() {
    let store = MemoryStore::new();

    let empty_serial_number = process_request(
        &store,
        RegisterToolRequest {
            serial_number: "   ".to_owned(),
            name: "Rotary Hammer".to_owned(),
            category: "drilling".to_owned(),
            manufacturer: None,
            model: None,
            home_location: None,
            initial_condition: None,
        },
    )
    .unwrap_err();

    let empty_name = process_request(
        &store,
        RegisterToolRequest {
            serial_number: "SN-1001".to_owned(),
            name: "   ".to_owned(),
            category: "drilling".to_owned(),
            manufacturer: None,
            model: None,
            home_location: None,
            initial_condition: None,
        },
    )
    .unwrap_err();

    let empty_category = process_request(
        &store,
        RegisterToolRequest {
            serial_number: "SN-1001".to_owned(),
            name: "Rotary Hammer".to_owned(),
            category: "   ".to_owned(),
            manufacturer: None,
            model: None,
            home_location: None,
            initial_condition: None,
        },
    )
    .unwrap_err();

    assert_eq!(empty_serial_number.code(), "empty_serial_number");
    assert_eq!(empty_name.code(), "empty_name");
    assert_eq!(empty_category.code(), "empty_category");
    assert_eq!(
        RegisterToolErrorCode::SerialNumberAlreadyRegistered.as_str(),
        "serial_number_already_registered"
    );
    assert_eq!(RegisterToolErrorCode::StoreError.as_str(), "store_error");
}

#[test]
fn duplicate_serial_number_returns_duplicate_error_and_appends_no_second_fact()
-> Result<(), Box<dyn std::error::Error>> {
    let store = MemoryStore::new();

    process_request(
        &store,
        RegisterToolRequest {
            serial_number: "SN-1001".to_owned(),
            name: "Rotary Hammer".to_owned(),
            category: "drilling".to_owned(),
            manufacturer: None,
            model: None,
            home_location: None,
            initial_condition: None,
        },
    )
    .expect("first registration should succeed");

    let duplicate_error = process_request(
        &store,
        RegisterToolRequest {
            serial_number: "SN-1001".to_owned(),
            name: "Second Hammer".to_owned(),
            category: "drilling".to_owned(),
            manufacturer: None,
            model: None,
            home_location: None,
            initial_condition: None,
        },
    )
    .unwrap_err();

    let query_result = store.query(&EventQuery::for_event_types([TOOL_REGISTERED_EVENT_TYPE]))?;

    assert_eq!(duplicate_error.code(), "serial_number_already_registered");
    assert_eq!(query_result.event_records.len(), 1);

    Ok(())
}

#[test]
fn conditional_append_conflict_maps_to_duplicate_error() {
    let store = ConflictOnAppendIfStore;

    let error = process_request(
        &store,
        RegisterToolRequest {
            serial_number: "SN-1001".to_owned(),
            name: "Rotary Hammer".to_owned(),
            category: "drilling".to_owned(),
            manufacturer: None,
            model: None,
            home_location: None,
            initial_condition: None,
        },
    )
    .unwrap_err();

    assert_eq!(error.code(), "serial_number_already_registered");
}

struct ConflictOnAppendIfStore;

impl EventStore for ConflictOnAppendIfStore {
    fn query(&self, _event_query: &EventQuery) -> Result<QueryResult, EventStoreError> {
        Ok(QueryResult {
            event_records: Vec::new(),
            last_returned_sequence_number: None,
            current_context_version: Some(0),
        })
    }

    fn append(&self, _new_events: Vec<NewEvent>) -> Result<AppendResult, EventStoreError> {
        Err(EventStoreError::NotImplemented {
            store: "ConflictOnAppendIfStore",
            operation: "append",
        })
    }

    fn append_if(
        &self,
        _new_events: Vec<NewEvent>,
        _context_query: &EventQuery,
        _expected_context_version: Option<u64>,
    ) -> Result<AppendResult, EventStoreError> {
        Err(EventStoreError::ConditionalAppendConflict {
            expected: Some(0),
            actual: Some(1),
        })
    }

    fn stream_all(&self, _handle: HandleStream) -> Result<EventStream, EventStoreError> {
        Err(EventStoreError::NotImplemented {
            store: "ConflictOnAppendIfStore",
            operation: "stream_all",
        })
    }

    fn stream_to(
        &self,
        _event_query: &EventQuery,
        _handle: HandleStream,
    ) -> Result<EventStream, EventStoreError> {
        Err(EventStoreError::NotImplemented {
            store: "ConflictOnAppendIfStore",
            operation: "stream_to",
        })
    }

    fn stream_all_durable(
        &self,
        _durable_stream: &DurableStream,
        _handle: HandleStream,
    ) -> Result<EventStream, EventStoreError> {
        Err(EventStoreError::NotImplemented {
            store: "ConflictOnAppendIfStore",
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
            store: "ConflictOnAppendIfStore",
            operation: "stream_to_durable",
        })
    }
}
