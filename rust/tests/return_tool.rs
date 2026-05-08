use factstr::{
    AppendResult, DurableStream, EventQuery, EventRecord, EventStore, EventStoreError, EventStream,
    HandleStream, NewEvent, QueryResult,
};
use factstr_memory::MemoryStore;
use factstr_tool_rental_rust::events::{
    TOOL_CHECKED_OUT_EVENT_TYPE, TOOL_RETURNED_EVENT_TYPE, ToolReturnedPayload,
};
use factstr_tool_rental_rust::features::check_out_tool::{
    CheckOutToolRequest, process_request as check_out_tool,
};
use factstr_tool_rental_rust::features::register_tool::{
    RegisterToolRequest, process_request as register_tool,
};
use factstr_tool_rental_rust::features::return_tool::{
    ReturnToolErrorCode, ReturnToolRequest, process_request as return_tool,
};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

#[test]
fn checked_out_tool_can_be_returned() -> Result<(), Box<dyn std::error::Error>> {
    let store = MemoryStore::new();
    let tool_id = register_and_check_out_tool(&store)?;
    let returned_at = parse_time("2026-05-10T09:00:00Z")?;

    let response = return_tool(
        &store,
        ReturnToolRequest {
            tool_id: tool_id.clone(),
            returned_at: Some(returned_at),
            returned_to_location: Some("warehouse-a".to_owned()),
            condition_at_return: Some("ready".to_owned()),
        },
    )
    .expect("return should succeed");

    let query_result = store.query(&EventQuery::for_event_types([TOOL_RETURNED_EVENT_TYPE]))?;

    assert_eq!(query_result.event_records.len(), 1);
    assert_eq!(response.tool_id, tool_id);
    assert_eq!(response.returned_at, returned_at);
    assert_eq!(response.returned_to_location, "warehouse-a");

    let payload: ToolReturnedPayload =
        serde_json::from_value(query_result.event_records[0].payload.clone())?;

    assert_eq!(payload.tool_id, response.tool_id);
    assert_eq!(payload.returned_at, returned_at);
    assert_eq!(payload.returned_to_location, "warehouse-a");
    assert_eq!(payload.condition_at_return, "ready");

    Ok(())
}

#[test]
fn missing_optional_fields_use_defaults() -> Result<(), Box<dyn std::error::Error>> {
    let store = MemoryStore::new();
    let tool_id = register_and_check_out_tool(&store)?;

    return_tool(
        &store,
        ReturnToolRequest {
            tool_id,
            returned_at: Some(parse_time("2026-05-10T09:00:00Z")?),
            returned_to_location: None,
            condition_at_return: None,
        },
    )
    .expect("return should succeed");

    let query_result = store.query(&EventQuery::for_event_types([TOOL_RETURNED_EVENT_TYPE]))?;
    let payload: ToolReturnedPayload =
        serde_json::from_value(query_result.event_records[0].payload.clone())?;

    assert_eq!(payload.returned_to_location, "unassigned");
    assert_eq!(payload.condition_at_return, "usable");

    Ok(())
}

#[test]
fn blank_optional_fields_use_defaults() -> Result<(), Box<dyn std::error::Error>> {
    let store = MemoryStore::new();
    let tool_id = register_and_check_out_tool(&store)?;

    return_tool(
        &store,
        ReturnToolRequest {
            tool_id,
            returned_at: Some(parse_time("2026-05-10T09:00:00Z")?),
            returned_to_location: Some("   ".to_owned()),
            condition_at_return: Some("\t".to_owned()),
        },
    )
    .expect("return should succeed");

    let query_result = store.query(&EventQuery::for_event_types([TOOL_RETURNED_EVENT_TYPE]))?;
    let payload: ToolReturnedPayload =
        serde_json::from_value(query_result.event_records[0].payload.clone())?;

    assert_eq!(payload.returned_to_location, "unassigned");
    assert_eq!(payload.condition_at_return, "usable");

    Ok(())
}

#[test]
fn input_strings_are_trimmed_before_storage() -> Result<(), Box<dyn std::error::Error>> {
    let store = MemoryStore::new();
    let tool_id = register_and_check_out_tool(&store)?;

    let response = return_tool(
        &store,
        ReturnToolRequest {
            tool_id: format!("  {tool_id}  "),
            returned_at: Some(parse_time("2026-05-10T09:00:00Z")?),
            returned_to_location: Some(" warehouse-a ".to_owned()),
            condition_at_return: Some(" ready ".to_owned()),
        },
    )
    .expect("return should succeed");

    let query_result = store.query(&EventQuery::for_event_types([TOOL_RETURNED_EVENT_TYPE]))?;
    let payload: ToolReturnedPayload =
        serde_json::from_value(query_result.event_records[0].payload.clone())?;

    assert_eq!(payload.tool_id, response.tool_id);
    assert_eq!(payload.returned_to_location, "warehouse-a");
    assert_eq!(payload.condition_at_return, "ready");

    Ok(())
}

#[test]
fn blank_tool_id_and_missing_returned_at_return_stable_error_codes()
-> Result<(), Box<dyn std::error::Error>> {
    let store = MemoryStore::new();
    let tool_id = register_and_check_out_tool(&store)?;

    let empty_tool_id = return_tool(
        &store,
        ReturnToolRequest {
            tool_id: "   ".to_owned(),
            returned_at: Some(parse_time("2026-05-10T09:00:00Z")?),
            returned_to_location: None,
            condition_at_return: None,
        },
    )
    .unwrap_err();

    let missing_returned_at = return_tool(
        &store,
        ReturnToolRequest {
            tool_id,
            returned_at: None,
            returned_to_location: None,
            condition_at_return: None,
        },
    )
    .unwrap_err();

    assert_eq!(empty_tool_id.code(), "empty_tool_id");
    assert_eq!(missing_returned_at.code(), "missing_returned_at");
    assert_eq!(
        ReturnToolErrorCode::ToolNotRegistered.as_str(),
        "tool_not_registered"
    );
    assert_eq!(
        ReturnToolErrorCode::ToolNotCheckedOut.as_str(),
        "tool_not_checked_out"
    );
    assert_eq!(ReturnToolErrorCode::StoreError.as_str(), "store_error");

    Ok(())
}

#[test]
fn unknown_tool_returns_tool_not_registered() -> Result<(), Box<dyn std::error::Error>> {
    let store = MemoryStore::new();

    let error = return_tool(
        &store,
        ReturnToolRequest {
            tool_id: "tool-unknown".to_owned(),
            returned_at: Some(parse_time("2026-05-10T09:00:00Z")?),
            returned_to_location: None,
            condition_at_return: None,
        },
    )
    .unwrap_err();

    let query_result = store.query(&EventQuery::for_event_types([TOOL_RETURNED_EVENT_TYPE]))?;

    assert_eq!(error.code(), "tool_not_registered");
    assert_eq!(query_result.event_records.len(), 0);

    Ok(())
}

#[test]
fn available_but_not_checked_out_tool_returns_tool_not_checked_out()
-> Result<(), Box<dyn std::error::Error>> {
    let store = MemoryStore::new();
    let tool_id = register_sample_tool(&store)?;

    let error = return_tool(
        &store,
        ReturnToolRequest {
            tool_id,
            returned_at: Some(parse_time("2026-05-10T09:00:00Z")?),
            returned_to_location: None,
            condition_at_return: None,
        },
    )
    .unwrap_err();

    let query_result = store.query(&EventQuery::for_event_types([TOOL_RETURNED_EVENT_TYPE]))?;

    assert_eq!(error.code(), "tool_not_checked_out");
    assert_eq!(query_result.event_records.len(), 0);

    Ok(())
}

#[test]
fn already_returned_tool_returns_tool_not_checked_out() -> Result<(), Box<dyn std::error::Error>> {
    let store = MemoryStore::new();
    let tool_id = register_and_check_out_tool(&store)?;

    return_tool(
        &store,
        ReturnToolRequest {
            tool_id: tool_id.clone(),
            returned_at: Some(parse_time("2026-05-10T09:00:00Z")?),
            returned_to_location: None,
            condition_at_return: None,
        },
    )
    .expect("first return should succeed");

    let error = return_tool(
        &store,
        ReturnToolRequest {
            tool_id,
            returned_at: Some(parse_time("2026-05-10T10:00:00Z")?),
            returned_to_location: None,
            condition_at_return: None,
        },
    )
    .unwrap_err();

    let query_result = store.query(&EventQuery::for_event_types([TOOL_RETURNED_EVENT_TYPE]))?;

    assert_eq!(error.code(), "tool_not_checked_out");
    assert_eq!(query_result.event_records.len(), 1);

    Ok(())
}

#[test]
fn conditional_append_conflict_maps_to_tool_not_checked_out()
-> Result<(), Box<dyn std::error::Error>> {
    let store = ConflictOnAppendIfStore {
        tool_id: "tool-123".to_owned(),
    };

    let error = return_tool(
        &store,
        ReturnToolRequest {
            tool_id: store.tool_id.clone(),
            returned_at: Some(parse_time("2026-05-10T09:00:00Z")?),
            returned_to_location: None,
            condition_at_return: None,
        },
    )
    .unwrap_err();

    assert_eq!(error.code(), "tool_not_checked_out");

    Ok(())
}

fn register_and_check_out_tool(
    store: &impl EventStore,
) -> Result<String, Box<dyn std::error::Error>> {
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

fn register_sample_tool(store: &impl EventStore) -> Result<String, Box<dyn std::error::Error>> {
    let response = register_tool(
        store,
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
    .expect("tool registration should succeed");

    Ok(response.tool_id)
}

fn parse_time(value: &str) -> Result<OffsetDateTime, time::error::Parse> {
    OffsetDateTime::parse(value, &Rfc3339)
}

struct ConflictOnAppendIfStore {
    tool_id: String,
}

impl EventStore for ConflictOnAppendIfStore {
    fn query(&self, _event_query: &EventQuery) -> Result<QueryResult, EventStoreError> {
        let matching_records = vec![
            EventRecord {
                sequence_number: 1,
                occurred_at: parse_time("2026-05-08T08:00:00Z")
                    .expect("static timestamp should parse"),
                event_type: factstr_tool_rental_rust::events::TOOL_REGISTERED_EVENT_TYPE.to_owned(),
                payload: serde_json::json!({
                    "tool_id": self.tool_id,
                    "serial_number": "SN-1001",
                    "name": "Rotary Hammer",
                    "category": "drilling",
                    "manufacturer": "unknown",
                    "model": "unknown",
                    "home_location": "unassigned",
                    "initial_condition": "usable"
                }),
            },
            EventRecord {
                sequence_number: 2,
                occurred_at: parse_time("2026-05-08T09:00:00Z")
                    .expect("static timestamp should parse"),
                event_type: TOOL_CHECKED_OUT_EVENT_TYPE.to_owned(),
                payload: serde_json::json!({
                    "tool_id": self.tool_id,
                    "checked_out_to": "Team Alpha",
                    "checked_out_at": "2026-05-08T09:00:00Z",
                    "due_back_at": "2026-05-09T09:00:00Z",
                    "use_location": "unknown",
                    "condition_at_checkout": "usable"
                }),
            },
        ];

        Ok(QueryResult {
            current_context_version: matching_records
                .last()
                .map(|event_record| event_record.sequence_number),
            last_returned_sequence_number: matching_records
                .last()
                .map(|event_record| event_record.sequence_number),
            event_records: matching_records,
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
            expected: Some(2),
            actual: Some(3),
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
