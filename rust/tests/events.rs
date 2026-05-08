use factstr_tool_rental_rust::events::{
    TOOL_CHECKED_OUT_EVENT_TYPE, TOOL_REGISTERED_EVENT_TYPE, TOOL_RETURNED_EVENT_TYPE,
    ToolCheckedOutPayload, ToolRegisteredPayload, ToolReturnedPayload,
};
use serde_json::{Value, json};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

fn parse_timestamp(value: &str) -> Result<OffsetDateTime, time::error::Parse> {
    OffsetDateTime::parse(value, &Rfc3339)
}

#[test]
fn event_type_constants_match_expected_values() {
    assert_eq!(TOOL_REGISTERED_EVENT_TYPE, "tool-registered");
    assert_eq!(TOOL_CHECKED_OUT_EVENT_TYPE, "tool-checked-out");
    assert_eq!(TOOL_RETURNED_EVENT_TYPE, "tool-returned");
}

#[test]
fn tool_registered_payload_serializes_with_expected_json_shape()
-> Result<(), Box<dyn std::error::Error>> {
    let payload = ToolRegisteredPayload {
        tool_id: "tool-001".to_owned(),
        serial_number: "SN-1001".to_owned(),
        name: "Rotary Hammer".to_owned(),
        category: "drilling".to_owned(),
        manufacturer: "Bosch".to_owned(),
        model: "GBH 2-26".to_owned(),
        home_location: "warehouse-a".to_owned(),
        initial_condition: "ready".to_owned(),
    };

    let serialized = serde_json::to_value(&payload)?;

    assert_eq!(
        serialized,
        json!({
            "tool_id": "tool-001",
            "serial_number": "SN-1001",
            "name": "Rotary Hammer",
            "category": "drilling",
            "manufacturer": "Bosch",
            "model": "GBH 2-26",
            "home_location": "warehouse-a",
            "initial_condition": "ready"
        })
    );

    let round_trip: ToolRegisteredPayload = serde_json::from_value(serialized)?;
    assert_eq!(round_trip, payload);

    Ok(())
}

#[test]
fn tool_checked_out_payload_serializes_rfc3339_timestamps_and_round_trips()
-> Result<(), Box<dyn std::error::Error>> {
    let checked_out_at = parse_timestamp("2026-05-08T09:30:00Z")?;
    let due_back_at = parse_timestamp("2026-05-09T17:00:00Z")?;
    let payload = ToolCheckedOutPayload {
        tool_id: "tool-001".to_owned(),
        checked_out_to: "job-447".to_owned(),
        checked_out_at,
        due_back_at,
        use_location: "site-madrid".to_owned(),
        condition_at_checkout: "good".to_owned(),
    };

    let serialized = serde_json::to_value(&payload)?;

    assert_eq!(
        serialized,
        json!({
            "tool_id": "tool-001",
            "checked_out_to": "job-447",
            "checked_out_at": "2026-05-08T09:30:00Z",
            "due_back_at": "2026-05-09T17:00:00Z",
            "use_location": "site-madrid",
            "condition_at_checkout": "good"
        })
    );

    let checked_out_at_json = serialized
        .get("checked_out_at")
        .and_then(Value::as_str)
        .ok_or("missing checked_out_at string")?;
    let due_back_at_json = serialized
        .get("due_back_at")
        .and_then(Value::as_str)
        .ok_or("missing due_back_at string")?;

    assert_eq!(parse_timestamp(checked_out_at_json)?, checked_out_at);
    assert_eq!(parse_timestamp(due_back_at_json)?, due_back_at);

    let round_trip: ToolCheckedOutPayload = serde_json::from_value(serialized)?;
    assert_eq!(round_trip, payload);

    Ok(())
}

#[test]
fn tool_returned_payload_serializes_rfc3339_timestamp_and_round_trips()
-> Result<(), Box<dyn std::error::Error>> {
    let returned_at = parse_timestamp("2026-05-10T12:15:00Z")?;
    let payload = ToolReturnedPayload {
        tool_id: "tool-001".to_owned(),
        returned_at,
        returned_to_location: "warehouse-a".to_owned(),
        condition_at_return: "good".to_owned(),
    };

    let serialized = serde_json::to_value(&payload)?;

    assert_eq!(
        serialized,
        json!({
            "tool_id": "tool-001",
            "returned_at": "2026-05-10T12:15:00Z",
            "returned_to_location": "warehouse-a",
            "condition_at_return": "good"
        })
    );

    let returned_at_json = serialized
        .get("returned_at")
        .and_then(Value::as_str)
        .ok_or("missing returned_at string")?;

    assert_eq!(parse_timestamp(returned_at_json)?, returned_at);

    let round_trip: ToolReturnedPayload = serde_json::from_value(serialized)?;
    assert_eq!(round_trip, payload);

    Ok(())
}
