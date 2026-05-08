pub const CREATE_PROJECTION_SCHEMA: &str = "CREATE SCHEMA IF NOT EXISTS projections";

pub const CREATE_INVENTORY_ITEMS_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS projections.inventory_items (
    tool_id TEXT PRIMARY KEY,
    serial_number TEXT NOT NULL,
    name TEXT NOT NULL,
    category TEXT NOT NULL,
    manufacturer TEXT NOT NULL,
    model TEXT NOT NULL,
    home_location TEXT NOT NULL,
    current_location TEXT NOT NULL,
    status TEXT NOT NULL,
    checked_out_to TEXT NULL,
    due_back_at TIMESTAMPTZ NULL
)
"#;

pub const CREATE_INVENTORY_ITEMS_ORDERING_INDEX: &str = r#"
CREATE INDEX IF NOT EXISTS inventory_items_category_name_serial_number_idx
ON projections.inventory_items (category, name, serial_number)
"#;

pub fn schema_statements() -> [&'static str; 3] {
    [
        CREATE_PROJECTION_SCHEMA,
        CREATE_INVENTORY_ITEMS_TABLE,
        CREATE_INVENTORY_ITEMS_ORDERING_INDEX,
    ]
}
