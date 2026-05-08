use factstr::{
    AppendResult, DurableStream, EventQuery, EventRecord, EventStore, EventStoreError, EventStream,
    HandleStream, NewEvent, QueryResult,
};
use factstr_memory::MemoryStore;
use factstr_tool_rental_rust::events::{
    TOOL_CHECKED_OUT_EVENT_TYPE, TOOL_REGISTERED_EVENT_TYPE, ToolCheckedOutPayload,
};
use factstr_tool_rental_rust::features::check_out_tool::{
    CheckOutToolErrorCode, CheckOutToolRequest, process_request as check_out_tool,
};
use factstr_tool_rental_rust::features::register_tool::{
    RegisterToolRequest, process_request as register_tool,
};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

#[test]
fn registered_available_tool_can_be_checked_out() -> Result<(), Box<dyn std::error::Error>> {
    let store = MemoryStore::new();
    let tool_id = register_sample_tool(&store)?;
    let checked_out_at = parse_time("2026-05-08T09:00:00Z")?;
    let due_back_at = parse_time("2026-05-09T09:00:00Z")?;

    let response = check_out_tool(
        &store,
        CheckOutToolRequest {
            tool_id: tool_id.clone(),
            checked_out_to: "Team Alpha".to_owned(),
            checked_out_at: Some(checked_out_at),
            due_back_at: Some(due_back_at),
            use_location: Some("job-site-7".to_owned()),
            condition_at_checkout: Some("ready".to_owned()),
        },
    )
    .expect("checkout should succeed");

    let query_result = store.query(&EventQuery::for_event_types([TOOL_CHECKED_OUT_EVENT_TYPE]))?;

    assert_eq!(query_result.event_records.len(), 1);
    assert_eq!(response.tool_id, tool_id);
    assert_eq!(response.checked_out_to, "Team Alpha");
    assert_eq!(response.checked_out_at, checked_out_at);
    assert_eq!(response.due_back_at, due_back_at);

    let payload: ToolCheckedOutPayload =
        serde_json::from_value(query_result.event_records[0].payload.clone())?;

    assert_eq!(payload.tool_id, response.tool_id);
    assert_eq!(payload.checked_out_to, "Team Alpha");
    assert_eq!(payload.checked_out_at, checked_out_at);
    assert_eq!(payload.due_back_at, due_back_at);
    assert_eq!(payload.use_location, "job-site-7");
    assert_eq!(payload.condition_at_checkout, "ready");

    Ok(())
}

#[test]
fn missing_optional_fields_use_defaults() -> Result<(), Box<dyn std::error::Error>> {
    let store = MemoryStore::new();
    let tool_id = register_sample_tool(&store)?;

    check_out_tool(
        &store,
        CheckOutToolRequest {
            tool_id,
            checked_out_to: "Team Alpha".to_owned(),
            checked_out_at: Some(parse_time("2026-05-08T09:00:00Z")?),
            due_back_at: Some(parse_time("2026-05-09T09:00:00Z")?),
            use_location: None,
            condition_at_checkout: None,
        },
    )
    .expect("checkout should succeed");

    let query_result = store.query(&EventQuery::for_event_types([TOOL_CHECKED_OUT_EVENT_TYPE]))?;
    let payload: ToolCheckedOutPayload =
        serde_json::from_value(query_result.event_records[0].payload.clone())?;

    assert_eq!(payload.use_location, "unknown");
    assert_eq!(payload.condition_at_checkout, "usable");

    Ok(())
}

#[test]
fn blank_optional_fields_use_defaults() -> Result<(), Box<dyn std::error::Error>> {
    let store = MemoryStore::new();
    let tool_id = register_sample_tool(&store)?;

    check_out_tool(
        &store,
        CheckOutToolRequest {
            tool_id,
            checked_out_to: "Team Alpha".to_owned(),
            checked_out_at: Some(parse_time("2026-05-08T09:00:00Z")?),
            due_back_at: Some(parse_time("2026-05-09T09:00:00Z")?),
            use_location: Some("   ".to_owned()),
            condition_at_checkout: Some("\t".to_owned()),
        },
    )
    .expect("checkout should succeed");

    let query_result = store.query(&EventQuery::for_event_types([TOOL_CHECKED_OUT_EVENT_TYPE]))?;
    let payload: ToolCheckedOutPayload =
        serde_json::from_value(query_result.event_records[0].payload.clone())?;

    assert_eq!(payload.use_location, "unknown");
    assert_eq!(payload.condition_at_checkout, "usable");

    Ok(())
}

#[test]
fn input_strings_are_trimmed_before_storage() -> Result<(), Box<dyn std::error::Error>> {
    let store = MemoryStore::new();
    let tool_id = register_sample_tool(&store)?;

    let response = check_out_tool(
        &store,
        CheckOutToolRequest {
            tool_id: format!("  {tool_id}  "),
            checked_out_to: " Team Alpha ".to_owned(),
            checked_out_at: Some(parse_time("2026-05-08T09:00:00Z")?),
            due_back_at: Some(parse_time("2026-05-09T09:00:00Z")?),
            use_location: Some(" job-site-7 ".to_owned()),
            condition_at_checkout: Some(" ready ".to_owned()),
        },
    )
    .expect("checkout should succeed");

    let query_result = store.query(&EventQuery::for_event_types([TOOL_CHECKED_OUT_EVENT_TYPE]))?;
    let payload: ToolCheckedOutPayload =
        serde_json::from_value(query_result.event_records[0].payload.clone())?;

    assert_eq!(response.tool_id, tool_id);
    assert_eq!(payload.tool_id, response.tool_id);
    assert_eq!(payload.checked_out_to, "Team Alpha");
    assert_eq!(payload.use_location, "job-site-7");
    assert_eq!(payload.condition_at_checkout, "ready");

    Ok(())
}

#[test]
fn blank_and_missing_required_fields_return_stable_error_codes()
-> Result<(), Box<dyn std::error::Error>> {
    let store = MemoryStore::new();
    let tool_id = register_sample_tool(&store)?;
    let checked_out_at = Some(parse_time("2026-05-08T09:00:00Z")?);
    let due_back_at = Some(parse_time("2026-05-09T09:00:00Z")?);

    let empty_tool_id = check_out_tool(
        &store,
        CheckOutToolRequest {
            tool_id: "   ".to_owned(),
            checked_out_to: "Team Alpha".to_owned(),
            checked_out_at,
            due_back_at,
            use_location: None,
            condition_at_checkout: None,
        },
    )
    .unwrap_err();

    let empty_checked_out_to = check_out_tool(
        &store,
        CheckOutToolRequest {
            tool_id: tool_id.clone(),
            checked_out_to: "   ".to_owned(),
            checked_out_at,
            due_back_at,
            use_location: None,
            condition_at_checkout: None,
        },
    )
    .unwrap_err();

    let missing_checked_out_at = check_out_tool(
        &store,
        CheckOutToolRequest {
            tool_id: tool_id.clone(),
            checked_out_to: "Team Alpha".to_owned(),
            checked_out_at: None,
            due_back_at,
            use_location: None,
            condition_at_checkout: None,
        },
    )
    .unwrap_err();

    let missing_due_back_at = check_out_tool(
        &store,
        CheckOutToolRequest {
            tool_id,
            checked_out_to: "Team Alpha".to_owned(),
            checked_out_at,
            due_back_at: None,
            use_location: None,
            condition_at_checkout: None,
        },
    )
    .unwrap_err();

    assert_eq!(empty_tool_id.code(), "empty_tool_id");
    assert_eq!(empty_checked_out_to.code(), "empty_checked_out_to");
    assert_eq!(missing_checked_out_at.code(), "missing_checked_out_at");
    assert_eq!(missing_due_back_at.code(), "missing_due_back_at");
    assert_eq!(
        CheckOutToolErrorCode::DueBackMustBeLaterThanCheckedOut.as_str(),
        "due_back_must_be_later_than_checked_out"
    );
    assert_eq!(
        CheckOutToolErrorCode::ToolNotRegistered.as_str(),
        "tool_not_registered"
    );
    assert_eq!(
        CheckOutToolErrorCode::ToolAlreadyCheckedOut.as_str(),
        "tool_already_checked_out"
    );
    assert_eq!(CheckOutToolErrorCode::StoreError.as_str(), "store_error");

    Ok(())
}

#[test]
fn due_back_must_be_later_than_checked_out() -> Result<(), Box<dyn std::error::Error>> {
    let store = MemoryStore::new();
    let tool_id = register_sample_tool(&store)?;
    let timestamp = parse_time("2026-05-08T09:00:00Z")?;

    let equal_error = check_out_tool(
        &store,
        CheckOutToolRequest {
            tool_id: tool_id.clone(),
            checked_out_to: "Team Alpha".to_owned(),
            checked_out_at: Some(timestamp),
            due_back_at: Some(timestamp),
            use_location: None,
            condition_at_checkout: None,
        },
    )
    .unwrap_err();

    let earlier_error = check_out_tool(
        &store,
        CheckOutToolRequest {
            tool_id,
            checked_out_to: "Team Alpha".to_owned(),
            checked_out_at: Some(parse_time("2026-05-08T09:00:00Z")?),
            due_back_at: Some(parse_time("2026-05-08T08:59:59Z")?),
            use_location: None,
            condition_at_checkout: None,
        },
    )
    .unwrap_err();

    assert_eq!(
        equal_error.code(),
        "due_back_must_be_later_than_checked_out"
    );
    assert_eq!(
        earlier_error.code(),
        "due_back_must_be_later_than_checked_out"
    );

    Ok(())
}

#[test]
fn unknown_tool_returns_tool_not_registered() -> Result<(), Box<dyn std::error::Error>> {
    let store = MemoryStore::new();

    let error = check_out_tool(
        &store,
        CheckOutToolRequest {
            tool_id: "tool-unknown".to_owned(),
            checked_out_to: "Team Alpha".to_owned(),
            checked_out_at: Some(parse_time("2026-05-08T09:00:00Z")?),
            due_back_at: Some(parse_time("2026-05-09T09:00:00Z")?),
            use_location: None,
            condition_at_checkout: None,
        },
    )
    .unwrap_err();

    assert_eq!(error.code(), "tool_not_registered");

    Ok(())
}

#[test]
fn already_checked_out_tool_returns_conflict_and_appends_no_second_checkout_fact()
-> Result<(), Box<dyn std::error::Error>> {
    let store = MemoryStore::new();
    let tool_id = register_sample_tool(&store)?;

    check_out_tool(
        &store,
        CheckOutToolRequest {
            tool_id: tool_id.clone(),
            checked_out_to: "Team Alpha".to_owned(),
            checked_out_at: Some(parse_time("2026-05-08T09:00:00Z")?),
            due_back_at: Some(parse_time("2026-05-09T09:00:00Z")?),
            use_location: None,
            condition_at_checkout: None,
        },
    )
    .expect("first checkout should succeed");

    let error = check_out_tool(
        &store,
        CheckOutToolRequest {
            tool_id,
            checked_out_to: "Team Beta".to_owned(),
            checked_out_at: Some(parse_time("2026-05-08T10:00:00Z")?),
            due_back_at: Some(parse_time("2026-05-09T10:00:00Z")?),
            use_location: None,
            condition_at_checkout: None,
        },
    )
    .unwrap_err();

    let query_result = store.query(&EventQuery::for_event_types([TOOL_CHECKED_OUT_EVENT_TYPE]))?;

    assert_eq!(error.code(), "tool_already_checked_out");
    assert_eq!(query_result.event_records.len(), 1);

    Ok(())
}

#[test]
fn conditional_append_conflict_maps_to_tool_already_checked_out()
-> Result<(), Box<dyn std::error::Error>> {
    let tool_id = register_sample_tool(&MemoryStore::new())?;
    let store = ConflictOnAppendIfStore { tool_id };

    let error = check_out_tool(
        &store,
        CheckOutToolRequest {
            tool_id: store.tool_id.clone(),
            checked_out_to: "Team Alpha".to_owned(),
            checked_out_at: Some(parse_time("2026-05-08T09:00:00Z")?),
            due_back_at: Some(parse_time("2026-05-09T09:00:00Z")?),
            use_location: None,
            condition_at_checkout: None,
        },
    )
    .unwrap_err();

    assert_eq!(error.code(), "tool_already_checked_out");

    Ok(())
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
        let matching_records = vec![EventRecord {
            sequence_number: 1,
            occurred_at: parse_time("2026-05-08T08:00:00Z").expect("static timestamp should parse"),
            event_type: TOOL_REGISTERED_EVENT_TYPE.to_owned(),
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
        }];

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
            expected: Some(1),
            actual: Some(2),
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
